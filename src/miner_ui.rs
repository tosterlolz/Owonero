use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::miner::MinerStats;

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
⠀⠀⠈⢻⠀⡆⠀⠀⠀⠀⠀⠀⠀⠀⠐⠆⡘⡇⠀⣼⣿⡇⢀⠀⠀⠀⢱⠁⠀                              V.%s
⠐⢦⣀⠸⡀⢸⣦⣄⡀⠒⠄⠀⠀⠀⢀⣀⣴⠀⣸⣿⣿⠁⣼⢦⠀⠀⠘⠀
⠀⠀⢎⠳⣇⠀⢿⣿⣿⣶⣤⡶⣾⠿⠋⣁⡆⡰⢿⣿⣿⡜⢣⠀⢆⡄⠇⠀
⠀⠀⠈⡄⠈⢦⡘⡇⠟⢿⠙⡿⢀⠐⠁⢰⡜⠀⠀⠙⢿⡇⠀⡆⠈⡟⠀⠀
"#;

pub struct MinerUI {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    logs: Vec<String>,
    max_logs: usize,
    hashrate_history: Vec<f64>,
    max_hashrate_history: usize,
    displayed_hashrate: f64,
}

impl MinerUI {
    pub fn new() -> anyhow::Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            logs: Vec::new(),
            max_logs: 50,
            hashrate_history: Vec::new(),
            max_hashrate_history: 60, // 60 seconds of history
            displayed_hashrate: 0.0,
        })
    }

    pub fn add_log(&mut self, log: String) {
        self.logs.push(log);
        if self.logs.len() > self.max_logs {
            self.logs.remove(0);
        }
    }

    // Update raw hashrate history and a smoothed displayed hashrate (EMA)
    pub fn update_hashrate(&mut self, hashrate: f64) {
        self.hashrate_history.push(hashrate);
        if self.hashrate_history.len() > self.max_hashrate_history {
            self.hashrate_history.remove(0);
        }

        // Exponential moving average smoothing to avoid showing transient zeros
        let alpha = 0.3f64; // smoothing factor (0..1) - higher = more responsive
        if self.displayed_hashrate <= 0.0 {
            self.displayed_hashrate = hashrate;
        } else {
            self.displayed_hashrate = alpha * hashrate + (1.0 - alpha) * self.displayed_hashrate;
        }
    }

    pub fn draw(&mut self, stats: Option<&MinerStats>) -> anyhow::Result<()> {
        let logs = &self.logs;
        let hashrate_history = &self.hashrate_history;

        self.terminal.draw(|f| {
            let size = f.area();

            // Create main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(15), // Header/Ascii
                    Constraint::Length(8),  // Hashrate chart
                    Constraint::Length(6),  // Stats
                    Constraint::Min(10),    // Logs
                    Constraint::Length(3),  // Footer
                ])
                .split(size);

            // Header with ascii (inject build version/commit)
            let full_version = format!(
                "v{}=>{}",
                env!("CARGO_PKG_VERSION"),
                option_env!("GIT_HASH_SHORT").unwrap_or("unknown")
            );
            let ascii = ASCII_LOGO.replace("%s", &full_version);
            let header = Paragraph::new(ascii)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("OWONERO MINER"),
                )
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(header, chunks[0]);

            // Hashrate chart
            draw_hashrate_chart_static(
                f,
                chunks[1],
                stats,
                hashrate_history,
                self.displayed_hashrate,
            );

            // Statistics
            draw_stats_static(f, chunks[2], stats);

            // Logs
            draw_logs_static(f, chunks[3], logs);

            // Footer
            draw_footer_static(f, chunks[4], stats);
        })?;

        Ok(())
    }

    pub async fn run(
        &mut self,
        mut stats_rx: mpsc::Receiver<MinerStats>,
        mut log_rx: mpsc::Receiver<String>,
        shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    ) -> anyhow::Result<()> {
        let mut last_draw = Instant::now();
        let mut last_stats: Option<MinerStats> = None;

        loop {
            // Handle input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Handle quit keys: 'q', Esc, or Ctrl+C
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            // Signal shutdown to the rest of the program if a sender was provided
                            if let Some(ref tx) = shutdown_tx {
                                let _ = tx.send(true);
                            }

                            // As a safety-net, schedule a forced process exit after a short grace
                            // period so background threads/tasks that don't properly listen to
                            // the shutdown channel won't keep the process alive.
                            // Give a small grace so other tasks can shutdown cleanly.
                            let _ = tokio::spawn(async {
                                sleep(Duration::from_millis(500)).await;
                                std::process::exit(0);
                            });

                            break;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let Some(ref tx) = shutdown_tx {
                                let _ = tx.send(true);
                            }

                            let _ = tokio::spawn(async {
                                sleep(Duration::from_millis(500)).await;
                                std::process::exit(0);
                            });

                            break;
                        }
                        KeyCode::Char('c') => {
                            self.logs.clear();
                        }
                        KeyCode::Char('r') => {
                            self.hashrate_history.clear();
                        }
                        _ => {}
                    }
                }
            }

            // Receive stats and logs — drain stats channel to take the most recent sample
            while let Ok(stats) = stats_rx.try_recv() {
                // store last stats so draw() can access fields (threads, uptime, etc.)
                self.update_hashrate(stats.total_hps as f64);
                last_stats = Some(stats);
            }

            if let Ok(log) = log_rx.try_recv() {
                self.add_log(log);
            }

            // Draw UI at ~10 FPS
            if last_draw.elapsed() >= Duration::from_millis(100) {
                self.draw(last_stats.as_ref())?;
                last_draw = Instant::now();
            }

            sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    pub fn cleanup(&mut self) -> anyhow::Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for MinerUI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn draw_hashrate_chart_static(
    f: &mut ratatui::Frame,
    area: Rect,
    stats: Option<&MinerStats>,
    hashrate_history: &[f64],
    displayed_hashrate: f64,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Hashrate (H/s)");

    f.render_widget(block, area);

    if stats.is_some() {
        let inner_area = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);

        // Simple ASCII chart
        let chart_height = inner_area.height as usize;
        let chart_width = inner_area.width as usize;

        if !hashrate_history.is_empty() {
            let max_rate = hashrate_history.iter().cloned().fold(0.0f64, f64::max);
            let scale = if max_rate > 0.0 {
                chart_height as f64 / max_rate
            } else {
                1.0
            };

            for (i, &rate) in hashrate_history.iter().enumerate() {
                if i >= chart_width {
                    break;
                }

                let bar_height = (rate * scale) as usize;
                for j in 0..bar_height.min(chart_height) {
                    let y = inner_area.y + (chart_height - 1 - j) as u16;
                    let x = inner_area.x + i as u16;

                    if y < inner_area.y + inner_area.height && x < inner_area.x + inner_area.width {
                        let symbol = if j == bar_height - 1 { '█' } else { '█' };
                        let span =
                            Span::styled(symbol.to_string(), Style::default().fg(Color::Green));
                        f.buffer_mut().set_span(x, y, &span, inner_area.width);
                    }
                }
            }
        }

        // Current hashrate text (use smoothed displayed_hashrate)
        let hashrate_text = format!("Current: {}", format_hashrate(displayed_hashrate as u64));
        let text = Paragraph::new(hashrate_text).style(Style::default().fg(Color::White));
        f.render_widget(text, inner_area);
    }
}

