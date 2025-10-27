use serde::{Deserialize, Serialize};
use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING, KeyPair};
use ring::rand::SystemRandom;
use std::fs;
use anyhow::{Result, anyhow};
use hex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub address: String,
    pub pub_key: String,
    pub priv_key: String,
}

impl Wallet {
    pub fn generate_address() -> String {
        format!("OWO{:016x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())
    }

    pub fn new() -> Result<Self> {
        let rng = SystemRandom::new();
        let pkcs8_bytes = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
            .map_err(|_| anyhow!("Failed to generate key pair"))?;

        let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8_bytes.as_ref(), &SystemRandom::new())
            .map_err(|_| anyhow!("Failed to load key pair"))?;

        let pub_key_bytes = key_pair.public_key().as_ref();

        Ok(Self {
            address: Self::generate_address(),
            pub_key: hex::encode(pub_key_bytes),
            priv_key: hex::encode(pkcs8_bytes),
        })
    }

    pub fn get_balance(&self, blockchain: &crate::blockchain::Blockchain) -> i64 {
        let mut balance = 0i64;
        for block in &blockchain.chain {
            for tx in &block.transactions {
                if tx.to == self.address {
                    balance += tx.amount;
                }
                if tx.from == self.address {
                    balance -= tx.amount;
                }
            }
        }
        balance
    }

    pub fn create_signed_transaction(&self, to: &str, amount: i64) -> Result<crate::blockchain::Transaction> {
        let mut tx = crate::blockchain::Transaction {
            // Use address in `from` for human-readable bookkeeping and balance checks,
            // and include the public key separately so signature verification can use it.
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
    if std::path::Path::new(path).exists() {
        let data = fs::read_to_string(path)?;
        let wallet: Wallet = serde_json::from_str(&data)?;
        Ok(wallet)
    } else {
        let wallet = Wallet::new()?;
        let data = serde_json::to_string_pretty(&wallet)?;
        fs::write(path, data)?;
        Ok(wallet)
    }
}