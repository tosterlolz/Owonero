mod blockchain;
mod completions;
mod config;
mod daemon;
mod miner;
mod miner_ui;
mod update;
mod wallet;
// wallet_ui removed: CLI-only sending is used

use clap::{Parser, ValueHint};
use colored::Colorize;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::miner_ui::run_miner_ui;
use serde_json;

const ASCII_LOGO: &str = r#"⡰⠁⠀⠀⢀⢔⣔⣤⠐⠒⠒⠒⠒⠠⠄⢀⠀⠐⢀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⡐⢀⣾⣷⠪⠑⠛⠛⠛⠂⠠⠶⢶⣿⣦⡀⠀⠈⢐⢠⣑⠤⣀⠀⠀⠀
⠀⢀⡜⠀⢸⠟⢁⠔⠁⠀⠀⠀⠀⠀⠀⠀⠉⠻⢷⠀⠀⠀⡦⢹⣷⣄⠀⢀⣀⡀
⠀⠸⠀⠠⠂⡰⠁⡜⠀⠀⠀⠀⠀⠀⠀⡀⠀⠀⠈⠇⠀⠀⢡⠙⢿⣿⣾⣿⣿⠃
⠀⠀⠠⠁⠰⠁⢠⢀⠀⠀⡄⠀⠀⠀⠀⠁⠀⠀⠀⠀⠀⠀⠀⢢⠀⢉⡻⣿⣇⠀
⠀⠠⠁⠀⡇⠀⡀⣼⠀⢰⡇⠀⠀⠀⠀⢸⠀⠀⠀⠀⠀⠀⠀⢸⣧⡈⡘⣷⠟⠀   ______          ________ 
⠀⠀⠀⠈⠀⠀⣧⢹⣀⡮⡇⠀⠀⠀⢸⢸⡄⠀⠀⠀⠀⠀⠀⢸⠈⠈⠲⠇⠀⠀  / __ \ \        / /  ____|
⠀⢰⠀⢸⢰⢰⠘⠀⢶⠀⢷⡄⠈⠁⡚⡾⢧⢠⡀⢠⠀⠀⠀⢸⡀⠀⠀⠰⠀  | |  | \ \  /\  / /| |__
⣧⠈⡄⠈⣿⡜⢱⣶⣦⠀⠀⢠⠆⠀⣁⣀⠘⢸⠀⢸⠀⡄⠀⠀⡆⠀⠠⡀⠃  | |  | |\ \/  \/ / |  __| 
⢻⣷⡡⢣⣿⠃⠘⠿⠏⠀⠀⠀⠂⠀⣿⣿⣿⡇⠀⡀⣰⡗⠄⡀⠰⠀⠀⠀⠀  | |__| | \  /\  /  | |____
⠀⠙⢿⣜⢻⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠋⢁⢡⠀⡷⣿⠁⠈⠋⠢⢇⠀⡀⠀  \_____/   \/  \/   |______|
⠀⠀⠈⢻⠀⡆⠀⠀⠀⠀⠀⠀⠀⠀⠐⠆⡘⡇⠀⣼⣿⡇⢀⠀⠀⠀⢱⠁⠀ 			  V.%s ≧◡≦
⠐⢦⣀⠸⡀⢸⣦⣄⡀⠒⠄⠀⠀⠀⢀⣀⣴⠀⣸⣿⣿⠁⣼⢦⠀⠀⠘⠀		
⠀⠀⢎⠳⣇⠀⢿⣿⣿⣶⣤⡶⣾⠿⠋⣁⡆⡰⢿⣿⣿⡜⢣⠀⢆⡄⠇⠀
⠀⠀⠈⡄⠈⢦⡘⡇⠟⢿⠙⡿⢀⠐⠁⢰⡜⠀⠀⠙⢿⡇⠀⡆⠈⡟⠀⠀      
"#;

#[derive(Parser)]
#[command(name = "owonero")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Owonero cryptocurrency miner/daemon")]
struct Cli {
    /// Run daemon in standalone mode (no peers)
    #[arg(short = 's', long)]
    standalone: bool,
    /// Run as daemon
    #[arg(short, long)]
    daemon: bool,

