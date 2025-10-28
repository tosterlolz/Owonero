use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{PathBuf};
use std::io::ErrorKind;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node_address: String,
    pub daemon_port: u16,
    pub web_port: u16,
    pub wallet_path: String,
    pub mining_threads: usize,
    pub peers: Vec<String>,
    pub auto_update: bool,
    pub sync_on_startup: bool,
    pub target_block_time: i64,
    pub mining_intensity: u8,
    pub pool: bool,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.daemon_port == self.web_port {
            anyhow::bail!("daemon_port and web_port must be different");
        }
        if self.mining_threads == 0 {
            anyhow::bail!("mining_threads must be at least 1");
        }
        if self.mining_intensity > 100 {
            anyhow::bail!("mining_intensity must be <= 100");
        }
        // Sprawdź ścieżkę portfela
        let wallet_path = std::path::Path::new(&self.wallet_path);
        if let Some(parent) = wallet_path.parent() {
            if !parent.exists() {
                anyhow::bail!("Wallet path directory does not exist: {}", parent.display());
            }
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let base_dir = get_config_dir();
        let wallet_path = base_dir.join("wallet.json");

        Self {
            node_address: "owonero.yabai.buzz:6969".to_string(),
            daemon_port: 6969,
            web_port: 6767,
            wallet_path: wallet_path.to_string_lossy().to_string(),
            mining_threads: 1,
            peers: Vec::new(),
            auto_update: true,
            sync_on_startup: true,
            target_block_time: 30,
            mining_intensity: 100,
            pool: false,
        }
    }
}

/// Determine the configuration directory for the current platform.
/// Linux/macOS → `$HOME/.config/Owonero`
/// Windows → `%APPDATA%\Owonero`
fn get_config_dir() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        let owonero_dir: PathBuf = dir.join("Owonero");
        if !owonero_dir.exists() {
            let _ = fs::create_dir_all(&owonero_dir);
        }
        owonero_dir
    } else {
        // Fallback: current directory if home/config can't be determined
        PathBuf::from(".")
    }
}

/// Returns the full path to the config file (`config.json`)
pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn load_config() -> Result<Config> {
    let path = get_config_path();
    let data = fs::read_to_string(&path).context("reading config file")?;
    let config: Config = serde_json::from_str(&data).context("parsing config JSON")?;
    config.validate()?;  // Dodane
    let _ = Ok::<Config, anyhow::Error>(config);
    match fs::read_to_string(&path) {
        Ok(data) => {
            let config: Config = serde_json::from_str(&data).context("parsing config JSON")?;
            config.validate()?;
            Ok(config)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let default = Config::default();
            save_config(&default)?;
            Ok(default)
        }
        Err(e) => Err(e.into()),
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = get_config_path();
    let data = serde_json::to_string_pretty(config).context("serializing config")?;
    fs::write(&path, data).context("writing config file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn save_load_roundtrip() {
        let temp_dir = std::env::temp_dir().join("owonero_test_config");
        let _ = fs::create_dir_all(&temp_dir);
        let temp_file = temp_dir.join("config.json");

        let mut cfg = Config::default();
        cfg.node_address = "127.0.0.1:1234".to_string();
        cfg.daemon_port = 1234;

        // save to temporary directory
        fs::write(&temp_file, serde_json::to_string_pretty(&cfg).unwrap()).unwrap();

        let loaded: Config = serde_json::from_str(&fs::read_to_string(&temp_file).unwrap()).unwrap();
        assert_eq!(loaded.node_address, cfg.node_address);
        assert_eq!(loaded.daemon_port, cfg.daemon_port);
    }
}
