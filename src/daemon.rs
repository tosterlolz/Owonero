use crate::blockchain::Blockchain;
use futures::SinkExt;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub struct PeerManager {
    peers: Mutex<Vec<String>>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: Mutex::new(Vec::new()),
        }
    }

    pub fn add_peer(&self, addr: String) {
        let mut peers = self.peers.lock().unwrap();
        if !peers.contains(&addr) {
            peers.push(addr);
        }
    }

    pub fn get_peers(&self) -> Vec<String> {
        self.peers.lock().unwrap().clone()
    }
}

pub async fn run_daemon(
    port: u16,
    blockchain: Arc<Mutex<Blockchain>>,
    pm: Arc<PeerManager>,
    pool: bool,
    standalone: bool,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Daemon listening on :{} (WebSocket)", port);

    let shares: Arc<Mutex<HashMap<String, i64>>> = Arc::new(Mutex::new(HashMap::new()));
    let mempool: Arc<Mutex<Vec<crate::blockchain::Transaction>>> = Arc::new(Mutex::new(Vec::new()));
    let wallet_hashrates: Arc<Mutex<HashMap<String, (f64, u64)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Background cleaner for stale hashrates
    {
        let wallet_hashrates_clean = wallet_hashrates.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp() as u64;
                let mut map = wallet_hashrates_clean.lock().unwrap();
                let stale: Vec<String> = map
                    .iter()
                    .filter_map(|(k, (_hr, ts))| {
                        if now.saturating_sub(*ts) > 10 {
                            Some(k.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                for k in stale {
                    map.remove(&k);
                }
            }
        });
    }

    // Background sync from peers
    let sync_interval_secs = std::env::var("OWONERO_SYNC_INTERVAL")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10);
    let _max_sync_attempts = 3;
    if !standalone {
        let _blockchain_sync = blockchain.clone();
        let pm_sync = pm.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(sync_interval_secs));
            loop {
                interval.tick().await;
                let peers = pm_sync.get_peers();
                if peers.is_empty() {
                    continue;
                }
                // Sync logic remains same as before (uses TCP/WebSocket to fetch chain)
                // For now, we'll skip peer-to-peer sync during WebSocket migration
            }
        });
    }

    loop {
        let accept_res = listener.accept().await;
        let (socket, _) = match accept_res {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Listener accept error: {}", e);
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }
        };

        let blockchain = blockchain.clone();
        let shares = shares.clone();
        let mempool = mempool.clone();
        let wallet_hashrates = wallet_hashrates.clone();
        let pool = pool;
        let pm = if standalone {
            Arc::new(PeerManager::new())
        } else {
            pm.clone()
        };

        tokio::spawn(async move {
            if let Err(e) = handle_websocket_connection(
                socket,
                blockchain,
                pm,
                shares,
                mempool,
                wallet_hashrates,
                pool,
            )
            .await
            {
                eprintln!("WebSocket connection error: {}", e);
            }
        });
    }
}

async fn handle_websocket_connection(
    socket: TcpStream,
    blockchain: Arc<Mutex<Blockchain>>,
    _pm: Arc<PeerManager>,
    _shares: Arc<Mutex<HashMap<String, i64>>>,
    mempool: Arc<Mutex<Vec<crate::blockchain::Transaction>>>,
    wallet_hashrates: Arc<Mutex<HashMap<String, (f64, u64)>>>,
    _pool: bool,
) -> anyhow::Result<()> {
    let mut ws = accept_async(socket).await?;

    // Send greeting
    let height = {
        let bc = blockchain.lock().unwrap();
        bc.chain.last().map(|b| b.index).unwrap_or(0)
    };
    ws.send(Message::Text(format!(
        "{{\"type\":\"greeting\",\"height\":{}}}",
        height
    )))
    .await?;

    while let Some(msg) = ws.next().await {
        let msg = msg?;

        if let Message::Text(text) = msg {
            let response =
                process_command(&text, &blockchain, &mempool, &wallet_hashrates, &_pm).await;
            ws.send(Message::Text(response)).await?;
        } else if let Message::Binary(_) = msg {
            ws.send(Message::Text(
                r#"{"type":"error","message":"binary messages not supported"}"#.to_string(),
            ))
            .await?;
        } else if let Message::Close(_) = msg {
            break;
        }
    }

    Ok(())
}

