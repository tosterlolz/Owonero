use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
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
    // Accept a path and return the parsed config. If the file doesn't exist,
    // create a default config, write it to disk, and return it.
    let path = Path::new(path);
    match fs::read_to_string(path) {
        Ok(data) => {
            let config: Config = serde_json::from_str(&data)
                .context("parsing config JSON")?;
            Ok(config)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let default = Config::default();
            // Try to persist the default so subsequent runs have a config file.
            if let Err(save_err) = save_config(&default, path) {
                // don't fail the whole load if saving fails; return the default with context
                anyhow::bail!("failed to write default config: {}", save_err);
            }
            Ok(default)
        }
        Err(e) => Err(e).context("reading config file"),
    }
}

pub fn save_config<P: AsRef<Path>>(config: &Config, path: P) -> Result<()> {
    let path = path.as_ref();
    let data = serde_json::to_string_pretty(config).context("serializing config")?;
    // Write directly for simplicity; could be made atomic by writing to a temp
    // file and renaming.
    fs::write(path, data).context("writing config file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn save_load_roundtrip() {
        let mut path = std::env::temp_dir();
        path.push(format!("owonero_config_test_{}.json", std::process::id()));
        let _ = fs::remove_file(&path);

        let mut cfg = Config::default();
        cfg.node_address = "127.0.0.1:1234".to_string();
        cfg.daemon_port = 1234;

    save_config(&cfg, &path).expect("save failed");
    let loaded = load_config(path.to_str().unwrap()).expect("load failed");
        assert_eq!(loaded.node_address, cfg.node_address);
        assert_eq!(loaded.daemon_port, cfg.daemon_port);

        let _ = fs::remove_file(&path);
    }
}