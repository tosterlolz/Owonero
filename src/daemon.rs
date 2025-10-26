use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::blockchain::Blockchain;

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

    loop {
        let (socket, _) = listener.accept().await?;
        let blockchain = blockchain.clone();
        let pm = pm.clone();
        let shares = shares.clone();
        let pool = pool;

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, blockchain, pm, shares, pool).await {
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

                let result = {
                    let mut bc = blockchain.lock().unwrap();
                    let dyn_diff = bc.get_dynamic_difficulty();
                    bc.add_block(block, dyn_diff)
                };

                if result {
                    // Save blockchain
                    {
                        let bc = blockchain.lock().unwrap();
                        bc.save_to_file("blockchain.json")?;
                    }
                    writer.write_all(b"ok\n").await?;
                } else {
                    writer.write_all(b"error: block invalid\n").await?;
                }
            }
            "getpeers" => {
                let peers = _pm.get_peers();
                let peers_json = serde_json::to_string(&peers)?;
                writer.write_all(format!("{}\n", peers_json).as_bytes()).await?;
            }
            _ => {
                writer.write_all(b"unknown command\n").await?;
            }
        }
        line.clear();
    }

    Ok(())
}