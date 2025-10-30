mod blockchain;
mod wallet;
mod miner;
mod daemon;
mod config;
mod update;
mod completions;
mod miner_ui;

use clap::{Parser, ValueHint};
use std::sync::Arc;
use colored::Colorize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::miner_ui::run_miner_ui;

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
    /// Run as daemon
    #[arg(short, long)]
    daemon: bool,

    /// Run wallet in TUI mode
    #[arg(long)]
    tui: bool,

    /// Show miner TUI during mining
    #[arg(long)]
    miner_ui: bool,

    /// Daemon port
    #[arg(short = 'p', long, default_value = "6969", value_hint = ValueHint::Other)]  // Hint for port numbers
    port: u16,

    /// Web stats server port
    #[arg(long, default_value = "6767", value_hint = ValueHint::Other)]  // Hint for port numbers
    web_port: u16,

    /// Wallet file path
    #[arg(short = 'w', long, default_value = "wallet.json", value_hint = ValueHint::FilePath)]  // File path completion
    wallet_path: String,

    /// Mine blocks
    #[arg(short = 'm', long)]
    mine: bool,

    /// How many blocks to mine (0 = forever)
    #[arg(short = 'b', long, default_value = "0", value_hint = ValueHint::Other)]  // Numeric hint
    blocks: u64,

    /// Enable pool mining mode
    #[arg(long)]
    pool: bool,

    /// CPU intensity percent (0-100)
    #[arg(short = 'i', long, default_value = "100", value_hint = ValueHint::Other)]  // Numeric hint
    intensity: u8,

    /// Node address (host:port)
    #[arg(short = 'n', long, default_value = "owonero.yabai.buzz:6969", value_hint = ValueHint::Hostname)]  // Hostname/port completion
    node_addr: String,

    /// Number of mining threads
    #[arg(short = 't', long, default_value = "4", value_hint = ValueHint::Other)]  // Numeric hint
    threads: usize,

    #[arg(long = "install-completions", value_name = "SHELL")]
    pub install_completions: Option<String>,

    /// Comma-separated list of peer addresses
    #[arg(long, value_hint = ValueHint::Hostname)]  // Hostname completion for peers
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

    /// Destination address for sending OWE
    #[arg(long, value_hint = ValueHint::Other)]  // Could be a wallet address; use Other for custom
    to: Option<String>,

    /// Amount to send (can be decimal, e.g. 1.5)
    #[arg(long, value_hint = ValueHint::Other)]  // Numeric/decimal hint
    amount: Option<f64>,
}

// enum Command {
//     Daemon,
//     Tui,
//     Mine,
//     MinerUi,
//     WalletInfo,
//     Send,
//     Completions { shell: String, output: Option<String> },
// }

// fn determine_command(cli: &Cli) -> Command {
//     if cli.send {
//         return Command::Send;
//     }
//     if cli.daemon {
//         Command::Daemon
//     } else if cli.tui {
//         Command::Tui
//     } else if cli.mine {
//         Command::Mine
//     } else if cli.miner_ui {
//         Command::MinerUi
//     } else {
//         Command::WalletInfo
//     }
// }

fn load_and_merge_config(cli: &Cli) -> anyhow::Result<config::Config> {
    // Load config
    let config_path: std::path::PathBuf = dirs::config_dir().unwrap_or_else(|| std::env::temp_dir()).join("owonero").join("config.json");
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
    config.peers = cli.peers.as_ref().map(|s| s.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default();
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
            completions::print_to_stdout("bash")?;  // Default to bash for stdout; adjust as needed
        } else {
            let path = completions::install_user_completion(shell)?;
            println!("Completions installed to: {}", path.display());
        }
        return Ok(());  // Exit early after handling completions
    }

    // Compose version string including short git commit (set by build.rs) and print ASCII logo
    let full_version = format!("v{}=>{}", env!("CARGO_PKG_VERSION"), option_env!("GIT_HASH_SHORT").unwrap_or("unknown"));
    println!("{}", ASCII_LOGO.replace("%s", &full_version).purple());

    let config = load_and_merge_config(&cli)?;

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
    } else if cli.tui {
        run_tui_mode().await
    } else if cli.miner_ui {
        run_miner_ui().await  // Note: This calls `run_miner_ui` from miner_ui module, not the unused `run_miner_ui_mode`
    } else if cli.send {
        run_send_mode(cli, config).await
    } else {
        // Default to wallet info if no mode flag is set
        run_wallet_info_mode(config).await
    }
}

