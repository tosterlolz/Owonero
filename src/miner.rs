use crate::blockchain::{Block, Blockchain};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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

    if let Some(ref tx) = log_tx {
        let _ = tx
            .send(format!(
                "Mining for wallet {} to node {}",
                wallet.address, node_addr
            ))
            .await;
    }

    // Fetch blockchain via WebSocket
    let blockchain: Blockchain = match crate::ws_client::fetch_chain(node_addr).await {
        Ok(bc) => {
            if let Some(ref tx) = log_tx {
                let _ = tx
                    .send(format!(
                        "Connected to node via WebSocket, chain height: {}",
                        bc.chain.len() - 1
                    ))
                    .await;
            }
            bc
        }
        Err(e) => {
            if let Some(ref tx) = log_tx {
                let _ = tx
                    .send(format!(
                        "Failed to fetch chain from node ({}); using local chain",
                        e
                    ))
                    .await;
            } else {
                eprintln!("Could not connect to node {}: {}", node_addr, e);
            }
            Blockchain::load_from_file("blockchain.json").unwrap_or_else(|_| Blockchain::new())
        }
    };

    let blockchain = Arc::new(Mutex::new(blockchain));
    let latest_block: Arc<Mutex<Option<Block>>> = Arc::new(Mutex::new(None));
    let mempool_shared: Arc<Mutex<Vec<crate::blockchain::Transaction>>> =
        Arc::new(Mutex::new(Vec::new()));
    let attempts = Arc::new(AtomicU64::new(0));
    let mined = Arc::new(AtomicU64::new(0));

    let (block_tx, mut block_rx) = mpsc::channel::<Block>(threads * 2);
    let (share_tx, mut share_rx) = mpsc::channel::<(String, u32, u64, Block)>(threads * 2);
    let (block_sync_tx, block_sync_rx) = std::sync::mpsc::channel::<Block>();
    let (_share_sync_tx, share_sync_rx) = std::sync::mpsc::channel::<(String, u32, u64, Block)>();

    let attempts_history: Arc<Mutex<VecDeque<u64>>> = Arc::new(Mutex::new(VecDeque::new()));
    let accepted = Arc::new(AtomicU64::new(0));
    let rejected = Arc::new(AtomicU64::new(0));
    let chain_version = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let start_time = std::time::Instant::now();

    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    if let Some(mut rx) = shutdown_rx {
        let flag = shutdown_flag.clone();
        tokio::spawn(async move {
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

    // Block submitter via WebSocket
    let node_addr_clone = node_addr.to_string();
    let log_tx_clone1 = log_tx.clone();
    let accepted_clone1 = accepted.clone();
    let rejected_clone1 = rejected.clone();
    let mempool_for_submitter = mempool_shared.clone();
    let latest_block_submitter = latest_block.clone();
    let chain_version_submitter = chain_version.clone();
    let submitter_handle = tokio::spawn(async move {
        while let Some(block) = block_rx.recv().await {
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

            match crate::ws_client::submit_block(&node_addr_clone, &block).await {
                Ok(status) if status == "ok" => {
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
                        chain_version_submitter.fetch_add(1, Ordering::Relaxed);
                    }
                    {
                        let mut mp = mempool_for_submitter.lock().unwrap();
                        mp.retain(|t| {
                            !block
                                .transactions
                                .iter()
                                .any(|bt| bt.signature == t.signature)
                        });
                    }
                }
                Ok(status) => {
                    rejected_clone1.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone1 {
                        let _ = tx.send(format!("Node rejected block: {}", status)).await;
                    } else {
                        eprintln!("Node rejected block: {}", status);
                    }
                }
                Err(e) => {
                    rejected_clone1.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone1 {
                        let _ = tx.send(format!("Failed to submit block: {}", e)).await;
                    } else {
                        eprintln!("Failed to submit block: {}", e);
                    }
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    // Share submitter via WebSocket
    let node_addr_clone2 = node_addr.to_string();
    let log_tx_clone2 = log_tx.clone();
    let accepted_clone2 = accepted.clone();
    let rejected_clone2 = rejected.clone();
    let share_submitter_handle = tokio::spawn(async move {
        while let Some((_wallet_addr, _nonce, _attempts_val, block)) = share_rx.recv().await {
            match crate::ws_client::submit_block(&node_addr_clone2, &block).await {
                Ok(status) if status == "ok" => {
                    accepted_clone2.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone2 {
                        let _ = tx.send("Share accepted".to_string()).await;
                    }
                }
                Ok(status) => {
                    rejected_clone2.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone2 {
                        let _ = tx.send(format!("Node rejected share: {}", status)).await;
                    } else {
                        eprintln!("Node rejected share: {}", status);
                    }
                }
                Err(e) => {
                    rejected_clone2.fetch_add(1, Ordering::Relaxed);
                    if let Some(ref tx) = log_tx_clone2 {
                        let _ = tx.send(format!("Failed to submit share: {}", e)).await;
                    }
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    // Mempool poller via WebSocket
    {
        let node_addr = node_addr.to_string();
        let mempool_clone = mempool_shared.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                if let Ok(mempool_vec) = crate::ws_client::fetch_mempool(&node_addr).await {
                    let mut mp = mempool_clone.lock().unwrap();
                    *mp = mempool_vec;
                }
            }
        });
    }

    // Block forwarder
    let _block_forwarder = {
        let block_tx = block_tx.clone();
        std::thread::spawn(move || {
            while let Ok(block) = block_sync_rx.recv() {
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
    let stats_handle = if let Some(stats_tx) = stats_tx {
        let stats_tx = stats_tx.clone();
        let attempts_clone = attempts.clone();
        let accepted_clone = accepted.clone();
        let rejected_clone = rejected.clone();
        let mined_clone = mined.clone();
        let attempts_history_clone = attempts_history.clone();

        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                let total_attempts = attempts_clone.load(Ordering::Relaxed);
                let acc = accepted_clone.load(Ordering::Relaxed);
                let rej = rejected_clone.load(Ordering::Relaxed);
                let mined = mined_clone.load(Ordering::Relaxed);
                let uptime = start_time.elapsed().as_secs();

                let avg_min = {
                    let hist = attempts_history_clone.lock().unwrap();
                    if hist.is_empty() {
                        0.0
                    } else {
                        hist.iter().sum::<u64>() as f64 / hist.len() as f64 / 60.0
                    }
                };
                let total_hps = if uptime > 0 {
                    total_attempts / uptime
                } else {
                    0
                };

                let stats = MinerStats {
                    total_hps,
                    sols: mined,
                    avg_min,
                    avg_hour: 0.0,
                    avg_day: 0.0,
                    threads,
                    mined,
                    attempts: total_attempts,
                    accepted: acc,
                    rejected: rej,
                    uptime,
                    pool_mode: pool,
                };

                let _ = stats_tx.send(stats).await;
            }
        }))
    } else {
        None
    };

    // Mining workers
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
        let chain_version_worker = chain_version.clone();
        let mined = mined.clone();

        let handle = std::thread::spawn(move || {
            loop {
                if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                let prev_block = {
                    if let Some(ref lb) = *latest_block_worker.lock().unwrap() {
                        lb.clone()
                    } else {
                        let bc = blockchain.lock().unwrap();
                        if let Some(last) = bc.chain.last() {
                            last.clone()
                        } else {
                            std::thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                    }
                };

                let diff = {
                    let bc = blockchain.lock().unwrap();
                    let dyn_diff = bc.get_dynamic_difficulty();
                    if pool {
                        dyn_diff.saturating_sub(2).max(1)
                    } else {
                        dyn_diff
                    }
                };

                let mempool_txs = {
                    let mp = mempool_shared.lock().unwrap();
                    mp.clone()
                };

                let mut mempool_with_coinbase: Vec<crate::blockchain::Transaction> = Vec::new();
                let reward_amount: i64 = {
                    let bc = blockchain.lock().unwrap();
                    bc.get_block_reward(prev_block.index + 1)
                };
                let mut coinbase_tx = crate::blockchain::Transaction {
                    from: "coinbase".to_string(),
                    pub_key: wallet_pub_key.clone(),
                    to: wallet_address.clone(),
                    amount: reward_amount,
                    signature: String::new(),
                };
                let _ = crate::blockchain::sign_transaction(&mut coinbase_tx, &wallet_priv_key);
                mempool_with_coinbase.push(coinbase_tx);
                mempool_with_coinbase.extend(mempool_txs.into_iter());

                let mut local_attempts = 0u64;
                let block_opt = crate::blockchain::Blockchain::mine_block_with_cancel(
                    &prev_block,
                    mempool_with_coinbase,
                    diff,
                    &mut local_attempts,
                    Some(&*attempts),
                    Some(&*chain_version_worker),
                );

                attempts.fetch_add(local_attempts, Ordering::Relaxed);

                if let Some(block) = block_opt {
                    let target = crate::blockchain::Blockchain::difficulty_to_target(diff as u64);
                    if block.hash.starts_with(&target) {
                        mined.fetch_add(1, Ordering::Relaxed);
                        let _ = block_sync_tx.send(block);
                    }
                }
            }
        });

        worker_handles.push(handle);
    }

    // Background poller: keep latest block up-to-date via WebSocket
    {
        let node_addr = node_addr.to_string();
        let shutdown = shutdown_flag.clone();
        let latest_block_poller = latest_block.clone();
        let chain_version_poller = chain_version.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop {
                interval.tick().await;
                if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                if let Ok(block) = crate::ws_client::fetch_latest_block(&node_addr).await {
                    *latest_block_poller.lock().unwrap() = Some(block);
                    chain_version_poller.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }

    // Wait for completion or cancellation
    if blocks_to_mine > 0 {
        while mined.load(Ordering::Relaxed) < blocks_to_mine
            && !shutdown_flag.load(std::sync::atomic::Ordering::Relaxed)
        {
            sleep(Duration::from_millis(200)).await;
        }
    } else {
        while !shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
            sleep(Duration::from_millis(200)).await;
        }
    }

    shutdown_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    for handle in worker_handles {
        let _ = handle.join();
    }
    drop(block_tx);
    drop(share_tx);

    submitter_handle.abort();
    share_submitter_handle.abort();
    if let Some(handle) = stats_handle {
        handle.abort();
    }

    Ok(())
}
