use serde::{Deserialize, Serialize};
use sha3::Digest;
use std::env;
use std::path::Path;
use std::time::{Duration, Instant};
use std::cell::RefCell;
use chrono::{DateTime, Utc};
use std::fs;
use anyhow::{Result, anyhow, Context};
use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use hex;

// RandomX-inspired RX/OWO Parameters (module-level so they can be reused without reallocating)
// These defaults are conservative; they can be tuned with environment variables
// to trade CPU work vs latency and to attempt hugepage usage.
const SCRATCHPAD_SIZE: usize = 2 * 1024 * 1024; // Default: 2MB scratchpad (RandomX-like)
const DEFAULT_ITERATIONS: usize = 2048; // Default iterations; tuned for reasonable CPU work
const L1_CACHE_SIZE: usize = 16 * 1024; // 16KB L1 cache simulation
const L2_CACHE_SIZE: usize = 256 * 1024; // 256KB L2 cache simulation

thread_local! {
    // Reusable per-thread scratchpad to avoid allocating 2MB each hash.
    // We attempt to enable huge pages (transparent hugepages via madvise) when
    // the environment variable `OWONERO_USE_HUGEPAGES=1` is set. If enabled but
    // the kernel does not support it, we silently fall back to normal pages.
    static SCRATCHPAD_BUF: RefCell<Vec<u8>> = RefCell::new(init_scratchpad());
}


// NOTE: the cancellable mining helper is implemented as an associated
// function on `Blockchain` below. Keeping a single implementation avoids
// duplication and potential name-resolution/visibility confusion.

