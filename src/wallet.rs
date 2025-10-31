use anyhow::{Result, anyhow};
use hex;
use ring::rand::SystemRandom;
use ring::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub address: String,
    pub pub_key: String,
    pub priv_key: String,
    pub node_address: Option<String>,
}

impl Wallet {
    pub fn new() -> Result<Self> {
        let rng = SystemRandom::new();
        let pkcs8_doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
            .map_err(|_| anyhow!("Failed to generate key pair"))?;

        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8_doc.as_ref(), &rng)
                .map_err(|_| anyhow!("Failed to load key pair"))?;

        let pub_key_bytes = key_pair.public_key().as_ref();

        Ok(Self {
            // Use the public key hex as the wallet "address" so that
            // transactions and coinbase rewards can be indexed directly
            // by the public key. This makes comparisons unambiguous and
            // avoids separate address-generation logic during development.
            address: hex::encode(pub_key_bytes),
            pub_key: hex::encode(pub_key_bytes),
            // zapis PKCS#8 jako hex (można też base64)
            priv_key: hex::encode(pkcs8_doc.as_ref()),
            node_address: None,
        })
    }

    pub fn get_balance(&self, blockchain: &crate::blockchain::Blockchain) -> i64 {
        // Normalize address comparisons to be case-insensitive and trim whitespace.
        let my_addr = self.address.trim().to_lowercase();
        let mut balance = 0i64;
        for block in &blockchain.chain {
            for tx in &block.transactions {
                let tx_to = tx.to.trim().to_lowercase();
                let tx_from = tx.from.trim().to_lowercase();
                if tx_to == my_addr {
                    balance += tx.amount;
                }
                if tx_from == my_addr {
                    balance -= tx.amount;
                }
            }
        }
        balance
    }

    pub fn create_signed_transaction(
        &self,
        to: &str,
        amount: i64,
    ) -> Result<crate::blockchain::Transaction> {
        let mut tx = crate::blockchain::Transaction {
            from: self.address.clone(),
            pub_key: self.pub_key.clone(),
            to: to.to_string(),
            amount,
            signature: String::new(),
        };

        crate::blockchain::sign_transaction(&mut tx, &self.priv_key)?;
        Ok(tx)
    }
}

pub fn load_or_create_wallet(path: &str) -> Result<Wallet> {
    // Expand ~ to home directory if present
    let expanded_path = if path.starts_with("~") {
        if let Some(home) = std::env::var("HOME").ok() {
            let mut p = home;
            p.push_str(&path[1..]);
            p
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };
    let p = std::path::Path::new(&expanded_path);
    if p.exists() {
        let data = std::fs::read_to_string(&expanded_path)?;
        let wallet: Wallet = serde_json::from_str(&data)?;
        Ok(wallet)
    } else {
        // Ensure parent directories exist for the target path.
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let mut wallet = Wallet::new()?;
        if let Ok(cfg) = crate::config::load_config() {
            wallet.node_address = Some(cfg.node_address);
        }

        let data = serde_json::to_string_pretty(&wallet)?;
        std::fs::write(&expanded_path, data)?;
        Ok(wallet)
    }
}
