mod blockchain;
mod wallet;
mod miner;
mod daemon;
mod config;
mod update;
mod miner_ui;

use clap::Parser;
use std::path::Path;
use std::sync::Arc;
use colored::Colorize;

const ASCII_LOGO: &str = r#"
⠀⠀⠀⠀⡰⠁⠀⠀⢀⢔⣔⣤⠐⠒⠒⠒⠒⠠⠄⢀⠀⠐⢀⠀⠀⠀⠀⠀⠀⠀
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
⠀⠀⠈⢻⠀⡆⠀⠀⠀⠀⠀⠀⠀⠀⠐⠆⡘⡇⠀⣼⣿⡇⢀⠀⠀⠀⢱⠁⠀ 							   V.%s
⠐⢦⣀⠸⡀⢸⣦⣄⡀⠒⠄⠀⠀⠀⢀⣀⣴⠀⣸⣿⣿⠁⣼⢦⠀⠀⠘⠀		
⠀⠀⢎⠳⣇⠀⢿⣿⣿⣶⣤⡶⣾⠿⠋⣁⡆⡰⢿⣿⣿⡜⢣⠀⢆⡄⠇⠀
⠀⠀⠈⡄⠈⢦⡘⡇⠟⢿⠙⡿⢀⠐⠁⢰⡜⠀⠀⠙⢿⡇⠀⡆⠈⡟⠀⠀      
"#;

#[derive(Parser)]
#[command(name = "owonero-rs")]
#[command(version = "0.4.1")]
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
    #[arg(short = 'p', long, default_value = "6969")]
    port: u16,

    /// Web stats server port
    #[arg(long, default_value = "6767")]
    web_port: u16,

    /// Wallet file path
    #[arg(short = 'w', long, default_value = "wallet.json")]
    wallet_path: String,

    /// Mine blocks
    #[arg(short = 'm', long)]
    mine: bool,

    /// How many blocks to mine (0 = forever)
    #[arg(short = 'b', long, default_value = "0")]
    blocks: u64,

    /// Enable pool mining mode
    #[arg(long)]
    pool: bool,

    /// CPU intensity percent (0-100)
    #[arg(short = 'i', long, default_value = "100")]
    intensity: u8,

    /// Node address (host:port)
    #[arg(short = 'n', long, default_value = "localhost:6969")]
    node_addr: String,

    /// Number of mining threads
    #[arg(short = 't', long, default_value = "1")]
    threads: usize,

    /// Comma-separated list of peer addresses
    #[arg(long)]
    peers: Option<String>,

    /// Skip automatic update check
    #[arg(long)]
    no_update: bool,

    /// Don't initialize blockchain, rely on syncing
    #[arg(long)]
    no_init: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Print ASCII logo
    println!("{}", ASCII_LOGO.replace("%s", env!("CARGO_PKG_VERSION")).purple());

    // Load config
    let config_path = "config.json";
    let _config = if Path::new(config_path).exists() {
        config::load_config(config_path)?
    } else {
        config::Config::default()
    };

    // Override config with CLI args
    let config = config::Config {
        node_address: cli.node_addr.clone(),
        daemon_port: cli.port,
        web_port: cli.web_port,
        wallet_path: cli.wallet_path.clone(),
        mining_threads: cli.threads,
        peers: cli.peers.as_ref().map(|s| s.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default(),
        auto_update: !cli.no_update,
        sync_on_startup: true,
        target_block_time: 30,
        mining_intensity: cli.intensity,
        pool: cli.pool,
    };

    // Save updated config
    config::save_config(&config, config_path)?;

    println!("{}", format!("OWONERO-RS v{}", env!("CARGO_PKG_VERSION")).green());

    // Check for updates if enabled
    if config.auto_update {
        if let Err(e) = update::check_for_updates().await {
            eprintln!("{}", format!("Failed to check for updates: {}", e).red());
        }
    }

    if cli.tui {
        // TODO: Implement wallet TUI
        println!("{}", "Wallet TUI not yet implemented".yellow());
        return Ok(());
    }

    if cli.daemon {
        let blockchain = if !cli.no_init {
            let mut bc = blockchain::Blockchain::load_from_file("blockchain.json")?;
            bc.target_block_time = config.target_block_time;
            bc.save_to_file("blockchain.json")?;
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

        daemon::run_daemon(config.daemon_port, blockchain, pm, config.pool).await?;
        return Ok(());
    }

    if cli.mine {
        // Always start mining with UI
        let (stats_tx, stats_rx) = tokio::sync::mpsc::channel(10);
        let (log_tx, log_rx) = tokio::sync::mpsc::channel(100);

        // Create shutdown notifier so UI can request program shutdown
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        // Start mining in background
        let mining_handle = tokio::spawn(async move {
            if let Err(e) = miner::start_mining(
                &config.wallet_path,
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

        // Wait for UI to finish. When UI exits (user pressed 'q'), request shutdown and abort mining task.
        let _ = ui_handle.await;
        // Signal shutdown (UI also sends this, but send again to be sure)
        let _ = shutdown_tx.send(true);
        // Abort mining task so the program can exit promptly
        mining_handle.abort();
        // Wait for mining task to stop (best-effort)
        let _ = mining_handle.await;
        return Ok(());
    }

    if cli.miner_ui {
        // Show miner UI without mining
        miner_ui::run_miner_ui().await?;
        return Ok(());
    }

    // Default: show wallet info
    let wallet = wallet::load_or_create_wallet(&config.wallet_path)?;
    let blockchain = blockchain::Blockchain::load_from_file("blockchain.json")?;
    let balance = wallet.get_balance(&blockchain);

    println!("{} {}", "Wallet:".blue(), wallet.address);
    println!("{} {}", "Balance:".yellow(), balance);
    println!("{} {}", "Chain height:".cyan(), blockchain.chain.len() - 1);

    // Demonstrate transaction creation (for testing)
    if balance > 0 {
        let test_tx = wallet.create_signed_transaction("test-recipient", 1)?;
        println!("{} Created test transaction: {} -> {} (amount: {})", "DEBUG:".magenta(), 
                &test_tx.from[..16], test_tx.to, test_tx.amount);
    }

    Ok(())
}