    // TUI removed; use CLI flags (--send --to --amount) for sending

    /// Show miner TUI during mining
    #[arg(long)]
    miner_ui: bool,

    /// Daemon port
    #[arg(short = 'p', long, default_value = "6969", value_hint = ValueHint::Other)]
    // Hint for port numbers
    port: u16,

    /// Web stats server port
    #[arg(long, default_value = "6767", value_hint = ValueHint::Other)]
    // Hint for port numbers
    web_port: u16,

    /// Wallet file path
    #[arg(short = 'w', long, default_value_t = crate::config::get_wallet_path().to_string_lossy().to_string(), value_hint = ValueHint::FilePath)]
    wallet_path: String,

    /// Mine blocks
    #[arg(short = 'm', long)]
    mine: bool,

    /// How many blocks to mine (0 = forever)
    #[arg(short = 'b', long, default_value = "0", value_hint = ValueHint::Other)]
    // Numeric hint
    blocks: u64,

    /// Enable pool mining mode
    #[arg(long)]
    pool: bool,

    /// CPU intensity percent (0-100)
    #[arg(short = 'i', long, default_value = "100", value_hint = ValueHint::Other)]
    // Numeric hint
    intensity: u8,

    /// Node address (host:port)
    #[arg(short = 'n', long, default_value = "owonero.yabai.buzz:6969", value_hint = ValueHint::Hostname)]
    // Hostname/port completion
    node_addr: String,

    /// Number of mining threads
    #[arg(short = 't', long, default_value = "4", value_hint = ValueHint::Other)]
    // Numeric hint
    threads: usize,

    #[arg(long = "install-completions", value_name = "SHELL")]
    pub install_completions: Option<String>,

    /// Comma-separated list of peer addresses
    #[arg(long, value_hint = ValueHint::Hostname)] // Hostname completion for peers
    peers: Option<String>,

    /// Skip automatic update check
    #[arg(long)]
    no_update: bool,

    /// Don't initialize blockchain, rely on syncing
    #[arg(long)]
    no_init: bool,

    /// Send OWE to another wallet
    #[arg(long)]
    send: bool,

    /// Print transaction history for the configured wallet
    #[arg(long = "tx-history")]
    tx_history: bool,

    /// Destination address for sending OWE
    #[arg(long, value_hint = ValueHint::Other)]
    // Could be a wallet address; use Other for custom
    to: Option<String>,

    /// Amount to send (can be decimal, e.g. 1.5)
    #[arg(long, value_hint = ValueHint::Other)] // Numeric/decimal hint
    amount: Option<f64>,
}

