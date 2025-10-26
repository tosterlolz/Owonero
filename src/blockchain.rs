use serde::{Deserialize, Serialize};
use sha3::Digest;
use std::env;
use std::cell::RefCell;

// RX/OWO Parameters (module-level so they can be reused without reallocating)
const SCRATCHPAD_SIZE: usize = 2 * 1024 * 1024; // 2MB scratchpad (like RandomX)
const DEFAULT_ITERATIONS: usize = 1024; // Number of computational iterations
const L1_CACHE_SIZE: usize = 16 * 1024; // 16KB L1 cache simulation
const L2_CACHE_SIZE: usize = 256 * 1024; // 256KB L2 cache simulation

thread_local! {
    // Reusable per-thread scratchpad to avoid allocating 2MB each hash
    static SCRATCHPAD_BUF: RefCell<Vec<u8>> = RefCell::new(vec![0u8; SCRATCHPAD_SIZE]);
}
use chrono::{DateTime, Utc};
use std::fs;
use anyhow::{Result, anyhow};
use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use hex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: i64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: DateTime<Utc>,
    pub transactions: Vec<Transaction>,
    pub prev_hash: String,
    pub hash: String,
    pub nonce: u32,
    pub difficulty: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub target_block_time: i64,
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            chain: vec![Self::create_genesis_block()],
            target_block_time: 30,
        }
    }

    pub fn create_genesis_block() -> Block {
        let mut block = Block {
            index: 0,
            timestamp: "2025-10-11T00:00:00Z".parse().unwrap(),
            transactions: vec![Transaction {
                from: "genesis".to_string(),
                to: "network".to_string(),
                amount: 0,
                signature: "".to_string(),
            }],
            prev_hash: "".to_string(),
            hash: "".to_string(),
            nonce: 0,
            difficulty: 1,
        };
        block.hash = Self::calculate_hash(&block);
        block
    }

    pub fn calculate_hash(block: &Block) -> String {
        // RX/OWO Algorithm - RandomX-inspired memory-hard PoW for Owonero
        // Features: 2MB scratchpad, complex memory access patterns, ASIC-resistant operations
        // Designed to be memory-hard and CPU-friendly for fair mining distribution
        let block_for_hash = BlockForHash {
            index: block.index,
            timestamp: block.timestamp,
            transactions: block.transactions.clone(),
            prev_hash: block.prev_hash.clone(),
            nonce: block.nonce,
        };
        let block_bytes = serde_json::to_vec(&block_for_hash).unwrap();

        // Determine iterations (configurable via OWONERO_MINING_ITERATIONS env var)
        let iterations = env::var("OWONERO_MINING_ITERATIONS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_ITERATIONS);

        let seed = sha3::Sha3_256::digest(&block_bytes);

        // Use a reusable thread-local scratchpad to avoid reallocations
        let result_hash = SCRATCHPAD_BUF.with(|buf| {
            let mut scratchpad = buf.borrow_mut();

            // Initialize RNG state from seed
            let mut rng_state = u64::from_le_bytes(seed[0..8].try_into().unwrap());

            // Fill scratchpad with pseudo-random data (overwrite previous contents)
            for i in 0..SCRATCHPAD_SIZE {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                scratchpad[i] = (rng_state >> 32) as u8;
            }

            // RX/OWO Main Loop - Memory-hard computation
            let mut a = u64::from_le_bytes(seed[8..16].try_into().unwrap());
            let mut b = u64::from_le_bytes(seed[16..24].try_into().unwrap());
            let mut c = u64::from_le_bytes(seed[24..32].try_into().unwrap());

            for iteration in 0..iterations {
                // Memory access pattern 1: Random access with complex addressing
                let addr1 = ((a.wrapping_add(b).wrapping_mul(c)) % (SCRATCHPAD_SIZE as u64 / 8)) as usize * 8;
                let mem_val1 = u64::from_le_bytes(scratchpad[addr1..addr1+8].try_into().unwrap());

                // Memory access pattern 2: Sequential with offset
                let addr2 = ((iteration * 64) + (a as usize % 1024)) % SCRATCHPAD_SIZE;
                let mem_val2 = scratchpad[addr2] as u64;

                // Memory access pattern 3: touch an L1-resident cache line (use prefetch hint if available)
                let l1_addr = (a % (L1_CACHE_SIZE as u64 / 8)) as usize * 8;
                #[cfg(target_arch = "x86_64")]
                unsafe {
                    // prefetch into L1 (T0)
                    let p = scratchpad.as_ptr().add(l1_addr) as *const i8;
                    _mm_prefetch(p, _MM_HINT_T0);
                }
                let l1_val = u64::from_le_bytes(scratchpad[l1_addr..l1_addr+8].try_into().unwrap());

                // Memory access pattern 4: touch an L2-resident cache line (use prefetch hint if available)
                let l2_addr = ((b % (L2_CACHE_SIZE as u64 / 8)) as usize * 8) % SCRATCHPAD_SIZE;
                #[cfg(target_arch = "x86_64")]
                unsafe {
                    // prefetch into L2 (T1) — hint, actual behavior depends on CPU
                    let p2 = scratchpad.as_ptr().add(l2_addr) as *const i8;
                    _mm_prefetch(p2, _MM_HINT_T1);
                }
                let l2_val = u64::from_le_bytes(scratchpad[l2_addr..l2_addr+8].try_into().unwrap());

                // Complex arithmetic operations (ASIC-resistant)
                a = a.wrapping_mul(mem_val1).wrapping_add(l1_val);
                b = (b ^ mem_val2).wrapping_sub(l2_val);
                c = c.rotate_left((mem_val1 % 64) as u32).wrapping_add(a ^ b);

                // Non-linear operations
                a ^= (a >> 17) | (a << 47); // Bit rotation
                b ^= (b >> 23) | (b << 41);
                c ^= (c >> 29) | (c << 35);

                // Memory write-back (modify scratchpad)
                let write_addr = ((a ^ b ^ c) % (SCRATCHPAD_SIZE as u64 / 8)) as usize * 8;
                let write_val = (a.wrapping_add(b).wrapping_mul(c)).to_le_bytes();
                scratchpad[write_addr..write_addr+8].copy_from_slice(&write_val);

                // Additional entropy from block data
                if iteration % 128 == 0 {
                    let block_byte = block_bytes.get(iteration % block_bytes.len()).unwrap_or(&0);
                    a ^= *block_byte as u64;
                    b ^= (*block_byte as u64).rotate_left(8);
                    c ^= (*block_byte as u64).rotate_left(16);
                }
            }

            // Final hash computation
            let mut final_input = Vec::new();
            final_input.extend_from_slice(&a.to_le_bytes());
            final_input.extend_from_slice(&b.to_le_bytes());
            final_input.extend_from_slice(&c.to_le_bytes());
            final_input.extend_from_slice(&block_bytes);

            // Mix in some scratchpad data
            for i in 0..32 {
                let idx = (a.wrapping_add(i as u64) % SCRATCHPAD_SIZE as u64) as usize;
                final_input.push(scratchpad[idx]);
            }

            let hash = sha3::Sha3_256::digest(&final_input);
            hex::encode(hash)
        });

        return result_hash;
    }

    pub fn get_dynamic_difficulty(&self) -> u32 {
        let min_difficulty = 1;
        let max_difficulty = 7;
        let window = 10;

        if self.chain.len() <= window {
            return min_difficulty;
        }

        let latest = &self.chain[self.chain.len() - 1];
        let prev = &self.chain[self.chain.len() - window - 1];
        let avg_block_time = (latest.timestamp - prev.timestamp).num_seconds() / window as i64;
        let mut diff = latest.difficulty as i32;

        if avg_block_time < self.target_block_time {
            diff += 1;
        } else if avg_block_time > self.target_block_time {
            diff -= 1;
        }

        if diff < min_difficulty as i32 {
            diff = min_difficulty as i32;
        }
        if diff > max_difficulty as i32 {
            diff = max_difficulty as i32;
        }

        diff as u32
    }

    pub fn validate_block(&self, block: &Block, difficulty: u32, skip_pow: bool) -> bool {
        if self.chain.is_empty() {
            // Genesis validation
            if block.index != 0 {
                eprintln!("Genesis block validation failed: Index must be 0, got {}", block.index);
                return false;
            }
            if !block.prev_hash.is_empty() {
                eprintln!("Genesis block validation failed: PrevHash must be empty, got {}", block.prev_hash);
                return false;
            }
            if Self::calculate_hash(block) != block.hash {
                eprintln!("Genesis block validation failed: Hash mismatch");
                return false;
            }
            return true;
        }

        let last = &self.chain[self.chain.len() - 1];
        if block.prev_hash != last.hash {
            eprintln!("Block {} validation failed: PrevHash mismatch", block.index);
            return false;
        }
        if Self::calculate_hash(block) != block.hash {
            eprintln!("Block {} validation failed: Hash mismatch", block.index);
            return false;
        }
        if block.index != last.index + 1 {
            eprintln!("Block {} validation failed: Index mismatch", block.index);
            return false;
        }

        if !skip_pow {
            // Check PoW
            let hash_bytes = hex::decode(&block.hash).unwrap_or_default();
            let mut valid_pow = true;
            for i in 0..((difficulty + 1) / 2) {
                let byte_idx = i as usize;
                if byte_idx >= hash_bytes.len() {
                    break;
                }
                let byte_val = hash_bytes[byte_idx];
                if difficulty > i * 2 && byte_val >> 4 != 0 {
                    valid_pow = false;
                    break;
                }
                if difficulty > i * 2 + 1 && (byte_val & 0x0F) != 0 {
                    valid_pow = false;
                    break;
                }
            }
            if !valid_pow {
                eprintln!("Block {} validation failed: PoW check failed", block.index);
                return false;
            }
        }

        // Validate transaction signatures
        for tx in &block.transactions {
            if tx.from == "coinbase" {
                // Coinbase transactions don't need signatures
                continue;
            }
            if !verify_transaction_signature(tx, &tx.from) {
                eprintln!("Block {} validation failed: Invalid transaction signature for tx from {} to {}", block.index, tx.from, tx.to);
                return false;
            }
        }

        true
    }

    pub fn add_block(&mut self, block: Block, difficulty: u32) -> bool {
        self.add_block_skip_pow(block, difficulty, false)
    }

    pub fn add_block_skip_pow(&mut self, block: Block, difficulty: u32, skip_pow: bool) -> bool {
        if self.validate_block(&block, difficulty, skip_pow) {
            self.chain.push(block);
            true
        } else {
            false
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self> {
        if !std::path::Path::new(path).exists() {
            let bc = Self::new();
            bc.save_to_file(path)?;
            return Ok(bc);
        }

        let data = fs::read_to_string(path)?;
        let mut bc: Blockchain = serde_json::from_str(&data)?;

        // Recalculate hashes
        for block in &mut bc.chain {
            block.hash = Self::calculate_hash(block);
        }

        if bc.chain.is_empty() {
            bc.chain = vec![Self::create_genesis_block()];
        }

        Ok(bc)
    }

    // Make mine_block an associated function that does not take a lock on the blockchain.
    // This lets miners compute blocks in parallel without holding the chain mutex.
    pub fn mine_block(prev_block: &Block, transactions: Vec<Transaction>, difficulty: u32, attempts: &mut u64) -> Block {
        let target_prefix = "0".repeat(difficulty as usize);

        let mut block = Block {
            index: prev_block.index + 1,
            timestamp: Utc::now(),
            transactions,
            prev_hash: prev_block.hash.clone(),
            hash: String::new(),
            nonce: 0,
            difficulty,
        };

        // RX/OWO mining - memory-hard algorithm
        // The algorithm is inherently memory-hard due to the 2MB scratchpad usage
        // No precomputation needed as each nonce creates unique memory access patterns

        loop {
            block.hash = Self::calculate_hash(&block);
            *attempts += 1;

            // Check if hash meets difficulty
            if block.hash.starts_with(&target_prefix) {
                break;
            }

            block.nonce += 1;
        }

        block
    }
}

#[derive(Serialize)]
struct BlockForHash {
    index: u64,
    timestamp: DateTime<Utc>,
    transactions: Vec<Transaction>,
    prev_hash: String,
    nonce: u32,
}

// Transaction signing functions
pub fn sign_transaction(tx: &mut Transaction, priv_key_hex: &str) -> Result<()> {
    let priv_key_bytes = hex::decode(priv_key_hex)?;
    let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &priv_key_bytes, &ring::rand::SystemRandom::new())
        .map_err(|_| anyhow!("Invalid private key"))?;

    let message = format!("{}|{}|{}", tx.from, tx.to, tx.amount);
    let signature = key_pair.sign(&ring::rand::SystemRandom::new(), message.as_bytes())
        .map_err(|_| anyhow!("Failed to sign transaction"))?;

    tx.signature = hex::encode(signature.as_ref());
    Ok(())
}

pub fn verify_transaction_signature(tx: &Transaction, pub_key_hex: &str) -> bool {
    let pub_key_bytes = match hex::decode(pub_key_hex) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let message = format!("{}|{}|{}", tx.from, tx.to, tx.amount);
    let sig_bytes = match hex::decode(&tx.signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let public_key = ring::signature::UnparsedPublicKey::new(
        &ring::signature::ECDSA_P256_SHA256_FIXED,
        &pub_key_bytes,
    );

    public_key.verify(message.as_bytes(), &sig_bytes).is_ok()
}
// Use x86_64 prefetch intrinsics when available to touch cache lines
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{_mm_prefetch, _MM_HINT_T0, _MM_HINT_T1};