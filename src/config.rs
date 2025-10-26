use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

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

impl Default for Config {
    fn default() -> Self {
        Self {
            node_address: "localhost:6969".to_string(),
            daemon_port: 6969,
            web_port: 6767,
            wallet_path: "wallet.json".to_string(),
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

pub fn load_config(path: &str) -> Result<Config> {
    let data = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&data)?;
    Ok(config)
}

pub fn save_config(config: &Config, path: &str) -> Result<()> {
    let data = serde_json::to_string_pretty(config)?;
    fs::write(path, data)?;
    Ok(())
}