fn load_and_merge_config(cli: &Cli) -> anyhow::Result<config::Config> {
    // Load config
    let config_path: std::path::PathBuf = dirs::config_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("owonero")
        .join("config.json");
    let mut config = if config_path.exists() {
        config::load_config()?
    } else {
        config::Config::default()
    };

    // Override config with CLI args
    config.node_address = cli.node_addr.clone();
    config.daemon_port = cli.port;
    config.web_port = cli.web_port;
    config.wallet_path = cli.wallet_path.clone();
    config.mining_threads = cli.threads;
    config.peers = cli
        .peers
        .as_ref()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    config.auto_update = !cli.no_update;
    config.sync_on_startup = true;
    config.target_block_time = 30;
    config.mining_intensity = cli.intensity;
    config.pool = cli.pool;

    // Save updated config
    config::save_config(&config)?;

    Ok(config)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle completions installation/printing
    if let Some(shell) = &cli.install_completions {
        if shell == "stdout" {
            completions::print_to_stdout("bash")?; // Default to bash for stdout; adjust as needed
        } else {
            let path = completions::install_user_completion(shell)?;
            println!("Completions installed to: {}", path.display());
        }
        return Ok(()); // Exit early after handling completions
    }

    // Compose version string including short git commit (set by build.rs) and print ASCII logo
    let full_version = format!(
        "v{}=>{}",
        env!("CARGO_PKG_VERSION"),
        option_env!("GIT_HASH_SHORT").unwrap_or("unknown")
    );
    println!("{}", ASCII_LOGO.replace("%s", &full_version).purple());

    let config = load_and_merge_config(&cli)?;

    // Ensure a wallet exists in the config directory. Try to load it; if it
    // doesn't exist or loading fails, create a new wallet and save it so the
    // rest of the program can assume a wallet file is present.
    match config::load_wallet() {
        Ok(_) => {
            // wallet exists or was created by load_wallet()
        }
        Err(e) => {
            eprintln!(
                "Wallet not found or failed to load: {}. Creating a new wallet...",
                e
            );
            match crate::wallet::Wallet::new() {
                Ok(wallet) => {
                    let path = &config.wallet_path;
                    let p = std::path::Path::new(path);
                    if let Some(parent) = p.parent() {
                        if !parent.as_os_str().is_empty() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                    }
                    if let Ok(data) = serde_json::to_string_pretty(&wallet) {
                        if let Err(err) = std::fs::write(path, data) {
                            eprintln!("Failed to write new wallet to {}: {}", path, err);
                        } else {
                            eprintln!("Created new wallet at {}", path);
                        }
                    }
                }
                Err(err) => eprintln!("Failed to generate new wallet: {}", err),
            }
        }
    }

    // Check for updates if enabled
    if config.auto_update {
        if let Err(e) = update::check_for_updates().await {
            eprintln!("{}", format!("Failed to check for updates: {}", e).red());
        }
    }

    println!("{}", format!("OWONERO-RS {}", full_version).green());

    // Route to appropriate command handler
    if cli.daemon {
        run_daemon_mode(cli, config).await
    } else if cli.mine {
        run_mining_mode(cli, config).await
    } else if cli.miner_ui {
        run_miner_ui().await // Note: This calls `run_miner_ui` from miner_ui module, not the unused `run_miner_ui_mode`
    } else if cli.send {
        // CLI send mode: owonero --send --amount <amt> --to <pubkey>
        run_send_mode(cli, config).await
    } else if cli.tx_history {
        run_tx_history_mode(config).await
    } else {
        // Default to wallet info if no mode flag is set
        run_wallet_info_mode(config).await
    }
}

// Blockchain path lives in the config directory, use `config::get_blockchain_path()`.

async fn run_daemon_mode(cli: Cli, config: config::Config) -> anyhow::Result<()> {
    // Load local blockchain from file (daemon is authoritative)
    let loaded_chain = blockchain::Blockchain::load_from_file(crate::config::get_blockchain_path())
        .unwrap_or_else(|_| blockchain::Blockchain::new());
    let blockchain = Arc::new(std::sync::Mutex::new(loaded_chain));
    let pm = Arc::new(daemon::PeerManager::new());

    // Add peers from config
    for peer in &config.peers {
        pm.add_peer(peer.clone());
    }

    let daemon_port = config.daemon_port;
    // web_port and daemon_addr removed since HTTP API is not used

    // Spawn TCP daemon
    let standalone = cli.standalone;
    let daemon_handle = tokio::spawn(async move {
        if let Err(e) =
            daemon::run_daemon(daemon_port, blockchain, pm, config.pool, standalone).await
        {
            eprintln!("Daemon error: {}", e);
        }
    });

    // Wait for daemon to finish or Ctrl+C
    tokio::select! {
        _ = daemon_handle => {},
        _ = tokio::signal::ctrl_c() => {
            println!("Shutting down daemon...");
        }
    }

    Ok(())
}

// TUI removed: use CLI --send or miner_ui instead of the previous TUI mode.

