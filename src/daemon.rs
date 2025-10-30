use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::blockchain::Blockchain;
use anyhow::anyhow;

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

pub async fn run_daemon(port: u16, blockchain: Arc<Mutex<Blockchain>>, pm: Arc<PeerManager>, pool: bool) -> anyhow::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Daemon listening on :{}", port);

    let shares: Arc<Mutex<HashMap<String, i64>>> = Arc::new(Mutex::new(HashMap::new()));
    let mempool: Arc<Mutex<Vec<crate::blockchain::Transaction>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        let (socket, _) = listener.accept().await?;
    let blockchain = blockchain.clone();
    let pm = pm.clone();
    let shares = shares.clone();
    let mempool = mempool.clone();
        let pool = pool;

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, blockchain, pm, shares, mempool, pool).await {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    blockchain: Arc<Mutex<Blockchain>>,
    _pm: Arc<PeerManager>,
    _shares: Arc<Mutex<HashMap<String, i64>>>,
    mempool: Arc<Mutex<Vec<crate::blockchain::Transaction>>>,
    _pool: bool,
) -> anyhow::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);

    // Send greeting - get height without holding lock across await
    let height = {
        let bc = blockchain.lock().unwrap();
        bc.chain.len() - 1
    };
    writer.write_all(format!("owonero-daemon height={}\n", height).as_bytes()).await?;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let command = line.trim();
        match command {
            "getchain" => {
                let data = {
                    let bc = blockchain.lock().unwrap();
                    serde_json::to_string(&*bc)?
                };
                writer.write_all(format!("{}\n", data).as_bytes()).await?;
            }
            "getheight" => {
                let height = {
                    let bc = blockchain.lock().unwrap();
                    bc.chain.len() - 1
                };
                writer.write_all(format!("{}\n", height).as_bytes()).await?;
            }
            "submitblock" => {
                // Read next line for block JSON
                line.clear();
                reader.read_line(&mut line).await?;
                let block: crate::blockchain::Block = serde_json::from_str(line.trim())?;

                // Do a verbose validation to provide the miner a clearer rejection reason
                let response = {
                    let mut bc = blockchain.lock().unwrap();
                    let dyn_diff = bc.get_dynamic_difficulty();

                    if let Some(err) = bc.validate_block_verbose(&block, dyn_diff, false) {
                        format!("rejected: {}", err)
                    } else {
                        // validation passed, add block
                        let added = bc.add_block(block, dyn_diff);
                        if added {
                            // Save blockchain
                            if let Err(e) = bc.save_to_file("blockchain.json") {
                                eprintln!("Failed to save blockchain: {}", e);
                            }
                            String::from("ok")
                        } else {
                            String::from("error: failed to add block")
                        }
                    }
                };

                // Send response after lock is released
                writer.write_all(format!("{}\n", response).as_bytes()).await?;
            }
            "getpeers" => {
                let peers = _pm.get_peers();
                let peers_json = serde_json::to_string(&peers)?;
                writer.write_all(format!("{}\n", peers_json).as_bytes()).await?;
            }
            "getlatest" => {
                let response = {
                    let bc = blockchain.lock().unwrap();
                    if let Some(latest) = bc.chain.last() {
                        serde_json::to_string(latest)?
                    } else {
                        serde_json::to_string(&serde_json::Value::Null)?
                    }
                };
                writer.write_all(format!("{}\n", response).as_bytes()).await?;
            }
            "getmempool" => {
                let data = {
                    let mp = mempool.lock().unwrap();
                    serde_json::to_string(&*mp)?
                };
                writer.write_all(format!("{}\n", data).as_bytes()).await?;
            }
            "submittx" => {
                // Read next line for tx JSON
                line.clear();
                reader.read_line(&mut line).await?;
                let tx: crate::blockchain::Transaction = serde_json::from_str(line.trim())?;

                // Verify signature
                let valid = crate::blockchain::verify_transaction_signature(&tx, &tx.pub_key);
                if !valid {
                    writer.write_all(b"rejected: invalid signature\n").await?;
                } else {
                    // add to mempool
                    {
                        let mut mp = mempool.lock().unwrap();
                        mp.push(tx);
                    }
                    writer.write_all(b"ok\n").await?;
                }
            }
            "submitshare" => {
                // Read next line for share JSON
                line.clear();
                reader.read_line(&mut line).await?;

                let v: serde_json::Value = serde_json::from_str(line.trim())?;
                // Extract fields
                let wallet_addr = v.get("wallet").and_then(|s| s.as_str()).unwrap_or("").to_string();
                let attempts_val = v.get("attempts").and_then(|n| n.as_u64()).unwrap_or(0);
                let block_val = v.get("block").cloned().ok_or_else(|| anyhow!("missing block field"))?;
                let block: crate::blockchain::Block = serde_json::from_value(block_val)?;

                let response = {
                    let mut bc = blockchain.lock().unwrap();
                    let dyn_diff = bc.get_dynamic_difficulty();

                    // Basic prev_hash check
                    let last_hash = bc.chain.last().unwrap().hash.clone();
                    if block.prev_hash != last_hash {
                        String::from("rejected: prev_hash_mismatch")
                    } else if bc.validate_block(&block, block.difficulty, false) {
                        // If this share meets full network difficulty, treat as a mined block
                        if block.difficulty >= dyn_diff {
                            let added = bc.add_block(block, dyn_diff);
                            if added {
                                bc.save_to_file("blockchain.json")?;
                                String::from("ok")
                            } else {
                                String::from("error: failed to add block")
                            }
                        } else {
                            // It's a valid share for pool difficulty — record it and acknowledge
                            let mut map = _shares.lock().unwrap();
                            let entry = map.entry(wallet_addr.clone()).or_insert(0);
                            *entry += attempts_val as i64;
                            String::from("ok")
                        }
                    } else {
                        String::from("rejected: invalid share")
                    }
                };

                // Send response after lock is released
                writer.write_all(format!("{}\n", response).as_bytes()).await?;
            }
            _ => {
                writer.write_all(b"unknown command\n").await?;
            }
        }
        line.clear();
    }

    Ok(())
}