// Blockchain path lives in the config directory, use `config::get_blockchain_path()`.

async fn run_daemon_mode(cli: Cli, config: config::Config) -> anyhow::Result<()> {
    let blockchain = if !cli.no_init {
        let mut bc = blockchain::Blockchain::load_from_file(crate::config::get_blockchain_path())?;
        eprintln!("Syncing blockchain...");
        bc.sync(&config.peers).await?;  // Add this line
        bc
    } else {
        blockchain::Blockchain::new()
    };

    let blockchain = Arc::new(std::sync::Mutex::new(blockchain));
    let pm = Arc::new(daemon::PeerManager::new());

    // Add peers from config
    for peer in &config.peers {
        pm.add_peer(peer.clone());
    }

    daemon::run_daemon(config.daemon_port, blockchain, pm, config.pool).await
}

async fn run_tui_mode() -> anyhow::Result<()> {
    // TODO: Implement wallet TUI
    println!("{}", "Wallet TUI not yet implemented".yellow());
    Ok(())
}

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
        ).await {
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
    let wallet = config::load_wallet()?;
    // Try to sync the local chain from a configured node. Prefer the node
    // address stored in the wallet (if present), otherwise fall back to
    // the CLI/config node address. This allows wallets to remember which
    // node they primarily communicate with.
    let node_to_use = wallet.node_address.clone().unwrap_or(config.node_address.clone());

    // Load local chain
    let mut blockchain = blockchain::Blockchain::load_from_file(crate::config::get_blockchain_path())?;

    if config.sync_on_startup {
        if let Ok(stream) = tokio::net::TcpStream::connect(&node_to_use).await {
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
                        // If newer, replace local copy and persist
                        if new_chain.chain.len() > blockchain.chain.len() {
                            blockchain = new_chain;
                            let _ = blockchain.save_to_file(crate::config::get_blockchain_path());
                            println!("Synchronized blockchain from node {}", node_to_use);
                        }
                    }
                }
            }
        } else {
            eprintln!("Warning: failed to connect to node {} to sync blockchain", node_to_use);
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
    let mut matches = 0usize;
    for block in &blockchain.chain {
        for tx in &block.transactions {
            if tx.to.trim().eq_ignore_ascii_case(&wallet.address.trim()) || tx.from.trim().eq_ignore_ascii_case(&wallet.address.trim()) {
                matches += 1;
                println!("Block {}: tx from='{}' to='{}' amount={} sig={}", block.index, tx.from, tx.to, tx.amount, tx.signature);
            }
        }
    }
    if matches == 0 {
        println!("No matching transactions found in chain for this wallet.");
    } else {
        println!("Found {} matching transaction(s)", matches);
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

    // Connect to node and submit transaction
    let stream = tokio::net::TcpStream::connect(&config.node_address).await?;
    let (r, mut w) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(r);

    // Skip greeting line if present
    let mut greeting = String::new();
    let _ = reader.read_line(&mut greeting).await;

    w.write_all(b"submittx\n").await?;
    let tx_json = serde_json::to_string(&tx)?;
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
                        if peer.is_empty() { continue; }
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

        println!("Failed to submit transaction: no peers accepted submittx. Either the node is outdated or no peers support transaction submission.");
        return Ok(());
    } else {
        println!("Node response: {}", resp);
    }

    Ok(())
}