async fn run_mining_mode(cli: Cli, config: config::Config) -> anyhow::Result<()> {
    // Always start mining with UI
    let (stats_tx, stats_rx) = tokio::sync::mpsc::channel(10);
    let (log_tx, log_rx) = tokio::sync::mpsc::channel(100);

    // Create shutdown notifier so UI can request program shutdown
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Start mining in background
    let mining_handle = tokio::spawn(async move {
        if let Err(e) = miner::start_mining(
            &config.node_address,
            cli.blocks,
            config.mining_threads,
            config.pool,
            config.mining_intensity,
            Some(stats_tx),
            Some(log_tx),
            Some(shutdown_rx),
        )
        .await
        {
            eprintln!("Mining error: {}", e);
        }
    });

    // Start UI
    let ui_shutdown_tx = shutdown_tx.clone();
    let ui_handle = tokio::spawn(async move {
        match miner_ui::MinerUI::new() {
            Ok(mut ui) => {
                if let Err(e) = ui.run(stats_rx, log_rx, Some(ui_shutdown_tx)).await {
                    eprintln!("UI error: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to initialize UI: {}", e),
        }
    });

    // Wait for UI to finish or Ctrl+C. When UI exits (user pressed 'q') it will send shutdown;
    // on Ctrl+C we send shutdown and abort background tasks to exit promptly.
    tokio::select! {
        _ = ui_handle => {
            // UI finished normally (user pressed 'q') - request shutdown
            let _ = shutdown_tx.send(true);
            // Abort mining task so the program can exit promptly
            mining_handle.abort();
            let _ = mining_handle.await;
        }
        _ = tokio::signal::ctrl_c() => {
            eprintln!("Received Ctrl+C - shutting down");
            let _ = shutdown_tx.send(true);
            mining_handle.abort();
            let _ = mining_handle.await;
        }
    }

    Ok(())
}

async fn run_wallet_info_mode(config: config::Config) -> anyhow::Result<()> {
    let wallet = crate::wallet::load_or_create_wallet(&config.wallet_path)?;
    // Try to sync the local chain from a configured node. Prefer the node
    // address stored in the wallet (if present), otherwise fall back to
    // the CLI/config node address. This allows wallets to remember which
    // node they primarily communicate with.
    let node_to_use = wallet
        .node_address
        .clone()
        .unwrap_or(config.node_address.clone());

    // Load local chain
    let mut blockchain =
        blockchain::Blockchain::load_from_file(crate::config::get_blockchain_path())?;

    if config.sync_on_startup {
        if let Ok(stream) = tokio::net::TcpStream::connect(&node_to_use).await {
            let (r, mut w) = stream.into_split();
            let mut reader = tokio::io::BufReader::new(r);

            // skip greeting
            let mut greeting = String::new();
            let _ = reader.read_line(&mut greeting).await;
            // Try partial sync: request latest block header then fetch only missing blocks
            if w.write_all(b"getlatest\n").await.is_ok() {
                let mut latest_line = String::new();
                if reader.read_line(&mut latest_line).await.is_ok() {
                    if let Ok(node_latest) = serde_json::from_str::<crate::blockchain::Block>(
                        latest_line.trim(),
                    ) {
                        let local_height = blockchain.chain.last().map(|b| b.index).unwrap_or(0);
                        let node_height = node_latest.index;
                        if node_height > local_height {
                            println!(
                                "Partial sync: fetching blocks {}..{} from {}",
                                local_height + 1,
                                node_height,
                                node_to_use
                            );

                            // Fetch missing blocks one by one using getblock
                            let mut success = true;
                            for idx in (local_height + 1)..=node_height {
                                let cmd = format!("getblock {}\n", idx);
                                if w.write_all(cmd.as_bytes()).await.is_err() {
                                    eprintln!("Failed to request block {} from node", idx);
                                    success = false;
                                    break;
                                }
                                let mut block_line = String::new();
                                if reader.read_line(&mut block_line).await.is_err() {
                                    eprintln!("Failed to read block {} from node", idx);
                                    success = false;
                                    break;
                                }

                                let block_trim = block_line.trim();
                                if block_trim.starts_with("error:") || block_trim.is_empty() {
                                    eprintln!(
                                        "Node did not return block {} (response: {}), falling back to full chain",
                                        idx, block_trim
                                    );
                                    success = false;
                                    break;
                                }

                                match serde_json::from_str::<crate::blockchain::Block>(block_trim) {
                                    Ok(block) => {
                                        // compute dynamic difficulty from current chain state
                                        let dyn_diff = blockchain.get_dynamic_difficulty();
                                        if blockchain.validate_block(&block, dyn_diff, false) {
                                            blockchain.chain.push(block);
                                        } else {
                                            eprintln!(
                                                "Received invalid block {} during partial sync; aborting",
                                                idx
                                            );
                                            success = false;
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Failed to parse block {} from node: {}",
                                            idx, e
                                        );
                                        success = false;
                                        break;
                                    }
                                }
                            }

                            if success {
                                let _ = blockchain.save_to_file(crate::config::get_blockchain_path());
                                println!("Partial sync complete: new height {}", blockchain.chain.last().map(|b| b.index).unwrap_or(0));
                            } else {
                                // Fallback: request full chain if partial sync failed
                                eprintln!("Partial sync failed; attempting full chain fetch as fallback");
                                // Rewind the reader/writer by asking getchain
                                if w.write_all(b"getchain\n").await.is_ok() {
                                    let mut chain_line = String::new();
                                    if reader.read_line(&mut chain_line).await.is_ok() {
                                        if let Ok(new_chain) = serde_json::from_str::<
                                            crate::blockchain::Blockchain,
                                        >(chain_line.trim())
                                        {
                                            if new_chain.chain.len() > blockchain.chain.len() {
                                                blockchain = new_chain;
                                                let _ = blockchain.save_to_file(
                                                    crate::config::get_blockchain_path(),
                                                );
                                                println!(
                                                    "Synchronized blockchain from node {}",
                                                    node_to_use
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Couldn't parse latest; fall back to full chain fetch
                        eprintln!("Failed to parse latest block from node, requesting full chain");
                        if w.write_all(b"getchain\n").await.is_ok() {
                            let mut chain_line = String::new();
                            if reader.read_line(&mut chain_line).await.is_ok() {
                                if let Ok(new_chain) =
                                    serde_json::from_str::<crate::blockchain::Blockchain>(chain_line.trim())
                                {
                                    if new_chain.chain.len() > blockchain.chain.len() {
                                        blockchain = new_chain;
                                        let _ = blockchain.save_to_file(
                                            crate::config::get_blockchain_path(),
                                        );
                                        println!(
                                            "Synchronized blockchain from node {}",
                                            node_to_use
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            eprintln!(
                "Warning: failed to connect to node {} to sync blockchain",
                node_to_use
            );
        }
    }

    let balance = wallet.get_balance(&blockchain);

    println!("{} {}", "Wallet:".blue(), wallet.address);
    println!("{} {}", "Balance:".yellow(), balance);
    println!("{} {}", "Chain height:".cyan(), blockchain.chain.len() - 1);

    // Diagnostic: list any transactions that involve this wallet so the user
    // can see why the balance is what it is. This helps when blocks appear
    // to be mined but the wallet still shows zero balance.
    // println!("Diagnostics: scanning chain for transactions involving this wallet...");

    Ok(())
}

async fn run_tx_history_mode(config: config::Config) -> anyhow::Result<()> {
    let wallet = crate::wallet::load_or_create_wallet(&config.wallet_path)?;

    // Load local chain
    let mut blockchain =
        blockchain::Blockchain::load_from_file(crate::config::get_blockchain_path())?;

    // Optionally try to sync from the configured node to get up-to-date data
    if config.sync_on_startup {
        if let Ok(stream) = tokio::net::TcpStream::connect(&wallet.node_address.clone().unwrap_or(config.node_address.clone())).await {
            let (r, mut w) = stream.into_split();
            let mut reader = tokio::io::BufReader::new(r);

            // skip greeting
            let mut greeting = String::new();
            let _ = reader.read_line(&mut greeting).await;

            // request chain
            if w.write_all(b"getchain\n").await.is_ok() {
                let mut chain_line = String::new();
                if reader.read_line(&mut chain_line).await.is_ok() {
                    if let Ok(new_chain) = serde_json::from_str::<crate::blockchain::Blockchain>(chain_line.trim()) {
                        if new_chain.chain.len() > blockchain.chain.len() {
                            blockchain = new_chain;
                        }
                    }
                }
            }
        }
    }

    let my_addr = wallet.address.trim().to_lowercase();

    println!("Transaction history for wallet: {}", wallet.address);
    // Try to fetch mempool from node and show pending txs involving this wallet
    if let Some(mut node_addr) = wallet.node_address.clone().or(Some(config.node_address.clone())) {
        if node_addr.starts_with("http://") {
            node_addr = node_addr.trim_start_matches("http://").to_string();
        } else if node_addr.starts_with("https://") {
            node_addr = node_addr.trim_start_matches("https://").to_string();
        }
        if let Some(pos) = node_addr.find('/') {
            node_addr = node_addr[..pos].to_string();
        }
        if let Ok(stream) = tokio::net::TcpStream::connect(&node_addr).await {
            let (r, mut w) = stream.into_split();
            let mut reader = tokio::io::BufReader::new(r);
            let mut greeting = String::new();
            let _ = reader.read_line(&mut greeting).await;
            if w.write_all(b"getmempool\n").await.is_ok() {
                let mut mempool_line = String::new();
                if reader.read_line(&mut mempool_line).await.is_ok() {
                    if let Ok(mempool_vec) = serde_json::from_str::<Vec<crate::blockchain::Transaction>>(mempool_line.trim()) {
                        let mut found = false;
                        for tx in mempool_vec.iter() {
                            if tx.to.trim().to_lowercase() == my_addr || tx.from.trim().to_lowercase() == my_addr {
                                if !found {
                                    println!("PENDING transactions in mempool:");
                                    found = true;
                                }
                                    let dir = if tx.to.trim().to_lowercase() == my_addr { "IN" } else { "OUT" };
                                    println!("{} pending: {} -> {} amount: {} sig={}",
                                    dir,
                                    &tx.from[..std::cmp::min(8, tx.from.len())],
                                    &tx.to[..std::cmp::min(8, tx.to.len())],
                                        (tx.amount as f64) / 1000.0,
                                    &tx.signature[..std::cmp::min(16, tx.signature.len())]
                                );
                            }
                        }
                        if found {
                            println!("----");
                        }
                    }
                }
            }
        }
    }
    for block in &blockchain.chain {
        for tx in &block.transactions {
            let tx_to = tx.to.trim().to_lowercase();
            let tx_from = tx.from.trim().to_lowercase();
            if tx_to == my_addr || tx_from == my_addr {
                let direction = if tx_to == my_addr { "IN" } else { "OUT" };
                println!(
                    "{} [#{:>5}] {} {} -> {}  amount: {}",
                    direction,
                    block.index,
                    block.timestamp,
                    &tx.from[..std::cmp::min(8, tx.from.len())],
                    &tx.to[..std::cmp::min(8, tx.to.len())],
                    (tx.amount as f64) / 1000.0
                );
            }
        }
    }

    Ok(())
}

async fn run_send_mode(cli: Cli, config: config::Config) -> anyhow::Result<()> {
    if !cli.send {
        return Err(anyhow::anyhow!("send flag not set"));
    }

    let to = match cli.to {
        Some(t) if !t.is_empty() => t,
        _ => return Err(anyhow::anyhow!("missing --to argument for send")),
    };

    let amount_f = cli.amount.unwrap_or(0.0);
    if amount_f <= 0.0 {
        return Err(anyhow::anyhow!("amount must be > 0"));
    }

    // Convert decimal amount to internal atomic units (milli-OWE)
    // e.g. 1.234 OWE -> 1234 units
    let amount_units = (amount_f * 1000.0).round() as i64;
    if amount_units <= 0 {
        return Err(anyhow::anyhow!("amount too small after conversion"));
    }

    // Load wallet and create signed transaction
    let wallet = config::load_wallet()?;
    let tx = wallet.create_signed_transaction(&to, amount_units)?;

    // Normalize node address (allow passing http://host:port or host:port)
    let mut node_addr = config.node_address.trim().to_string();
    if node_addr.starts_with("http://") {
        node_addr = node_addr.trim_start_matches("http://").to_string();
    } else if node_addr.starts_with("https://") {
        node_addr = node_addr.trim_start_matches("https://").to_string();
    }
    // If a path was included, strip it (keep host:port)
    if let Some(pos) = node_addr.find('/') {
        node_addr = node_addr[..pos].to_string();
    }

    // Connect to node and submit transaction
    println!("Connecting to node at {}", node_addr);
    let stream = tokio::net::TcpStream::connect(&node_addr).await?;
    let (r, mut w) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(r);

    // Skip greeting line if present
    let mut greeting = String::new();
    let _ = reader.read_line(&mut greeting).await;

    w.write_all(b"submittx\n").await?;
    let tx_json = serde_json::to_string(&tx)?;
    // Debug: print tx summary (not private key)
    println!("Sending tx: from={} to={} amount={} signature_prefix={}",
        &tx.from[..std::cmp::min(8, tx.from.len())],
        &tx.to[..std::cmp::min(8, tx.to.len())],
        (tx.amount as f64) / 1000.0,
        &tx.signature[..std::cmp::min(16, tx.signature.len())]
    );
    w.write_all(format!("{}\n", tx_json).as_bytes()).await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let resp = response.trim().to_string();
    if resp == "unknown command" {
        println!("Node does not recognize 'submittx' - trying peers from node's peer list...");

        // Try peers returned by node.getpeers()
        if let Ok(peer_stream) = tokio::net::TcpStream::connect(&config.node_address).await {
            let (pr, mut pw) = peer_stream.into_split();
            let mut peer_reader = tokio::io::BufReader::new(pr);
            // skip greeting
            let mut tmp = String::new();
            let _ = peer_reader.read_line(&mut tmp).await;
            // request peers
            pw.write_all(b"getpeers\n").await?;
            let mut peers_line = String::new();
            if peer_reader.read_line(&mut peers_line).await.is_ok() {
                if let Ok(peers_vec) = serde_json::from_str::<Vec<String>>(peers_line.trim()) {
                    for peer in peers_vec {
                        if peer.is_empty() {
                            continue;
                        }
                        println!("Trying peer {}...", peer);
                        if let Ok(s) = tokio::net::TcpStream::connect(&peer).await {
                            let (r2, mut w2) = s.into_split();
                            let mut rbuf = tokio::io::BufReader::new(r2);
                            let mut greet = String::new();
                            let _ = rbuf.read_line(&mut greet).await;
                            w2.write_all(b"submittx\n").await?;
                            w2.write_all(format!("{}\n", tx_json).as_bytes()).await?;
                            let mut peer_resp = String::new();
                            if rbuf.read_line(&mut peer_resp).await.is_ok() {
                                let peer_resp = peer_resp.trim();
                                println!("Peer {} responded: {}", peer, peer_resp);
                                if peer_resp == "ok" {
                                    println!("Transaction accepted by peer {}", peer);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }

        println!(
            "Failed to submit transaction: no peers accepted submittx. Either the node is outdated or no peers support transaction submission."
        );
        return Ok(());
    } else {
        println!("Node response: {}", resp);
        // If node rejected due to signature, print local verification result for debugging
        if resp.starts_with("rejected") || resp.starts_with("error") {
            let valid = crate::blockchain::verify_transaction_signature(&tx, &tx.pub_key);
            println!("Local signature verification: {}", if valid { "OK" } else { "FAILED" });
        }

        // Immediately probe the node's mempool to confirm the tx is present (helpful when "ok" is returned)
        let probe_addr = node_addr.clone();
        if let Ok(s) = tokio::net::TcpStream::connect(&probe_addr).await {
            let (r, mut w2) = s.into_split();
            let mut rbuf = tokio::io::BufReader::new(r);
            // skip greeting
            let mut g = String::new();
            let _ = rbuf.read_line(&mut g).await;
            if w2.write_all(b"getmempool\n").await.is_ok() {
                let mut memline = String::new();
                if rbuf.read_line(&mut memline).await.is_ok() {
                    if let Ok(mempool_vec) = serde_json::from_str::<Vec<crate::blockchain::Transaction>>(memline.trim()) {
                        let mut found = false;
                        for ptx in mempool_vec.iter() {
                            if ptx.signature == tx.signature || (ptx.from == tx.from && ptx.to == tx.to && ptx.amount == tx.amount) {
                                println!("Probe: transaction is present in node mempool (signature prefix={})", &ptx.signature[..std::cmp::min(16, ptx.signature.len())]);
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            println!("Probe: transaction NOT found in node mempool");
                        }
                    } else {
                        println!("Probe: failed to parse mempool response");
                    }
                }
            }
        }
    }

    Ok(())
}
