use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;

const CONFIG_PATH: &str = "/etc/nixos/configuration.nix";

// Read config using sudo
fn read_config() -> Result<String, String> {
    let output = Command::new("sudo")
        .args(["cat", CONFIG_PATH])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

// Write config using sudo tee
fn write_config(content: &str) -> Result<(), String> {
    let mut child = Command::new("sudo")
        .args(["tee", CONFIG_PATH])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let output = child.wait_with_output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn add_package(attribute: &str) -> Result<(), String> {
    let content = read_config()?;

    // Already installed
    if content.contains(&format!("pkgs.{}", attribute)) {
        println!("{} already in configuration.nix", attribute);
        return Ok(());
    }

    let new_content = if content.contains("environment.systemPackages = with pkgs; [") {
        content.replace(
            "environment.systemPackages = with pkgs; [",
            &format!(
                "environment.systemPackages = with pkgs; [\n    pkgs.{}",
                attribute
            ),
        )
    } else {
        // Find last } and insert before it
        let pos = content.rfind('}')
            .ok_or("Could not find closing brace in configuration.nix")?;

        let mut new = content.clone();
        new.insert_str(
            pos,
            &format!(
                "  environment.systemPackages = with pkgs; [\n    pkgs.{}\n  ];\n",
                attribute
            ),
        );
        new
    };

    write_config(&new_content)?;
    println!("Added {} to configuration.nix", attribute);
    Ok(())
}

pub fn remove_package(attribute: &str) -> Result<(), String> {
    let content = read_config()?;

    let new_content = content
        .lines()
        .filter(|line| !line.contains(&format!("pkgs.{}", attribute)))
        .collect::<Vec<&str>>()
        .join("\n");

    write_config(&new_content)?;
    println!("Removed {} from configuration.nix", attribute);
    Ok(())
}

pub fn is_installed(attribute: &str) -> bool {
    read_config()
        .map(|c| c.contains(&format!("pkgs.{}", attribute)))
        .unwrap_or(false)
}

pub fn get_installed_packages() -> Vec<String> {
    let content = read_config().unwrap_or_default();
    let mut packages = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("pkgs.") {
            let pkg = line
                .trim_start_matches("pkgs.")
                .trim_end_matches(';')
                .trim()
                .to_string();
            if !pkg.is_empty() {
                packages.push(pkg);
            }
        }
    }

    packages
}

pub fn backup_config() -> Result<(), String> {
    let backup_path = format!("{}.bak", CONFIG_PATH);

    let mut child = Command::new("sudo")
        .args(["cp", CONFIG_PATH, &backup_path])
        .spawn()
        .map_err(|e| e.to_string())?;

    child.wait().map_err(|e| e.to_string())?;
    println!("Backup saved to {}", backup_path);
    Ok(())
}

pub fn restore_backup() -> Result<(), String> {
    let backup_path = format!("{}.bak", CONFIG_PATH);

    let mut child = Command::new("sudo")
        .args(["cp", &backup_path, CONFIG_PATH])
        .spawn()
        .map_err(|e| e.to_string())?;

    child.wait().map_err(|e| e.to_string())?;
    println!("Restored from backup");
    Ok(())
}
