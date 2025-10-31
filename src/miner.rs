use crate::blockchain::{Block, Blockchain};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

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
    node_addr: &str,
    blocks_to_mine: u64,
    threads: usize,
    pool: bool,
    _intensity: u8,
    stats_tx: Option<mpsc::Sender<MinerStats>>,
    log_tx: Option<mpsc::Sender<String>>,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) -> anyhow::Result<()> {
    let wallet = crate::config::load_wallet()?;

    // Prefer sending logs into the TUI when available. Fall back to stdout if not.
    if let Some(ref tx) = log_tx {
        let _ = tx
            .send(format!(
                "Mining for wallet {} to node {}",
                wallet.address, node_addr
            ))
            .await;
    }

    // Try to connect to node and fetch the authoritative chain. If the node
    // is unreachable or returns invalid data, fall back to the local
    // `blockchain.json` (or create a new chain). This lets mining continue
    // in solo/offline scenarios instead of failing outright.
    let mut greeting = String::new();
    let blockchain: Blockchain = match TcpStream::connect(node_addr).await {
        Ok(stream) => {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            // Try to read greeting (ignore errors)
            let _ = reader.read_line(&mut greeting).await;
            if let Some(ref tx) = log_tx {
                let _ = tx
                    .send(format!("Connected to node: {}", greeting.trim()))
                    .await;
            }

            // Request chain from node
            if writer.write_all(b"getchain\n").await.is_ok() {
                let mut chain_data = String::new();
                if reader.read_line(&mut chain_data).await.is_ok() {
                    if let Ok(bc) = serde_json::from_str::<Blockchain>(chain_data.trim()) {
                        bc
                    } else {
                        if let Some(ref tx) = log_tx {
                            let _ = tx
                                .send(
                                    "Failed to parse chain from node; using local chain"
                                        .to_string(),
                                )
                                .await;
                        }
                        Blockchain::load_from_file("blockchain.json")
                            .unwrap_or_else(|_| Blockchain::new())
                    }
                } else {
                    if let Some(ref tx) = log_tx {
                        let _ = tx
                            .send("Failed to read chain from node; using local chain".to_string())
                            .await;
                    }
                    Blockchain::load_from_file("blockchain.json")
                        .unwrap_or_else(|_| Blockchain::new())
                }
            } else {
                if let Some(ref tx) = log_tx {
                    let _ = tx
                        .send("Failed to request chain from node; using local chain".to_string())
                        .await;
                }
                Blockchain::load_from_file("blockchain.json").unwrap_or_else(|_| Blockchain::new())
            }
        }
        Err(_) => {
            if let Some(ref tx) = log_tx {
                let _ = tx
                    .send(format!(
                        "Could not connect to node {} - using local chain",
                        node_addr
                    ))
                    .await;
            } else {
                eprintln!(
                    "Could not connect to node {} - using local chain",
                    node_addr
                );
            }
            Blockchain::load_from_file("blockchain.json").unwrap_or_else(|_| Blockchain::new())
        }
    };

    let blockchain = Arc::new(Mutex::new(blockchain));
    let latest_block: Arc<Mutex<Option<Block>>> = Arc::new(Mutex::new(None));
    // Shared mempool (kept in sync with node via poller)
    let mempool_shared: Arc<Mutex<Vec<crate::blockchain::Transaction>>> =
        Arc::new(Mutex::new(Vec::new()));
    let attempts = Arc::new(AtomicU64::new(0));
    let mined = Arc::new(AtomicU64::new(0));

    let (block_tx, mut block_rx) = mpsc::channel::<Block>(threads * 2);
    let (share_tx, mut share_rx) = mpsc::channel::<(String, u32, u64, Block)>(threads * 2);

    // Lightweight std channels to buffer worker->submitter communication without blocking workers
    let (block_sync_tx, block_sync_rx) = std::sync::mpsc::channel::<Block>();
    let (_share_sync_tx, share_sync_rx) = std::sync::mpsc::channel::<(String, u32, u64, Block)>();
    // Stats tracking
    let attempts_history: Arc<Mutex<VecDeque<u64>>> = Arc::new(Mutex::new(VecDeque::new()));
    let accepted = Arc::new(AtomicU64::new(0));
    let rejected = Arc::new(AtomicU64::new(0));
    // Atomic version counter used to abort mining on template changes (prev_hash updates)
    let chain_version = Arc::new(std::sync::atomic::AtomicU64::new(0));
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
    let latest_block_submitter = latest_block.clone();
    let chain_version_submitter = chain_version.clone();
    let submitter_handle = tokio::spawn(async move {
        while let Some(block) = block_rx.recv().await {
            // Check local latest template before submitting. If our local latest doesn't
            // match the block's prev_hash, the block is stale — notify workers and skip submit.
            {
                let local_latest_opt = latest_block_submitter
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|b| b.hash.clone());
                if let Some(local_latest) = local_latest_opt {
                    if local_latest != block.prev_hash {
                        chain_version_submitter.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                }
            }

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
                // Diagnostic: log the exact block JSON we're about to submit so
                // we can verify coinbase/tx contents in case the node later
                // shows empty transactions.
                if let Some(ref tx) = log_tx_clone1 {
                    let _ = tx
                        .send(format!("[miner] submitting block JSON: {}", block_json))
                        .await;
                } else {
                    eprintln!("[miner] submitting block JSON: {}", block_json);
                }
                writer
                    .write_all(format!("{}\n", block_json).as_bytes())
                    .await?;

                let mut response = String::new();
                reader.read_line(&mut response).await?;
                let response = response.trim();

                if response == "ok" {
                    accepted_clone1.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone1 {
                        let _ = tx
                            .send(format!(
                                "Block accepted! Index={} Hash={}",
                                block.index, block.hash
                            ))
                            .await;
                    }
                    {
                        let mut latest_block_guard = latest_block_submitter.lock().unwrap();
                        *latest_block_guard = Some(block.clone());
                        chain_version_submitter.fetch_add(1, Ordering::Relaxed); // chill bro relax
                    }
                    // Remove included transactions from local mempool (except coinbase)
                    {
                        let mut mp = mempool_for_submitter.lock().unwrap();
                        mp.retain(|t| {
                            // keep txs that are NOT in the block (match by signature)
                            !block
                                .transactions
                                .iter()
                                .any(|bt| bt.signature == t.signature)
                        });
                    }
                } else {
                    rejected_clone1.fetch_add(1, Ordering::Relaxed);
                    // If node rejected due to PrevHash mismatch, log local view to help debugging
                    if response.contains("PrevHash")
                        || response.contains("prev_hash")
                        || response.contains("PrevHash mismatch")
                    {
                        let local_latest = latest_block_submitter
                            .lock()
                            .unwrap()
                            .as_ref()
                            .map(|b| b.hash.clone())
                            .unwrap_or_else(|| "<none>".to_string());
                        let cur_ver = chain_version_submitter.load(Ordering::Relaxed);
                        if let Some(ref lt) = log_tx_clone1 {
                            let _ = lt.send(format!("[miner] submitter: node rejected block: {} | submitted.prev_hash={} | local_latest={} | chain_version={}", response, block.prev_hash, local_latest, cur_ver)).await;
                        }
                    }
                    if let Some(ref tx) = log_tx_clone1 {
                        let _ = tx.send(format!("Node rejected block: {}", response)).await;
                    } else {
                        eprintln!("Node rejected block: {}", response);
                    }
                }
            } else {
                rejected_clone1.fetch_add(1, Ordering::Relaxed);
                if let Some(ref tx) = log_tx_clone1 {
                    let _ = tx
                        .send("Failed to connect to node for block submission".to_string())
                        .await;
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
                writer
                    .write_all(format!("{}\n", share_json).as_bytes())
                    .await?;

                let mut response = String::new();
                reader.read_line(&mut response).await?;
                let response = response.trim();

                if response == "ok" {
                    accepted_clone2.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone2 {
                        let _ = tx
                            .send(format!("Share accepted: {} attempts", attempts_val))
                            .await;
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
                    let _ = tx
                        .send("Failed to connect to node for share submission".to_string())
                        .await;
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
                            if let Ok(vec_tx) = serde_json::from_str::<
                                Vec<crate::blockchain::Transaction>,
                            >(memline.trim())
                            {
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
    // Capture wallet address and node addr for reporting to the daemon
    let wallet_addr_for_stats = wallet.address.clone();
    let node_addr_for_stats = node_addr.to_string();

    let stats_handle = if let Some(tx) = stats_tx {
        let attempts = attempts.clone();
        let mined = mined.clone();
        let attempts_history = attempts_history.clone();
        let accepted = accepted.clone();
        let rejected = rejected.clone();
        let start_time = start_time.clone();
        let log_tx = log_tx.clone();
        let wallet_addr_for_stats = wallet_addr_for_stats.clone();
        let node_addr_for_stats = node_addr_for_stats.clone();
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let mut zero_streak: u32 = 0;
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

                // Compute averages using the actual number of available samples
                let samples_min = std::cmp::min(60, history_clone.len());
                let samples_hour = std::cmp::min(3600, history_clone.len());

                let sum_min: u64 = history_clone.iter().rev().take(samples_min).sum();
                let sum_hour: u64 = history_clone.iter().rev().take(samples_hour).sum();
                let sum_day: u64 = history_clone.iter().sum();

                let avg_min = if samples_min > 0 {
                    sum_min as f64 / samples_min as f64
                } else {
                    0.0
                };
                let avg_hour = if samples_hour > 0 {
                    sum_hour as f64 / samples_hour as f64
                } else {
                    0.0
                };
                let avg_day = if history_clone.len() > 0 {
                    sum_day as f64 / history_clone.len() as f64
                } else {
                    0.0
                };

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
                // Send stats to UI
                let _ = tx.send(stats).await;

                // Also report summary stats to the daemon so the web UI / node
                // can aggregate per-wallet and network hashrates.
                // We only send a compact payload (wallet + total_hps + sols).
                let report = serde_json::json!({
                    "wallet": wallet_addr_for_stats,
                    "hashrate": h,
                    "sols": sols,
                    "timestamp": chrono::Utc::now().timestamp()
                });
                let report_str = report.to_string();
                // Best-effort: try to connect and send the report; ignore errors
                if let Ok(mut stream) = TcpStream::connect(&node_addr_for_stats).await {
                    let (reader, mut writer) = stream.split();
                    // skip greeting
                    let mut _tmp = String::new();
                    let mut reader = BufReader::new(reader);
                    let _ = reader.read_line(&mut _tmp).await;
                    let _ = writer.write_all(b"updatestats\n").await;
                    let _ = writer
                        .write_all(format!("{}\n", report_str).as_bytes())
                        .await;
                }

                // Emit occasional debug log when attempts are zero to help diagnose
                // why hashrate may show 0 H/s. Send logs at most once every 5 seconds
                // while the zero condition persists to avoid spamming the UI.
                if h == 0 {
                    zero_streak = zero_streak.saturating_add(1);
                    if zero_streak % 5 == 0 {
                        if let Some(ref lt) = log_tx {
                            let _ = lt.send(format!("DEBUG: swapped attempts = {}", h)).await;
                        }
                    }
                } else {
                    zero_streak = 0;
                }
            }
        }))
    } else {
        None
    };

    // Mining workers as dedicated OS threads
    let mut worker_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    for _id in 0..threads {
        let wallet_address = wallet.address.clone();
        let wallet_pub_key = wallet.pub_key.clone();
        let wallet_priv_key = wallet.priv_key.clone();
        let blockchain = blockchain.clone();
        let mempool_shared = mempool_shared.clone();
        let attempts = attempts.clone();
        let block_sync_tx = block_sync_tx.clone();
        let shutdown_flag = shutdown_flag.clone();
        let latest_block_worker = latest_block.clone();
        let log_tx_worker = log_tx.clone();
        let chain_version_worker = chain_version.clone();
        let wallet_pub_key = wallet_pub_key.clone();
        let wallet_priv_key = wallet_priv_key.clone();
        let handle = std::thread::spawn(move || {
            loop {
                if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                // Get prev_block from latest_block_worker, fallback to local chain with safety check
                let prev_block = {
                    if let Some(ref lb) = *latest_block_worker.lock().unwrap() {
                        lb.clone()
                    } else {
                        let bc = blockchain.lock().unwrap();
                        if let Some(last) = bc.chain.last() {
                            last.clone()
                        } else {
                            // Chain is empty; skip mining until synced
                            std::thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                    }
                };

                // Get difficulty from local chain
                let diff = {
                    let bc = blockchain.lock().unwrap();
                    let dyn_diff = bc.get_dynamic_difficulty();
                    if pool {
                        dyn_diff.saturating_sub(2).max(1)
                    } else {
                        dyn_diff
                    }
                };

                // Get mempool transactions
                let mempool_txs = {
                    let mp = mempool_shared.lock().unwrap();
                    mp.clone()
                };

                
                let mut mempool_with_coinbase: Vec<crate::blockchain::Transaction> = Vec::new();
                // Determine block reward via blockchain policy (internal units)
                let reward_amount: i64 = {
                    let bc = blockchain.lock().unwrap();
                    // next block height is prev_block.index + 1
                    bc.get_block_reward(prev_block.index + 1)
                };
                let mut coinbase_tx = crate::blockchain::Transaction {
                    from: "coinbase".to_string(),
                    pub_key: wallet_pub_key.clone(),
                    to: wallet_address.clone(),
                    amount: reward_amount,
                    signature: String::new(),
                };
                // Sign the coinbase transaction with the miner's private key
                let _ = crate::blockchain::sign_transaction(&mut coinbase_tx, &wallet_priv_key);
                mempool_with_coinbase.push(coinbase_tx);
                // Append existing mempool txs after coinbase
                mempool_with_coinbase.extend(mempool_txs.into_iter());

                // Mine using the cancellable function
                let mut local_attempts = 0u64;
                let block_opt = crate::blockchain::Blockchain::mine_block_with_cancel(
                    &prev_block,
                    mempool_with_coinbase,
                    diff,
                    &mut local_attempts,
                    Some(&*attempts),             // Pass atomic for shared updates
                    Some(&*chain_version_worker), // Abort on chain version change
                );

                // Update attempts
                attempts.fetch_add(local_attempts, Ordering::Relaxed);

                if let Some(block) = block_opt {
                    // Check hash against target
                    let target = crate::blockchain::Blockchain::difficulty_to_target(diff as u64);
                    if block.hash.starts_with(&target) {
                        let _ = block_sync_tx.send(block);
                    }
                } else {
                    // Mining aborted (likely due to chain_version change). Log for diagnostics.
                    let cur_ver = chain_version_worker.load(Ordering::Relaxed);
                    if let Some(ref lt) = log_tx_worker {
                        let _ = lt.try_send(format!(
                            "[miner] worker abort: chain_version={} prev_hash_template={}",
                            cur_ver, prev_block.hash
                        ));
                    }
                }
            }
        });

        worker_handles.push(handle);
    }

    // Background poller: keep local blockchain copy up-to-date by polling node periodically.
    // This reduces wasted work when other miners find blocks and the local template becomes stale.
    let _blockchain_poller = {
        let node_addr = node_addr.to_string();
        let shutdown = shutdown_flag.clone();
        let latest_block_poller = latest_block.clone();
        let chain_version_poller = chain_version.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500)); // More frequent updates
            loop {
                interval.tick().await;
                if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                if let Ok(mut stream) = TcpStream::connect(&node_addr).await {
                    let (r, mut w) = stream.split();
                    let mut r = BufReader::new(r);
                    // Skip greeting
                    let mut tmp = String::new();
                    let _ = r.read_line(&mut tmp).await;
                    // Request latest block
                    if w.write_all(b"getlatest\n").await.is_ok() {
                        let mut block_line = String::new();
                        if r.read_line(&mut block_line).await.is_ok() {
                            if let Ok(block) = serde_json::from_str::<Block>(block_line.trim()) {
                                *latest_block_poller.lock().unwrap() = Some(block);
                                // Notify workers to abort current templates so they don't mine on a stale prev_hash
                                chain_version_poller.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
        })
    };

    // Wait for completion or cancellation
    if blocks_to_mine > 0 {
        while mined.load(Ordering::Relaxed) < blocks_to_mine
            && !shutdown_flag.load(std::sync::atomic::Ordering::Relaxed)
        {
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
