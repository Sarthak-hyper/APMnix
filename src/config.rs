use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::Write;

// ── Path helpers ──────────────────────────────────────────────
fn home_nix_path() -> PathBuf {
    let user = std::env::var("USER").unwrap_or("user".to_string());
    PathBuf::from(format!("/home/{}/.dotfiles/home.nix", user))
}

fn system_nix_path() -> PathBuf {
    let user = std::env::var("USER").unwrap_or("user".to_string());
    PathBuf::from(format!("/home/{}/.dotfiles/configuration.nix", user))
}
// ── Generic read/write ────────────────────────────────────────

fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

fn write_file(path: &str, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

fn read_system_config() -> Result<String, String> {
    let path = system_nix_path();
    read_file(path.to_str().ok_or("Invalid system config path")?)
}

fn write_system_config(content: &str) -> Result<(), String> {
    let path = system_nix_path();
    write_file(path.to_str().ok_or("Invalid system config path")?, content)
}

// ── User (home-manager) install ───────────────────────────────
pub fn add_package_user(attribute: &str) -> Result<(), String> {
    let path = home_nix_path();
    let path_str = path.to_str().ok_or("Invalid home.nix path")?;

    let content = read_file(path_str)
        .map_err(|e| format!("Could not read {}: {}", path_str, e))?;

    if content.contains(&format!("pkgs.{}", attribute)) {
        return Ok(()); // already present
    }

    let new_content = insert_package_into_nix(&content, attribute)?;
    write_file(path_str, &new_content)
        .map_err(|e| format!("Could not write {}: {}", path_str, e))?;

    // Fix: Force home-manager to use our custom path via the HOME_MANAGER_CONFIG environment variable
    let cmd_str = format!("HOME_MANAGER_CONFIG=\"{}\" home-manager switch", path_str);
    let output = Command::new("bash")
        .args(["-l", "-c", &cmd_str])
        .output()
        .map_err(|e| format!("Failed to execute home-manager: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        // Roll back the file change
        write_file(path_str, &content).ok();
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn remove_package_user(attribute: &str) -> Result<(), String> {
    let path = home_nix_path();
    let path_str = path.to_str().ok_or("Invalid home.nix path")?;

    let content = read_file(path_str).map_err(|e| e.to_string())?;
    let new_content = remove_package_from_nix(&content, attribute);
    write_file(path_str, &new_content).map_err(|e| e.to_string())?;

    // Fix: Force home-manager to use our custom path via the HOME_MANAGER_CONFIG environment variable
    let cmd_str = format!("HOME_MANAGER_CONFIG=\"{}\" home-manager switch", path_str);
    let output = Command::new("bash")
        .args(["-l", "-c", &cmd_str])
        .output()
        .map_err(|e| format!("Failed to execute home-manager: {}", e))?;

    if !output.status.success() {
        write_file(path_str, &content).ok();
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn is_installed_user(attribute: &str) -> bool {
    let path = home_nix_path();
    read_file(path.to_str().unwrap_or(""))
        .map(|c| c.contains(&format!("pkgs.{}", attribute)))
        .unwrap_or(false)
}

// ── System (nixos) install ────────────────────────────────────

pub fn add_package_system(attribute: &str, sudo_password: &str) -> Result<(), String> {
    verify_sudo(sudo_password)?;
    backup_system_config()?;

    let content = read_system_config()?;

    if content.contains(&format!("pkgs.{}", attribute)) {
        return Ok(());
    }

    let new_content = insert_package_into_nix(&content, attribute)?;
    write_system_config(&new_content)?;

    // nixos-rebuild switch
    let mut child = Command::new("sudo")
        .args(["-S", "nixos-rebuild", "switch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(format!("{}\n", sudo_password).as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        restore_system_backup().ok();
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn remove_package_system(attribute: &str, sudo_password: &str) -> Result<(), String> {
    verify_sudo(sudo_password)?;
    backup_system_config()?;
    
    let content = read_system_config()?;
    let new_content = remove_package_from_nix(&content, attribute);
    write_system_config(&new_content)?;

    let mut child = Command::new("sudo")
        .args(["-S", "nixos-rebuild", "switch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(format!("{}\n", sudo_password).as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        restore_system_backup().ok();
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn is_installed_system(attribute: &str) -> bool {
    read_system_config()
        .map(|c| c.contains(&format!("pkgs.{}", attribute)))
        .unwrap_or(false)
}

// ── Shared helpers ────────────────────────────────────────────

fn insert_package_into_nix(content: &str, attribute: &str) -> Result<String, String> {
    if content.contains("home.packages = with pkgs; [") {
        return Ok(content.replace(
            "home.packages = with pkgs; [",
            &format!("home.packages = with pkgs; [\n    pkgs.{}", attribute),
        ));
    }

    if content.contains("environment.systemPackages = with pkgs; [") {
        return Ok(content.replace(
            "environment.systemPackages = with pkgs; [",
            &format!(
                "environment.systemPackages = with pkgs; [\n    pkgs.{}",
                attribute
            ),
        ));
    }

    let pos = content
        .rfind('}')
        .ok_or("Could not find closing brace in Nix file")?;

    let mut new = content.to_string();
    new.insert_str(
        pos,
        &format!(
            "  home.packages = with pkgs; [\n    pkgs.{}\n  ];\n",
            attribute
        ),
    );
    Ok(new)
}

fn remove_package_from_nix(content: &str, attribute: &str) -> String {
    content
        .lines()
        .filter(|line| !line.contains(&format!("pkgs.{}", attribute)))
        .collect::<Vec<&str>>()
        .join("\n")
}

fn verify_sudo(password: &str) -> Result<(), String> {
    let mut child = Command::new("sudo")
        .args(["-S", "true"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(format!("{}\n", password).as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err("Incorrect sudo password".to_string())
    }
}

// Backup logic updated to use local file copying since we no longer need sudo for .dotfiles/
fn backup_system_config() -> Result<(), String> {
    let path = system_nix_path();
    let backup = path.with_extension("nix.bak");
    std::fs::copy(&path, &backup).map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_system_backup() -> Result<(), String> {
    let path = system_nix_path();
    let backup = path.with_extension("nix.bak");
    if backup.exists() {
        std::fs::copy(&backup, &path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn is_installed(attribute: &str) -> bool {
    is_installed_user(attribute) || is_installed_system(attribute)
}

pub fn backup_config() -> Result<(), String> {
    Ok(())
}

pub fn restore_backup() -> Result<(), String> {
    Ok(())
}

pub fn add_package(attribute: &str) -> Result<(), String> {
    add_package_user(attribute)
}
