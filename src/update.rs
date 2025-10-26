use reqwest::Client;
use serde::Deserialize;
use std::env;
use anyhow::Result;

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub async fn check_for_updates() -> Result<()> {
    let client = Client::new();
    let response = client
        .get("https://api.github.com/repos/tosterlolz/Owonero/releases/latest")
        .header("User-Agent", "owonero-rs")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(()); // Silently ignore update check failures
    }

    let release: GitHubRelease = response.json().await?;
    let latest_ver = release.tag_name.trim_start_matches('v');
    let current_ver = env!("CARGO_PKG_VERSION");

    if latest_ver != current_ver {
        println!("New version available: {} (current: {})", latest_ver, current_ver);
        println!("Available assets:");
        for asset in &release.assets {
            println!("  - {}: {}", asset.name, asset.browser_download_url);
        }
        // TODO: Implement download and install
        println!("Update installation not yet implemented in Rust version");
    } else {
        println!("You are running the latest version ({})", current_ver);
    }

    Ok(())
}