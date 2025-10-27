use crate::blockchain::{Blockchain, Block, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerStats {
    pub total_hps: u64,
    pub sols: u64,
    pub avg_min: f64,
    pub avg_hour: f64,
    pub avg_day: f64,
    pub threads: usize,
    pub mined: u64,
    pub attempts: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub uptime: u64,
    pub pool_mode: bool,
}

pub async fn start_mining(
    wallet_path: &str,
    node_addr: &str,
    blocks_to_mine: u64,
    threads: usize,
    pool: bool,
    intensity: u8,
    stats_tx: Option<mpsc::Sender<MinerStats>>,
    log_tx: Option<mpsc::Sender<String>>,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) -> anyhow::Result<()> {
    let wallet = crate::wallet::load_or_create_wallet(wallet_path)?;

    // Prefer sending logs into the TUI when available. Fall back to stdout if not.
    if let Some(ref tx) = log_tx {
        let _ = tx.send(format!("Mining for wallet {} to node {}", wallet.address, node_addr)).await;
    } else {
        println!("Mining for wallet {} to node {}", wallet.address, node_addr);
    }

    // Connect to node
    let stream = TcpStream::connect(node_addr).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let mut greeting = String::new();
    reader.read_line(&mut greeting).await?;
    if let Some(ref tx) = log_tx {
        let _ = tx.send(format!("Connected to node: {}", greeting.trim())).await;
    } else {
        // idk what to do here so yk
    }

    // Get current chain
    writer.write_all(b"getchain\n").await?;
    let mut chain_data = String::new();
    reader.read_line(&mut chain_data).await?;
    let blockchain: Blockchain = serde_json::from_str(&chain_data.trim())?;

    let blockchain = Arc::new(Mutex::new(blockchain));
    // Shared mempool (kept in sync with node via poller)
    let mempool_shared: Arc<Mutex<Vec<crate::blockchain::Transaction>>> = Arc::new(Mutex::new(Vec::new()));
    let attempts = Arc::new(AtomicU64::new(0));
    let mined = Arc::new(AtomicU64::new(0));

    let (block_tx, mut block_rx) = mpsc::channel::<Block>(threads * 2);
    let (share_tx, mut share_rx) = mpsc::channel::<(String, u32, u64, Block)>(threads * 2);

    // Lightweight std channels to buffer worker->submitter communication without blocking workers
    let (block_sync_tx, block_sync_rx) = std::sync::mpsc::channel::<Block>();
    let (share_sync_tx, share_sync_rx) = std::sync::mpsc::channel::<(String, u32, u64, Block)>();

    // Stats tracking
    let attempts_history: Arc<Mutex<VecDeque<u64>>> = Arc::new(Mutex::new(VecDeque::new()));
    let accepted = Arc::new(AtomicU64::new(0));
    let rejected = Arc::new(AtomicU64::new(0));
    let start_time = std::time::Instant::now();

    // Shutdown flag shared with OS threads
    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    if let Some(mut rx) = shutdown_rx {
        let flag = shutdown_flag.clone();
        tokio::spawn(async move {
            // Wait for shutdown signal
            loop {
                if rx.changed().await.is_ok() {
                    if *rx.borrow() {
                        flag.store(true, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                } else {
                    break;
                }
            }
        });
    }

    // Block submitter
    let node_addr_clone = node_addr.to_string();
    let log_tx_clone1 = log_tx.clone();
    let accepted_clone1 = accepted.clone();
    let rejected_clone1 = rejected.clone();
    let mempool_for_submitter = mempool_shared.clone();
    let submitter_handle = tokio::spawn(async move {
        while let Some(block) = block_rx.recv().await {
            // Create new connection for each submission
            if let Ok(mut stream) = TcpStream::connect(&node_addr_clone).await {
                let (reader, mut writer) = stream.split();
                let mut reader = BufReader::new(reader);

                // Skip greeting
                let mut greeting = String::new();
                let _ = reader.read_line(&mut greeting).await;

                // Submit block
                writer.write_all(b"submitblock\n").await?;
                let block_json = serde_json::to_string(&block)?;
                writer.write_all(format!("{}\n", block_json).as_bytes()).await?;

                let mut response = String::new();
                reader.read_line(&mut response).await?;
                let response = response.trim();

                if response == "ok" {
                    accepted_clone1.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone1 {
                        let _ = tx.send(format!("Block accepted! Index={} Hash={}", block.index, block.hash)).await;
                    } else {
                        println!("Block accepted! Index={} Hash={}", block.index, block.hash);
                    }
                    // Remove included transactions from local mempool (except coinbase)
                    {
                        let mut mp = mempool_for_submitter.lock().unwrap();
                        mp.retain(|t| {
                            // keep txs that are NOT in the block (match by signature)
                            !block.transactions.iter().any(|bt| bt.signature == t.signature)
                        });
                    }
                } else {
                    rejected_clone1.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone1 {
                        let _ = tx.send(format!("Node rejected block: {}", response)).await;
                    } else {
                        eprintln!("Node rejected block: {}", response);
                    }
                }
            } else {
                rejected_clone1.fetch_add(1, Ordering::Relaxed);
                if let Some(ref tx) = log_tx_clone1 {
                    let _ = tx.send("Failed to connect to node for block submission".to_string()).await;
                } else {
                    eprintln!("Failed to connect to node for block submission");
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    // Share submitter
    let node_addr_clone2 = node_addr.to_string();
    let log_tx_clone2 = log_tx.clone();
    let accepted_clone2 = accepted.clone();
    let rejected_clone2 = rejected.clone();
    let share_submitter_handle = tokio::spawn(async move {
        while let Some((wallet_addr, nonce, attempts_val, block)) = share_rx.recv().await {
            // Create new connection for each submission
            if let Ok(mut stream) = TcpStream::connect(&node_addr_clone2).await {
                let (reader, mut writer) = stream.split();
                let mut reader = BufReader::new(reader);

                // Skip greeting
                let mut greeting = String::new();
                let _ = reader.read_line(&mut greeting).await;

                // Submit share
                writer.write_all(b"submitshare\n").await?;
                let share = serde_json::json!({
                    "wallet": wallet_addr,
                    "nonce": nonce,
                    "attempts": attempts_val,
                    "block": block
                });
                let share_json = serde_json::to_string(&share)?;
                writer.write_all(format!("{}\n", share_json).as_bytes()).await?;

                let mut response = String::new();
                reader.read_line(&mut response).await?;
                let response = response.trim();

                if response == "ok" {
                    accepted_clone2.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone2 {
                        let _ = tx.send(format!("Share accepted: {} attempts", attempts_val)).await;
                    } else {
                        println!("Share accepted: {} attempts", attempts_val);
                    }
                } else {
                    rejected_clone2.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone2 {
                        let _ = tx.send(format!("Node rejected share: {}", response)).await;
                    } else {
                        eprintln!("Node rejected share: {}", response);
                    }
                }
            } else {
                rejected_clone2.fetch_add(1, Ordering::Relaxed);
                if let Some(ref tx) = log_tx_clone2 {
                    let _ = tx.send("Failed to connect to node for share submission".to_string()).await;
                } else {
                    eprintln!("Failed to connect to node for share submission");
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    // Background mempool poller: periodically fetch mempool from node
    {
        let node_addr = node_addr.to_string();
        let mempool_clone = mempool_shared.clone();
        // let log_tx_clone = log_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                // try to fetch mempool
                if let Ok(mut stream) = TcpStream::connect(&node_addr).await {
                    let (r, mut w) = stream.split();
                    let mut r = BufReader::new(r);
                    // Skip greeting
                    let mut tmp = String::new();
                    let _ = r.read_line(&mut tmp).await;
                    // Request mempool
                    if w.write_all(b"getmempool\n").await.is_ok() {
                        let mut memline = String::new();
                        if r.read_line(&mut memline).await.is_ok() {
                            if let Ok(vec_tx) = serde_json::from_str::<Vec<crate::blockchain::Transaction>>(memline.trim()) {
                                let mut mp = mempool_clone.lock().unwrap();
                                *mp = vec_tx;
                            }
                        }
                    }
                }
            }
        });
    }

    // Forwarder threads: move from std channels into tokio mpsc so workers never block
    let _block_forwarder = {
        let block_tx = block_tx.clone();
        std::thread::spawn(move || {
            while let Ok(block) = block_sync_rx.recv() {
                // forward into async channel (may block if async channel is full)
                let _ = block_tx.blocking_send(block);
            }
        })
    };

    let _share_forwarder = {
        let share_tx = share_tx.clone();
        std::thread::spawn(move || {
            while let Ok(share) = share_sync_rx.recv() {
                let _ = share_tx.blocking_send(share);
            }
        })
    };

    // Stats reporter
    let stats_handle = if let Some(tx) = stats_tx {
        let attempts = attempts.clone();
        let mined = mined.clone();
        let attempts_history = attempts_history.clone();
        let accepted = accepted.clone();
        let rejected = rejected.clone();
        let start_time = start_time.clone();
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                let h = attempts.swap(0, Ordering::Relaxed);
                let sols = mined.load(Ordering::Relaxed);
                let acc = accepted.load(Ordering::Relaxed);
                let rej = rejected.load(Ordering::Relaxed);
                let uptime = start_time.elapsed().as_secs();

                // Update history - don't hold lock across await
                {
                    let mut history = attempts_history.lock().unwrap();
                    history.push_back(h);
                    if history.len() > 86400 {
                        history.pop_front();
                    }
                }

                let history_clone: Vec<u64> = {
                    let history = attempts_history.lock().unwrap();
                    history.iter().cloned().collect()
                };

                let avg_min = history_clone.iter().rev().take(60).sum::<u64>() as f64 / 60.0;
                let avg_hour = history_clone.iter().rev().take(3600).sum::<u64>() as f64 / 3600.0;
                let avg_day = history_clone.iter().sum::<u64>() as f64 / history_clone.len() as f64;

                let stats = MinerStats {
                    total_hps: h,
                    sols,
                    avg_min,
                    avg_hour,
                    avg_day,
                    threads,
                    mined: sols,
                    attempts: h,
                    accepted: acc,
                    rejected: rej,
                    uptime,
                    pool_mode: pool,
                };

                let _ = tx.send(stats).await;
            }
        }))
    } else {
        None
    };

    // Mining workers as dedicated OS threads
    let mut worker_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    for _id in 0..threads {
        let blockchain = blockchain.clone();
        let attempts = attempts.clone();
        let mined = mined.clone();
        let wallet = wallet.clone();
        let shutdown = shutdown_flag.clone();
        let block_sync_tx = block_sync_tx.clone();
        let share_sync_tx = share_sync_tx.clone();
        let mempool_for_worker = mempool_shared.clone();

        let handle = std::thread::spawn(move || {
            loop {
                if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                // Get block template (lock briefly)
                let (prev_block, _dyn_diff, diff) = {
                    let bc = blockchain.lock().unwrap();
                    let prev_block = bc.chain.last().unwrap().clone();
                    let dyn_diff = bc.get_dynamic_difficulty();
                    let diff = if pool { dyn_diff.saturating_sub(2).max(1) } else { dyn_diff };
                    (prev_block, dyn_diff, diff)
                };

                // Collect transactions from mempool (clone under lock)
                let mut txs: Vec<Transaction> = Vec::new();
                {
                    let mp = mempool_for_worker.lock().unwrap();
                    // include up to 10 transactions
                    for t in mp.iter().take(10) {
                        txs.push(t.clone());
                    }
                }

                let coinbase = Transaction {
                    from: "coinbase".to_string(),
                    pub_key: String::new(),
                    to: wallet.address.clone(),
                    amount: 1,
                    signature: String::new(),
                };

                // Prepend coinbase to transactions included in the block
                let mut block_txs = Vec::with_capacity(1 + txs.len());
                block_txs.push(coinbase);
                block_txs.extend(txs.into_iter());

                let mut local_attempts = 0u64;
                // Pass a direct reference to the shared atomic so mine_block can
                // flush attempt counts periodically for responsive hashrate reporting.
                let block = Blockchain::mine_block(&prev_block, block_txs, diff, &mut local_attempts, Some(&*attempts));

                if pool {
                    let share = (wallet.address.clone(), block.nonce, local_attempts, block);
                    // Send to std sync channel so workers never block on async channel fullness
                    let _ = share_sync_tx.send(share);
                } else {
                    mined.fetch_add(1, Ordering::Relaxed);
                    // Send to std sync channel so workers never block on async channel fullness
                    let _ = block_sync_tx.send(block);
                }

                // CPU throttling (blocking sleep)
                if intensity < 100 {
                    let throttle_ms = ((100 - intensity) as f64 / 100.0 * 10.0) as u64;
                    if throttle_ms > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(throttle_ms));
                    }
                }
            }
        });

        worker_handles.push(handle);
    }

    // Background poller: keep local blockchain copy up-to-date by polling node periodically.
    // This reduces wasted work when other miners find blocks and the local template becomes stale.
    let _blockchain_poller = {
        let blockchain = blockchain.clone();
        let node_addr = node_addr.to_string();
        let shutdown = shutdown_flag.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                match TcpStream::connect(&node_addr).await {
                    Ok(s) => {
                        let (r, mut w) = s.into_split();
                        let mut r = BufReader::new(r);
                        // Skip greeting
                        let mut tmp = String::new();
                        let _ = r.read_line(&mut tmp).await;
                        // Request chain
                        let _ = w.write_all(b"getchain\n").await;
                        let mut chain_line = String::new();
                        if r.read_line(&mut chain_line).await.is_ok() {
                            if let Ok(new_chain) = serde_json::from_str::<crate::blockchain::Blockchain>(chain_line.trim()) {
                                let mut bc_lock = blockchain.lock().unwrap();
                                // If newer, replace local copy
                                if new_chain.chain.len() > bc_lock.chain.len() {
                                    *bc_lock = new_chain;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // ignore connection errors; will retry on next tick
                    }
                }
            }
        })
    };

    // Wait for completion or cancellation
    if blocks_to_mine > 0 {
        while mined.load(Ordering::Relaxed) < blocks_to_mine && !shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
            sleep(Duration::from_millis(200)).await;
        }
    } else {
        // Run until shutdown
        while !shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
            sleep(Duration::from_millis(200)).await;
        }
    }

    // Signal threads to stop and join them
    shutdown_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    for handle in worker_handles {
        let _ = handle.join();
    }
    // Drop senders to close channels so submitters exit
    drop(block_tx);
    drop(share_tx);

    // Wait for submitter tasks to finish
    submitter_handle.abort();
    share_submitter_handle.abort();
    if let Some(handle) = stats_handle {
        handle.abort();
    }

    Ok(())
}