async fn process_command(
    cmd_text: &str,
    blockchain: &Arc<Mutex<Blockchain>>,
    mempool: &Arc<Mutex<Vec<crate::blockchain::Transaction>>>,
    _wallet_hashrates: &Arc<Mutex<HashMap<String, (f64, u64)>>>,
    pm: &Arc<PeerManager>,
) -> String {
    // Parse as JSON RPC-like command: {"method":"...", "params":{...}}
    match serde_json::from_str::<serde_json::Value>(cmd_text) {
        Ok(json) => {
            let method = json
                .get("method")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown");

            match method {
                "getchain" => {
                    let bc = blockchain.lock().unwrap();
                    match serde_json::to_value(&*bc) {
                        Ok(data) => {
                            serde_json::json!({"type": "response", "method": "getchain", "data": data})
                                .to_string()
                        }
                        Err(_) => serde_json::json!({"type":"error","message":"failed to serialize chain"}).to_string(),
                    }
                }
                "getlatest" => {
                    let bc = blockchain.lock().unwrap();
                    if let Some(latest) = bc.chain.last() {
                        match serde_json::to_value(latest) {
                            Ok(data) => serde_json::json!({"type": "response", "method": "getlatest", "data": data}).to_string(),
                            Err(_) => serde_json::json!({"type":"error","message":"failed to serialize block"}).to_string(),
                        }
                    } else {
                        serde_json::json!({"type": "response", "method": "getlatest", "data": null})
                            .to_string()
                    }
                }
                "getheight" => {
                    let bc = blockchain.lock().unwrap();
                    let height = bc.chain.last().map(|b| b.index).unwrap_or(0);
                    serde_json::json!({"type": "response", "method": "getheight", "height": height})
                        .to_string()
                }
                "getblock" => {
                    let idx = json.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                    let bc = blockchain.lock().unwrap();
                    if idx < bc.chain.len() {
                        match serde_json::to_value(&bc.chain[idx]) {
                            Ok(data) => serde_json::json!({"type": "response", "method": "getblock", "data": data}).to_string(),
                            Err(_) => serde_json::json!({"type":"error","message":"failed to serialize block"}).to_string(),
                        }
                    } else {
                        serde_json::json!({"type": "error", "message": "block not found"})
                            .to_string()
                    }
                }
                "getmempool" => {
                    let mp = mempool.lock().unwrap();
                    match serde_json::to_value(&*mp) {
                        Ok(data) => serde_json::json!({"type": "response", "method": "getmempool", "data": data}).to_string(),
                        Err(_) => serde_json::json!({"type":"error","message":"failed to serialize mempool"}).to_string(),
                    }
                }
                "submittx" => {
                    // Check both top-level and params for backward compatibility
                    let tx_val = json.get("tx").or_else(|| json.get("params").and_then(|p| p.get("tx")));
                    if let Some(tx_val) = tx_val {
                        match serde_json::from_value::<crate::blockchain::Transaction>(
                            tx_val.clone(),
                        ) {
                            Ok(tx) => {
                                let valid =
                                    crate::blockchain::verify_transaction_signature(&tx, &tx.pub_key);
                                if !valid {
                                    return serde_json::json!({"type":"error","message":"rejected: invalid signature"}).to_string();
                                }

                                if tx.amount <= 0 {
                                    return serde_json::json!({"type":"error","message":"rejected: invalid amount"}).to_string();
                                }

                                let balances = {
                                    let bc = blockchain.lock().unwrap();
                                    let mut map: HashMap<String, i64> = HashMap::new();
                                    for b in &bc.chain {
                                        for t in &b.transactions {
                                            let to = t.to.trim().to_lowercase();
                                            if t.from != "coinbase" {
                                                let from = t.from.trim().to_lowercase();
                                                *map.entry(from).or_insert(0) -= t.amount;
                                            }
                                            *map.entry(to).or_insert(0) += t.amount;
                                        }
                                    }
                                    map
                                };

                                let pending_out: i64 = {
                                    let mp = mempool.lock().unwrap();
                                    mp.iter()
                                        .filter(|t| {
                                            t.from
                                                .trim()
                                                .eq_ignore_ascii_case(&tx.from.trim().to_lowercase())
                                        })
                                        .map(|t| t.amount)
                                        .sum()
                                };

                                let from_key = tx.from.trim().to_lowercase();
                                let onchain_bal = balances.get(&from_key).cloned().unwrap_or(0);
                                if from_key != "coinbase" && onchain_bal - pending_out < tx.amount {
                                    return serde_json::json!({"type":"error","message":"rejected: insufficient funds"}).to_string();
                                }

                                {
                                    let mut mp = mempool.lock().unwrap();
                                    mp.push(tx);
                                }
                                serde_json::json!({"type": "response", "method": "submittx", "status": "ok"})
                                    .to_string()
                            }
                            Err(_) => serde_json::json!({"type":"error","message":"failed to parse transaction"}).to_string(),
                        }
                    } else {
                        serde_json::json!({"type":"error","message":"missing tx field"}).to_string()
                    }
                }
                "submitblock" => {
                    // Check both top-level and params for backward compatibility
                    let block_val = json.get("block").or_else(|| json.get("params").and_then(|p| p.get("block")));
                    if let Some(block_val) = block_val {
                        match serde_json::from_value::<crate::blockchain::Block>(block_val.clone())
                        {
                            Ok(block) => {
                                let response = {
                                    let mut bc = blockchain.lock().unwrap();
                                    let dyn_diff = bc.get_dynamic_difficulty();

                                    if let Some(last) = bc.chain.last() {
                                        if block.index <= last.index {
                                            format!(
                                                "rejected: block index {} already exists (current height {})",
                                                block.index, last.index
                                            )
                                        } else if let Some(err) =
                                            bc.validate_block_verbose(&block, dyn_diff, false)
                                        {
                                            format!("rejected: {}", err)
                                        } else {
                                            let added = bc.add_block(block, dyn_diff);
                                            if added {
                                                let _ = bc.save_to_file("blockchain.json");
                                                "ok".to_string()
                                            } else {
                                                "error: failed to add block".to_string()
                                            }
                                        }
                                    } else {
                                        if let Some(err) =
                                            bc.validate_block_verbose(&block, dyn_diff, false)
                                        {
                                            format!("rejected: {}", err)
                                        } else {
                                            let added = bc.add_block(block, dyn_diff);
                                            if added {
                                                let _ = bc.save_to_file("blockchain.json");
                                                "ok".to_string()
                                            } else {
                                                "error: failed to add block".to_string()
                                            }
                                        }
                                    }
                                };
                                serde_json::json!({"type": "response", "method": "submitblock", "status": response})
                                    .to_string()
                            }
                            Err(_) => serde_json::json!({"type":"error","message":"failed to parse block"}).to_string(),
                        }
                    } else {
                        serde_json::json!({"type":"error","message":"missing block field"})
                            .to_string()
                    }
                }
                "getpeers" => {
                    let peers = pm.get_peers();
                    match serde_json::to_value(peers) {
                        Ok(data) => serde_json::json!({"type": "response", "method": "getpeers", "data": data}).to_string(),
                        Err(_) => serde_json::json!({"type":"error","message":"failed to serialize peers"}).to_string(),
                    }
                }
                _ => serde_json::json!({"type":"error","message":"unknown method"}).to_string(),
            }
        }
        Err(_) => serde_json::json!({"type":"error","message":"invalid JSON"}).to_string(),
    }
}
