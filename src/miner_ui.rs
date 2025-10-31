use crate::miner::MinerStats;
use crossterm::{
    event::{self, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct MinerUI {
    stats: Option<MinerStats>,
    logs: Vec<String>,
}

impl MinerUI {
    pub fn new() -> anyhow::Result<Self> {
        Ok(MinerUI {
            stats: None,
            logs: Vec::new(),
        })
    }

    pub async fn run(
        &mut self,
        mut stats_rx: mpsc::Receiver<MinerStats>,
        mut log_rx: mpsc::Receiver<String>,
        shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    ) -> anyhow::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.ui_loop(&mut terminal, &mut stats_rx, &mut log_rx, shutdown_tx).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        res
    }

    async fn ui_loop<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        stats_rx: &mut mpsc::Receiver<MinerStats>,
        log_rx: &mut mpsc::Receiver<String>,
        shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    ) -> anyhow::Result<()> {
        let mut update_interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            // Handle events (non-blocking)
            if event::poll(Duration::from_millis(10))? {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if let Some(tx) = &shutdown_tx {
                                let _ = tx.send(true);
                            }
                            break;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let Some(tx) = &shutdown_tx {
                                let _ = tx.send(true);
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }

            // Update stats
            while let Ok(stats) = stats_rx.try_recv() {
                self.stats = Some(stats);
            }

            // Update logs
            while let Ok(log) = log_rx.try_recv() {
                self.logs.push(log);
                if self.logs.len() > 50 {
                    self.logs.remove(0);
                }
            }

            // Render UI
            update_interval.tick().await;
            terminal.draw(|f| draw_ui(f, &self.stats, &self.logs))?;
        }

        Ok(())
    }
}

fn draw_ui(f: &mut ratatui::Frame, stats: &Option<MinerStats>, logs: &[String]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Min(10),
            ]
            .as_ref(),
        )
        .split(f.size());

    // Header
    let header = Paragraph::new("⛏  OWONERO MINER  ⛏")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Stats section
    if let Some(stats) = stats {
        render_stats(f, chunks[1], stats);
    }

    // Logs section
    render_logs(f, chunks[2], logs);
}

fn render_stats(f: &mut ratatui::Frame, area: Rect, stats: &MinerStats) {
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
            ]
            .as_ref(),
        )
        .split(area);

    // Performance block
    let perf_block = Block::default()
        .title(" Performance ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));
    let perf_inner = perf_block.inner(inner_chunks[0]);
    f.render_widget(perf_block, inner_chunks[0]);

    let perf_lines = vec![
        Line::from(vec![
            Span::raw("Hashrate: "),
            Span::styled(
                format!("{} H/s", format_number(stats.total_hps)),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Average: "),
            Span::styled(
                format!("{} H/s", format_number(stats.avg_min as u64)),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(perf_lines), perf_inner);

    // Solutions block
    let sol_block = Block::default()
        .title(" Solutions ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Magenta));
    let sol_inner = sol_block.inner(inner_chunks[1]);
    f.render_widget(sol_block, inner_chunks[1]);

    let total_shares = stats.accepted + stats.rejected;
    let accept_rate = if total_shares > 0 {
        (stats.accepted as f64 / total_shares as f64) * 100.0
    } else {
        0.0
    };

    let sol_lines = vec![
        Line::from(vec![
            Span::raw("Mined: "),
            Span::styled(
                stats.mined.to_string(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Accept Rate: "),
            Span::styled(
                format!("{:.1}%", accept_rate),
                if accept_rate > 90.0 {
                    Style::default().fg(Color::Green)
                } else if accept_rate > 70.0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(sol_lines), sol_inner);

    // Shares block
    let shares_block = Block::default()
        .title(" Shares ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Blue));
    let shares_inner = shares_block.inner(inner_chunks[2]);
    f.render_widget(shares_block, inner_chunks[2]);

    let shares_lines = vec![
        Line::from(vec![
            Span::raw("Accepted: "),
            Span::styled(
                stats.accepted.to_string(),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("Rejected: "),
            Span::styled(
                stats.rejected.to_string(),
                Style::default().fg(Color::Red),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(shares_lines), shares_inner);

    // Session block
    let session_block = Block::default()
        .title(" Session ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let session_inner = session_block.inner(inner_chunks[3]);
    f.render_widget(session_block, inner_chunks[3]);

    let session_lines = vec![
        Line::from(vec![
            Span::raw("Threads: "),
            Span::styled(
                stats.threads.to_string(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Uptime: "),
            Span::styled(
                format_uptime(stats.uptime),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Attempts: "),
            Span::styled(
                format_number(stats.attempts),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(session_lines), session_inner);
}

fn render_logs(f: &mut ratatui::Frame, area: Rect, logs: &[String]) {
    let logs_block = Block::default()
        .title(" Activity Log ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Gray));
    let inner = logs_block.inner(area);
    f.render_widget(logs_block, area);

    let log_items: Vec<ListItem> = logs
        .iter()
        .rev()
        .take((inner.height as usize).saturating_sub(1))
        .map(|log| {
            let style = if log.contains("accepted") || log.contains("Accepted") {
                Style::default().fg(Color::Green)
            } else if log.contains("rejected") || log.contains("Rejected") {
                Style::default().fg(Color::Red)
            } else if log.contains("error") || log.contains("Error") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(log.clone(), style)))
        })
        .collect();

    let log_list = List::new(log_items);
    f.render_widget(log_list, inner);
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