fn draw_stats_static(f: &mut ratatui::Frame, area: Rect, stats: Option<&MinerStats>) {
    let stats_text = if let Some(stats) = stats {
        format!(
            "Threads: {} | Solutions: {} | Accepted: {} | Rejected: {} | Uptime: {}\n\
             1min: {} | 1hour: {} | 1day: {} | Pool: {}",
            stats.threads,
            stats.sols,
            stats.accepted,
            stats.rejected,
            format_duration(stats.uptime),
            format_hashrate(stats.avg_min as u64),
            format_hashrate(stats.avg_hour as u64),
            format_hashrate(stats.avg_day as u64),
            if stats.pool_mode { "Yes" } else { "No" }
        )
    } else {
        "Waiting for mining statistics...".to_string()
    };

    let stats_widget = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title("Statistics"))
        .wrap(Wrap { trim: true });
    f.render_widget(stats_widget, area);
}

fn draw_logs_static(f: &mut ratatui::Frame, area: Rect, logs: &[String]) {
    let log_text: Vec<Line> = logs
        .iter()
        .rev()
        .take(20)
        .rev()
        .map(|log| {
            Line::from(vec![Span::styled(
                log.clone(),
                Style::default().fg(Color::White),
            )])
        })
        .collect();

    let logs_widget = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: true });
    f.render_widget(logs_widget, area);
}

fn draw_footer_static(f: &mut ratatui::Frame, area: Rect, _stats: Option<&MinerStats>) {
    let footer_text = "Press 'q' or 'Ctrl+C' to quit | 'c' to clear logs | 'r' to reset stats";

    let footer_widget = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(footer_widget, area);
}

fn format_hashrate(hps: u64) -> String {
    if hps >= 1_000_000_000 {
        format!("{:.2} GH/s", hps as f64 / 1_000_000_000.0)
    } else if hps >= 1_000_000 {
        format!("{:.2} MH/s", hps as f64 / 1_000_000.0)
    } else if hps >= 1_000 {
        format!("{:.2} KH/s", hps as f64 / 1_000.0)
    } else {
        format!("{} H/s", hps)
    }
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}

pub async fn run_miner_ui() -> anyhow::Result<()> {
    let mut ui = MinerUI::new()?;

    // Add initial logs
    ui.add_log("OWONERO Miner UI started".to_string());
    ui.add_log("Press 'q' to quit, 'c' to clear logs, 'r' to reset stats".to_string());

    // Create dummy channels for demo
    let (_stats_tx, stats_rx) = mpsc::channel::<MinerStats>(10);
    let (_log_tx, log_rx) = mpsc::channel::<String>(100);

    // Run the UI (no external shutdown sender when running standalone)
    ui.run(stats_rx, log_rx, None).await?;

    Ok(())
}