fn init_scratchpad() -> Vec<u8> {
    // Allow adjusting scratchpad size with environment variable (bytes)
    let size = std::env::var("OWONERO_SCRATCHPAD_SIZE").ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&v| v >= 1024)
        .unwrap_or(SCRATCHPAD_SIZE);

    let mut buf = vec![0u8; size];

    // Only attempt to enable transparent huge pages when user opts in.
    let try_huge = std::env::var("OWONERO_USE_HUGEPAGES").map(|v| v != "0" && v.to_lowercase() != "false").unwrap_or(false);
    if try_huge {
        #[cfg(target_os = "linux")]
        {
            use std::io;
            unsafe {
                let ret = libc::madvise(buf.as_mut_ptr() as *mut libc::c_void, buf.len(), libc::MADV_HUGEPAGE);
                if ret == 0 {
                    eprintln!("OWONERO: attempted MADV_HUGEPAGE for scratchpad ({} bytes)", buf.len());
                } else {
                    let err = io::Error::last_os_error();
                    eprintln!("OWONERO: MADV_HUGEPAGE failed: {}. Falling back to normal pages.", err);
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            eprintln!("OWONERO: OWONERO_USE_HUGEPAGES=1 set on Windows, but automatic large page allocation is not implemented; falling back to normal pages.");
        }
    }

    buf
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub pub_key: String,
    pub to: String,
    pub amount: i64,
    pub signature: String,
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
                pub_key: String::new(),
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

        // Use a reusable thread-local scratchpad to avoid reallocations and operate
        // on u64 words for better throughput.
        let result_hash = SCRATCHPAD_BUF.with(|buf| {
            let mut scratchpad = buf.borrow_mut();

            // Ensure scratchpad len is a multiple of 8 for safe u64 views
            let sp_len = scratchpad.len();

            // Initialize RNG state from seed
            let mut rng_state = u64::from_le_bytes(seed[0..8].try_into().unwrap());

            // Fill scratchpad with pseudo-random data (word-wise) for faster writes
            unsafe {
                let ptr = scratchpad.as_mut_ptr() as *mut u64;
                let words = sp_len / 8;
                for i in 0..words {
                    rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    // spread bits to 64-bit value
                    let v = (rng_state >> 1) ^ (rng_state << 33);
                    ptr.add(i).write_unaligned(v.to_le());
                }
                // If there are leftover bytes (unlikely when size multiple of 8), leave them
            }

            // RX/OWO Main Loop - Memory-hard computation
            let mut a = u64::from_le_bytes(seed[8..16].try_into().unwrap());
            let mut b = u64::from_le_bytes(seed[16..24].try_into().unwrap());
            let mut c = u64::from_le_bytes(seed[24..32].try_into().unwrap());

            // Operating on u64 words reduces bounds checks and increases throughput.
            let sp_words = sp_len / 8;
            unsafe {
                let sp_ptr = scratchpad.as_mut_ptr() as *mut u64;
                for iteration in 0..iterations {
                    // Memory access pattern 1: Random word access
                    let idx1 = ((a.wrapping_add(b).wrapping_mul(c)) % (sp_words as u64)) as usize;
                    let mem_val1 = sp_ptr.add(idx1).read_unaligned();

                    // Memory access pattern 2: Sequential with offset (byte-level offset folded into word index)
                    let idx2 = (((iteration as usize * 8) + (a as usize % 1024)) % sp_len) / 8;
                    let mem_val2 = sp_ptr.add(idx2).read_unaligned();

                    // Prefetch hints where available
                    #[cfg(target_arch = "x86_64")]
                    {
                        use core::arch::x86_64::_mm_prefetch;
                        use core::arch::x86_64::_MM_HINT_T0;
                        let p = sp_ptr.add(idx1) as *const i8;
                        _mm_prefetch(p, _MM_HINT_T0);
                    }

                    // Touch L1/L2 simulated addresses
                    let l1_idx = (a % (L1_CACHE_SIZE as u64 / 8)) as usize % sp_words;
                    let l1_val = sp_ptr.add(l1_idx).read_unaligned();

                    let l2_idx = ((b % (L2_CACHE_SIZE as u64 / 8)) as usize) % sp_words;
                    let l2_val = sp_ptr.add(l2_idx).read_unaligned();

                    // Mix operations - designed to keep CPU busy and to have memory-dependent
                    // data-dependent addressing (RandomX-like)
                    a = a.wrapping_mul(mem_val1).wrapping_add(l1_val);
                    b = (b ^ mem_val2).wrapping_sub(l2_val);
                    c = c.rotate_left((mem_val1 % 64) as u32).wrapping_add(a ^ b);

                    // Non-linear mixing
                    a ^= a.rotate_right(17);
                    b ^= b.rotate_right(23);
                    c ^= c.rotate_right(29);

                    // Memory write-back (modify scratchpad)
                    let write_idx = ((a ^ b ^ c) % (sp_words as u64)) as usize;
                    let write_val = a.wrapping_add(b).wrapping_mul(c);
                    sp_ptr.add(write_idx).write_unaligned(write_val.to_le());

                    // Additional entropy from block data occasionally
                    if iteration & 127 == 0 {
                        let block_byte = *block_bytes.get(iteration % block_bytes.len()).unwrap_or(&0);
                        a ^= block_byte as u64;
                        b ^= (block_byte as u64).rotate_left(8);
                        c ^= (block_byte as u64).rotate_left(16);
                    }
                }
            }

            // Final hash computation - preallocate to avoid reallocations
            let mut final_input = Vec::with_capacity(8 * 3 + block_bytes.len() + 32);
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

    pub fn verify_chain(&self) -> Result<()> {
        for i in 1..self.chain.len() {
            let prev = &self.chain[i - 1];
            let cur = &self.chain[i];

            if cur.prev_hash != prev.hash {
                anyhow::bail!(
                    "chain broken at index {}: prev_hash {} != prev.hash {}",
                    cur.index,
                    cur.prev_hash,
                    prev.hash
                );
            }

            let calc = Self::calculate_hash(cur);
            if calc != cur.hash {
                anyhow::bail!("invalid hash at index {}: {} != {}", cur.index, calc, cur.hash);
            }
        }
        Ok(())
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
            if !verify_transaction_signature(tx, &tx.pub_key) {
                eprintln!("Block {} validation failed: Invalid transaction signature for tx from {} to {}", block.index, tx.from, tx.to);
                return false;
            }
        }

        true
    }

    /// Validate a block but return a textual error describing the first failure, if any.
    pub fn validate_block_verbose(&self, block: &Block, difficulty: u32, skip_pow: bool) -> Option<String> {
        if self.chain.is_empty() {
            // Genesis validation
            if block.index != 0 {
                return Some(format!("Genesis block validation failed: Index must be 0, got {}", block.index));
            }
            if !block.prev_hash.is_empty() {
                return Some(format!("Genesis block validation failed: PrevHash must be empty, got {}", block.prev_hash));
            }
            if Self::calculate_hash(block) != block.hash {
                return Some("Genesis block validation failed: Hash mismatch".to_string());
            }
            return None;
        }

        let last = &self.chain[self.chain.len() - 1];
        if block.prev_hash != last.hash {
            return Some(format!("PrevHash mismatch: expected {} got {}", last.hash, block.prev_hash));
        }
        if Self::calculate_hash(block) != block.hash {
            return Some("Hash mismatch".to_string());
        }
        if block.index != last.index + 1 {
            return Some(format!("Index mismatch: expected {} got {}", last.index + 1, block.index));
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
                return Some("PoW check failed".to_string());
            }
        }

        // Validate transaction signatures
        for tx in &block.transactions {
            if tx.from == "coinbase" {
                // Coinbase transactions don't need signatures
                continue;
            }
            if !verify_transaction_signature(tx, &tx.pub_key) {
                return Some(format!("Invalid transaction signature for tx from {} to {}", tx.from, tx.to));
            }
        }

        None
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

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();

        if !path_ref.exists() {
            let bc = Self::new();
            bc.save_to_file(path_ref)?;
            return Ok(bc);
        }

        let data = fs::read_to_string(path_ref)?;
        let mut bc: Blockchain = serde_json::from_str(&data)?;

        // Recalculate hashes for safety
        for block in &mut bc.chain {
            block.hash = Self::calculate_hash(block);
        }

        if bc.chain.is_empty() {
            bc.chain = vec![Self::create_genesis_block()];
        }

        // Verify integrity before returning
        bc.verify_chain().context("loaded blockchain failed integrity check")?;

        Ok(bc)
    }

    // Make mine_block an associated function that does not take a lock on the blockchain.
    // This lets miners compute blocks in parallel without holding the chain mutex.
    /// Mine a block using the given template. The function will periodically
    /// check `attempts_atomic` to flush local attempt counters and will also
    /// check `chain_version` (if provided) to abort mining early when the
    /// local chain version changes (so workers stop working on stale templates).
    ///
    /// Returns `Some(Block)` when a valid block is found, or `None` when
    /// mining was aborted due to a chain version update.
    pub fn mine_block_with_cancel(
        prev_block: &Block,
        transactions: Vec<Transaction>,
        difficulty: u32,
        attempts: &mut u64,
        attempts_atomic: Option<&std::sync::atomic::AtomicU64>,
        chain_version: Option<&std::sync::atomic::AtomicU64>,
    ) -> Option<Block> {
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

        // To provide responsive hashrate reporting, optionally flush local attempt
        // counters into a shared atomic counter periodically. This avoids waiting
        // until a full block is found to report attempts. The threshold may be
        // configured with OWONERO_MINING_FLUSH (attempts). Default is 64 to
        // give timely hashrate updates without excessive atomic traffic.
        let mut flush_chunk: u64 = 0;
        // Number of attempts to buffer before flushing into the shared atomic.
        let flush_threshold: u64 = std::env::var("OWONERO_MINING_FLUSH")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(64);
        // Also flush at least every X milliseconds to keep hashrate reporting
        // timely when attempt rate is low. Configurable via
        // OWONERO_MINING_FLUSH_MS (milliseconds). Default 250ms.
        let flush_interval_ms: u64 = std::env::var("OWONERO_MINING_FLUSH_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(250);
        let mut last_flush = Instant::now();

        // Snapshot chain version at start; if it changes we abort.
        let start_version = chain_version.map(|v| v.load(std::sync::atomic::Ordering::Relaxed));

        loop {
            block.hash = Self::calculate_hash(&block);
            *attempts += 1;
            flush_chunk += 1;

            // Periodically flush into the shared atomic counter if provided.
                if let Some(at) = attempts_atomic {
                    if flush_chunk >= flush_threshold || last_flush.elapsed() >= Duration::from_millis(flush_interval_ms) {
                        at.fetch_add(flush_chunk, std::sync::atomic::Ordering::Relaxed);
                        // reset chunk after flushing
                        flush_chunk = 0;
                        last_flush = Instant::now();
                    }
                }

            // Check if hash meets difficulty
            if block.hash.starts_with(&target_prefix) {
                // flush any remaining attempts
                if let Some(at) = attempts_atomic {
                    if flush_chunk > 0 {
                        at.fetch_add(flush_chunk, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                return Some(block);
            }

            // Periodically check whether the chain version changed; if so,
            // abort mining this template to avoid PrevHash mismatches.
            if let Some(v) = chain_version {
                let cur = v.load(std::sync::atomic::Ordering::Relaxed);
                if Some(cur) != start_version {
                    // Optionally flush remaining attempts before aborting
                    if let Some(at) = attempts_atomic {
                        if flush_chunk > 0 {
                            at.fetch_add(flush_chunk, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    return None;
                }
            }

            block.nonce += 1;
        }
    }

    /// Backwards-compatible wrapper kept for callers that expect the old
    /// synchronous behaviour. This will block until a valid block is found
    /// and will not return early on chain updates.
    pub fn mine_block(
        prev_block: &Block,
        transactions: Vec<Transaction>,
        difficulty: u32,
        attempts: &mut u64,
        attempts_atomic: Option<&std::sync::atomic::AtomicU64>,
    ) -> Block {
        // Call the cancellable variant with no chain_version so it behaves
        // like the original function and will not abort early.
        match Self::mine_block_with_cancel(prev_block, transactions, difficulty, attempts, attempts_atomic, None) {
            Some(b) => b,
            None => panic!("mine_block unexpectedly aborted when no chain_version provided"),
        }
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
    // If the provided pub_key_hex is empty (older format), fall back to using tx.from
    let key_hex = if pub_key_hex.is_empty() { &tx.from } else { pub_key_hex };

    let pub_key_bytes = match hex::decode(key_hex) {
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