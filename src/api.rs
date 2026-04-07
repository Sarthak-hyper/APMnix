use serde::{Deserialize, Serialize};
use reqwest::blocking::Client;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub attribute: String,
}

#[derive(Debug, Deserialize)]
struct NixpkgsData {
    packages: std::collections::HashMap<String, NixPackage>,
}

#[derive(Debug, Deserialize)]
struct NixPackage {
    pname: String,
    version: String,
    meta: Meta,
}

#[derive(Debug, Deserialize)]
struct Meta {
    description: Option<String>,
}

/// Main function to get packages. 
/// Checks for 'packages.json' locally first to simulate API for testing.
pub fn fetch_all_packages() -> Result<Vec<Package>, String> {
    let local_file = "packages.json";

    // 1. Simulation Check: Load local file if it exists
    if Path::new(local_file).exists() {
        let file = File::open(local_file)
            .map_err(|e| format!("Found local file but couldn't open: {}", e))?;
        let reader = BufReader::new(file);
        
        let data: NixpkgsData = serde_json::from_reader(reader)
            .map_err(|e| format!("Local JSON parse failed: {}", e))?;
            
        return Ok(transform_data(data));
    }

    // 2. Real API Logic: Fallback if local file is missing
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60)) // Increased timeout for slow internet
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://channels.nixos.org/nixos-25.05/packages.json.br";

    let response = client
        .get(url)
        .header("Accept-Encoding", "br")
        .send()
        .map_err(|e| format!("Network request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let data: NixpkgsData = response
        .json()
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(transform_data(data))
}

/// Helper to convert raw Nix structure to our GUI-friendly Package struct
fn transform_data(data: NixpkgsData) -> Vec<Package> {
    data.packages
        .into_iter()
        .map(|(attr, pkg)| Package {
            name: pkg.pname,
            version: pkg.version,
            description: pkg.meta.description
                .unwrap_or_else(|| "No description available".to_string()),
            attribute: attr,
        })
        .collect()
}

/// Search packages locally after fetching
pub fn search_packages(all: &[Package], query: &str) -> Vec<Package> {
    let query = query.to_lowercase();
    all.iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&query)
                || p.description.to_lowercase().contains(&query)
                || p.attribute.to_lowercase().contains(&query)
        })
        .cloned()
        .take(50)
        .collect()
}

/// Curated list shown on home screen
pub fn get_curated(all: &[Package]) -> Vec<Package> {
    let curated = vec![
        "firefox", "chromium", "vlc", "gimp", "libreoffice",
        "vscode", "git", "vim", "neovim", "htop", "btop",
        "neofetch", "discord", "telegram-desktop", "obs-studio",
        "thunderbird", "inkscape", "blender", "mpv", "transmission",
        "flameshot", "python3", "nodejs", "gcc13", "cmake",
    ];

    curated.iter()
        .filter_map(|name| all.iter().find(|p| p.name == *name).cloned())
        .collect()
}
