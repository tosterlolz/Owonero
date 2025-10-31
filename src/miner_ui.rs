use crate::miner::MinerStats;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct MinerUI {
    last_display: Instant,
    stats_history: Vec<MinerStats>,
}

impl MinerUI {
    pub fn new() -> anyhow::Result<Self> {
        Ok(MinerUI {
            last_display: Instant::now(),
            stats_history: Vec::new(),
        })
    }

    pub async fn run(
        &mut self,
        mut stats_rx: mpsc::Receiver<MinerStats>,
        mut log_rx: mpsc::Receiver<String>,
        shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    ) -> anyhow::Result<()> {
        self.display_header();

        loop {
            tokio::select! {
                Some(stats) = stats_rx.recv() => {
                    self.stats_history.push(stats.clone());
                    if self.stats_history.len() > 60 {
                        self.stats_history.remove(0);
                    }
                    
                    if self.last_display.elapsed() >= Duration::from_millis(500) {
                        self.display_stats(&stats);
                        self.last_display = Instant::now();
                    }
                }
                Some(log_msg) = log_rx.recv() => {
                    println!("\n  {}", format_log_message(&log_msg));
                    self.display_header();
                    if let Some(stats) = self.stats_history.last() {
                        self.display_stats(stats);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n\n  ⏹  Shutdown signal received...");
                    if let Some(tx) = shutdown_tx {
                        let _ = tx.send(true);
                    }
                    break;
                }
            }
        }

        println!("\n  Mining stopped.\n");
        Ok(())
    }

    fn display_header(&self) {
        println!("\n");
        println!("  ╔════════════════════════════════════════════════════════════════════════╗");
        println!("  ║                        ⛏  OWONERO MINER  ⛏                          ║");
        println!("  ╚════════════════════════════════════════════════════════════════════════╝");
        println!();
    }

    fn display_stats(&self, stats: &MinerStats) {
        let hps = stats.total_hps;
        let avg_hps = stats.avg_min;
        let mined = stats.mined;
        let accepted = stats.accepted;
        let rejected = stats.rejected;
        let threads = stats.threads;
        let uptime = stats.uptime;

        let total_shares = accepted + rejected;
        let accept_rate = if total_shares > 0 {
            (accepted as f64 / total_shares as f64) * 100.0
        } else {
            0.0
        };

        // Calculate difficulty from attempts and solutions (rough estimate)
        let avg_diff_per_solution = if mined > 0 {
            stats.attempts / mined
        } else {
            0
        };

        // Uptime formatting
        let uptime_str = format_uptime(uptime);

        // Clear previous line and show new stats
        print!("\r");

        println!("  ┌─ Performance ────────────────────────────────────────────────────────┐");
        println!(
            "  │  Hashrate: {:>12} H/s  │  Avg: {:>12} H/s                       │",
            format_number(hps),
            format_number(avg_hps as u64)
        );
        println!(
            "  │  Solutions: {:>10}       │  Avg Diff: {:>12}                  │",
            mined,
            format_number(avg_diff_per_solution)
        );
        println!("  ├─ Shares ─────────────────────────────────────────────────────────────┤");
        println!(
            "  │  Accepted: {:>11}      │  Rejected: {:>11}  ({:.2}%)        │",
            accepted, rejected, 100.0 - accept_rate
        );
        println!(
            "  │  Accept Rate: {:.1}%                                              │",
            accept_rate
        );
        println!("  ├─ Session ────────────────────────────────────────────────────────────┤");
        println!(
            "  │  Threads: {}    │  Total Attempts: {:>12}                │",
            threads,
            format_number(stats.attempts)
        );
        println!(
            "  │  Uptime: {}                                          │",
            pad_right(&uptime_str, 49)
        );
        println!("  └───────────────────────────────────────────────────────────────────────┘");
    }
}

fn format_number(n: u64) -> String {
    match n {
        0..=999 => format!("{}", n),
        1_000..=999_999 => format!("{:.2}K", n as f64 / 1_000.0),
        1_000_000..=999_999_999 => format!("{:.2}M", n as f64 / 1_000_000.0),
        _ => format!("{:.2}G", n as f64 / 1_000_000_000.0),
    }
}

fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{}h {}m {}s", hours, minutes, secs)
}

fn format_log_message(msg: &str) -> String {
    if msg.contains("accepted") || msg.contains("Accepted") {
        format!("✓ {}", msg)
    } else if msg.contains("rejected") || msg.contains("Rejected") {
        format!("✗ {}", msg)
    } else if msg.contains("error") || msg.contains("Error") {
        format!("⚠ {}", msg)
    } else {
        format!("ℹ {}", msg)
    }
}

fn pad_right(s: &str, width: usize) -> String {
    format!("{:<width$}", s, width = width)
}
