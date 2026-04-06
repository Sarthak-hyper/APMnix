use serde::{Deserialize, Serialize};
use reqwest::blocking::Client;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub attribute: String,
}

// Raw structure matching nixpkgs packages.json
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

// Download and cache nixpkgs package list
pub fn fetch_all_packages() -> Result<Vec<Package>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    // Official nixpkgs package list — no auth needed
    let url = "https://channels.nixos.org/nixos-25.05/packages.json.br";

    let response = client
        .get(url)
        .header("Accept-Encoding", "br")
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch packages: {}", response.status()));
    }

    let data: NixpkgsData = response
        .json()
        .map_err(|e| format!("Failed to parse packages: {}", e))?;

    let packages = data.packages
        .into_iter()
        .map(|(attr, pkg)| Package {
            name: pkg.pname,
            version: pkg.version,
            description: pkg.meta.description
                .unwrap_or_else(|| "No description available".to_string()),
            attribute: attr,
        })
        .collect();

    Ok(packages)
}

// Search packages locally after fetching
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

// Curated list shown on home screen
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
