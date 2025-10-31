use anyhow::Result;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use std::env;

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
    let latest_ver_str = release.tag_name.trim_start_matches('v');
    let current_ver_str = env!("CARGO_PKG_VERSION");

    // Prefer semantic version comparison; fall back to string inequality if parsing fails
    match (
        Version::parse(latest_ver_str),
        Version::parse(current_ver_str),
    ) {
        (Ok(latest_ver), Ok(current_ver)) => {
            if latest_ver > current_ver {
                println!(
                    "New version available: {} (current: {})",
                    latest_ver, current_ver
                );
                println!("Available assets:");
                for asset in &release.assets {
                    println!("  - {}: {}", asset.name, asset.browser_download_url);
                }
                println!("Update installation not yet implemented in Rust version");
            } else if latest_ver == current_ver {
                println!("You are running the latest version ({})", current_ver);
            } else {
                println!(
                    "You are running a newer version ({}) than the latest release ({})",
                    current_ver, latest_ver
                );
            }
        }
        // Fallback: compare as strings (legacy behavior)
        _ => {
            if latest_ver_str != current_ver_str {
                println!(
                    "New version available: {} (current: {})",
                    latest_ver_str, current_ver_str
                );
                println!("Available assets:");
                for asset in &release.assets {
                    println!("  - {}: {}", asset.name, asset.browser_download_url);
                }
                println!("Update installation not yet implemented in Rust version");
            } else {
                println!("You are running the latest version ({})", current_ver_str);
            }
        }
    }

    Ok(())
}
