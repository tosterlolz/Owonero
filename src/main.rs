mod blockchain;
mod completions;
mod config;
mod daemon;
mod miner;
mod miner_ui;
mod update;
mod wallet;
mod ws_client;

use clap::{Parser, ValueHint};
use colored::Colorize;
use std::sync::Arc;

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

    // Spawn WebSocket daemon
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
    let node_to_use = wallet
        .node_address
        .clone()
        .unwrap_or(config.node_address.clone());

    // Load local chain
    let mut blockchain =
        blockchain::Blockchain::load_from_file(crate::config::get_blockchain_path())?;

    if config.sync_on_startup {
        // Fetch chain via WebSocket
        match crate::ws_client::fetch_chain(&node_to_use).await {
            Ok(new_chain) => {
                if new_chain.chain.len() > blockchain.chain.len() {
                    blockchain = new_chain;
                    let _ =
                        blockchain.save_to_file(crate::config::get_blockchain_path());
                    println!(
                        "Synchronized blockchain from node {}",
                        node_to_use
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: failed to sync blockchain from node {}: {}",
                    node_to_use, e
                );
            }
        }
    }

    let balance = wallet.get_balance(&blockchain);

    println!("{} {}", "Wallet:".blue(), wallet.address);
    // Display balance in human-friendly OWE (1 OWE == 1000 internal units)
    println!(
        "{} {:.3} OWE",
        "Balance:".yellow(),
        (balance as f64) / 1000.0
    );
    println!("{} {}", "Chain height:".cyan(), blockchain.chain.len() - 1);

    Ok(())
}

async fn run_tx_history_mode(config: config::Config) -> anyhow::Result<()> {
    let wallet = crate::wallet::load_or_create_wallet(&config.wallet_path)?;

    // Load local chain
    let mut blockchain =
        blockchain::Blockchain::load_from_file(crate::config::get_blockchain_path())?;

    // Optionally try to sync from the configured node to get up-to-date data
    if config.sync_on_startup {
        let node_addr = wallet
            .node_address
            .clone()
            .unwrap_or(config.node_address.clone());
        
        if let Ok(new_chain) = crate::ws_client::fetch_chain(&node_addr).await {
            if new_chain.chain.len() > blockchain.chain.len() {
                blockchain = new_chain;
            }
        }
    }

    let my_addr = wallet.address.trim().to_lowercase();

    println!("Transaction history for wallet: {}", wallet.address);
    // Try to fetch mempool from node and show pending txs involving this wallet
    if let Some(node_addr) = wallet
        .node_address
        .clone()
        .or(Some(config.node_address.clone()))
    {
        if let Ok(mempool_vec) = crate::ws_client::fetch_mempool(&node_addr).await {
            let mut found = false;
            for tx in mempool_vec.iter() {
                if tx.to.trim().to_lowercase() == my_addr
                    || tx.from.trim().to_lowercase() == my_addr
                {
                    if !found {
                        println!("PENDING transactions in mempool:");
                        found = true;
                    }
                    let dir = if tx.to.trim().to_lowercase() == my_addr {
                        "IN"
                    } else {
                        "OUT"
                    };
                    println!(
                        "{} pending: {} -> {} amount: {} sig={}",
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

    // Submit transaction via WebSocket
    println!("Connecting to node at {}", node_addr);
    println!(
        "Sending tx: from={} to={} amount={} signature_prefix={}",
        &tx.from[..std::cmp::min(8, tx.from.len())],
        &tx.to[..std::cmp::min(8, tx.to.len())],
        (tx.amount as f64) / 1000.0,
        &tx.signature[..std::cmp::min(16, tx.signature.len())]
    );

    match crate::ws_client::submit_tx(&node_addr, &tx).await {
        Ok(status) if status == "ok" => {
            println!("Node response: {}", status);
            
            // Probe mempool to confirm transaction is present
            if let Ok(mempool_vec) = crate::ws_client::fetch_mempool(&node_addr).await {
                let mut found = false;
                for ptx in mempool_vec.iter() {
                    if ptx.signature == tx.signature
                        || (ptx.from == tx.from
                            && ptx.to == tx.to
                            && ptx.amount == tx.amount)
                    {
                        println!(
                            "Probe: transaction is present in node mempool (signature prefix={})",
                            &ptx.signature[..std::cmp::min(16, ptx.signature.len())]
                        );
                        found = true;
                        break;
                    }
                }
                if !found {
                    println!("Probe: transaction NOT found in node mempool");
                }
            }
            Ok(())
        }
        Ok(status) => {
            println!("Node response: {}", status);
            if status.starts_with("rejected") || status.starts_with("error") {
                let valid = crate::blockchain::verify_transaction_signature(&tx, &tx.pub_key);
                println!(
                    "Local signature verification: {}",
                    if valid { "OK" } else { "FAILED" }
                );
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to submit transaction to node: {}", e);
            Err(e)
        }
    }
}
