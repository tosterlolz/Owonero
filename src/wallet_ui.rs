use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;

use crate::wallet::Wallet;
use crate::blockchain::Blockchain;
use reqwest::Client;

#[derive(Debug, Clone, Copy, PartialEq)]
enum UIMode {
    View,
    SendRecipient,
    SendAmount,
    SendPrivKey,
}

pub struct WalletUI {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    logs: Vec<String>,
    max_logs: usize,
    mode: UIMode,
    input_buffer: String,
    send_recipient: String,
    send_amount: String,
    send_privkey: String,
}

impl WalletUI {
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
                mode: UIMode::View,
                input_buffer: String::new(),
                send_recipient: String::new(),
                send_amount: String::new(),
                send_privkey: String::new(),
            })
    }

    pub fn add_log(&mut self, log: String) {
        self.logs.push(log);
        if self.logs.len() > self.max_logs {
            self.logs.remove(0);
        }
    }

    pub fn draw(&mut self, wallet: &Wallet, blockchain: &Blockchain) -> anyhow::Result<()> {
        let logs = &self.logs;
        let mode = self.mode;
        let input_buffer = &self.input_buffer;

        self.terminal.draw(|f| {
            let size = f.area();

            // Create main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),  // Header
                    Constraint::Length(8),  // Wallet info
                    Constraint::Min(8),     // Logs
                    Constraint::Length(5),  // Input area (if needed)
                ])
                .split(size);

            // Header
            let header_text = "⬡ OWONERO WALLET";
            let header = Paragraph::new(header_text)
                .block(Block::default().borders(Borders::ALL).title("WALLET"))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(header, chunks[0]);

            // Wallet Info
            draw_wallet_info(f, chunks[1], wallet, blockchain);

            // Logs
            draw_logs(f, chunks[2], logs);

            // Input area based on mode
                match mode {
                    UIMode::View => {
                        draw_footer(f, chunks[3]);
                    }
                    UIMode::SendRecipient => {
                        draw_input(f, chunks[3], "Enter recipient address (Esc to cancel):", input_buffer);
                    }
                    UIMode::SendAmount => {
                        draw_input(f, chunks[3], "Enter amount in OWE (Esc to cancel):", input_buffer);
                    }
                    UIMode::SendPrivKey => {
                        draw_input(f, chunks[3], "Enter your private key (Esc to cancel):", input_buffer);
                    }
                }
        })?;

        Ok(())
    }

    pub async fn run(&mut self, wallet: Wallet, _blockchain: Blockchain) -> anyhow::Result<()> {
        let mut last_draw = Instant::now();
        let config = crate::config::load_config().ok();
        let client = Client::new();
        let node_addr = wallet.node_address.clone()
            .or_else(|| config.as_ref().map(|c| c.node_address.clone()))
            .unwrap_or_else(|| "http://127.0.0.1:6767".to_string());

        // Helper to fetch blockchain from node
        async fn fetch_chain(client: &Client, node_addr: &str) -> Option<Blockchain> {
            let url = format!("{}/api/chain", node_addr.trim_end_matches('/'));
            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(chain) = resp.json::<Blockchain>().await {
                        Some(chain)
                    } else { None }
                }
                Err(_) => None,
            }
        }

        let mut blockchain = fetch_chain(&client, &node_addr).await.unwrap_or_else(|| Blockchain::new());

        self.add_log(format!("Wallet loaded: {}", &wallet.address[..8]));
        self.add_log(format!("Balance: {} OWE", wallet.get_balance(&blockchain) as f64));
        self.add_log("Press 's' to send OWE, 'r' to refresh, 'q' to quit".to_string());

        loop {
            // Handle input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match self.mode {
                        UIMode::View => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('c') => {
                                    self.logs.clear();
                                }
                                KeyCode::Char('s') => {
                                    self.mode = UIMode::SendRecipient;
                                    self.input_buffer.clear();
                                    self.add_log("Enter recipient address:".to_string());
                                }
                                KeyCode::Char('r') => {
                                    // Refresh blockchain from node
                                    if let Some(new_chain) = fetch_chain(&client, &node_addr).await {
                                        blockchain = new_chain;
                                        let balance = wallet.get_balance(&blockchain) as f64;
                                        self.add_log(format!("Balance: {} OWE", balance));
                                    } else {
                                        self.add_log("Failed to fetch blockchain from node".to_string());
                                    }
                                }
                                _ => {}
                            }
                        }
                        UIMode::SendRecipient => {
                            match key.code {
                                KeyCode::Char(c) => {
                                    self.input_buffer.push(c);
                                }
                                KeyCode::Backspace => {
                                    self.input_buffer.pop();
                                }
                                KeyCode::Enter => {
                                    self.send_recipient = self.input_buffer.clone();
                                    self.mode = UIMode::SendAmount;
                                    self.input_buffer.clear();
                                    self.add_log("Enter amount in OWE:".to_string());
                                }
                                KeyCode::Esc => {
                                    self.mode = UIMode::View;
                                    self.input_buffer.clear();
                                    self.add_log("Send cancelled".to_string());
                                }
                                _ => {}
                            }
                        }
                        UIMode::SendAmount => {
                            match key.code {
                                KeyCode::Char(c) if c.is_numeric() || c == '.' => {
                                    self.input_buffer.push(c);
                                }
                                KeyCode::Backspace => {
                                    self.input_buffer.pop();
                                }
                                KeyCode::Enter => {
                                    self.send_amount = self.input_buffer.clone();
                                    let amount_f = self.send_amount.parse::<f64>().unwrap_or(0.0);
                                    if amount_f <= 0.0 {
                                        self.add_log("Invalid amount".to_string());
                                        self.mode = UIMode::View;
                                        self.input_buffer.clear();
                                        self.send_recipient.clear();
                                        self.send_amount.clear();
                                    } else {
                                        self.mode = UIMode::SendPrivKey;
                                        self.input_buffer.clear();
                                        self.add_log("Enter your private key:".to_string());
                                    }
                                }
                                KeyCode::Esc => {
                                    self.mode = UIMode::View;
                                    self.input_buffer.clear();
                                    self.add_log("Send cancelled".to_string());
                                }
                                _ => {}
                            }
                        }
                            UIMode::SendPrivKey => {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        self.input_buffer.push(c);
                                    }
                                    KeyCode::Backspace => {
                                        self.input_buffer.pop();
                                    }
                                    KeyCode::Enter => {
                                        self.send_privkey = self.input_buffer.clone();
                                        let amount_f = self.send_amount.parse::<f64>().unwrap_or(0.0);
                                        let amount_units = (amount_f * 1000.0).round() as i64;
                                        // Validate private key and public key
                                        let pub_key = &wallet.pub_key;
                                        let priv_key = &self.send_privkey;
                                        let mut tx = crate::blockchain::Transaction {
                                            from: wallet.address.clone(),
                                            pub_key: pub_key.clone(),
                                            to: self.send_recipient.clone(),
                                            amount: amount_units,
                                            signature: String::new(),
                                        };
                                        match crate::blockchain::sign_transaction(&mut tx, priv_key) {
                                            Ok(_) => {
                                                // Verify signature matches public key
                                                if crate::blockchain::verify_transaction_signature(&tx, pub_key) {
                                                    self.add_log(format!("Transaction created: {} OWE to {}", amount_f, &self.send_recipient[..8]));
                                                    self.add_log("Attempting to send...".to_string());
                                                    let node_addr = wallet.node_address.clone()
                                                        .or_else(|| config.as_ref().map(|c| c.node_address.clone()))
                                                        .unwrap_or_else(|| "127.0.0.1:6969".to_string());
                                                    match send_transaction(&tx, &node_addr).await {
                                                        Ok(_) => {
                                                            self.add_log("✓ Transaction sent!".to_string());
                                                            // Refresh blockchain after sending
                                                            if let Some(new_chain) = fetch_chain(&client, &node_addr).await {
                                                                blockchain = new_chain;
                                                                let balance = wallet.get_balance(&blockchain) as f64;
                                                                self.add_log(format!("Balance: {} OWE", balance));
                                                            } else {
                                                                self.add_log("Failed to fetch blockchain from node".to_string());
                                                            }
                                                        },
                                                        Err(e) => self.add_log(format!("✗ Failed to send: {}", e)),
                                                    }
                                                } else {
                                                    self.add_log("Invalid private key for this wallet (signature mismatch)".to_string());
                                                }
                                            }
                                            Err(e) => {
                                                self.add_log(format!("Failed to sign transaction: {}", e));
                                            }
                                        }
                                        self.mode = UIMode::View;
                                        self.input_buffer.clear();
                                        self.send_recipient.clear();
                                        self.send_amount.clear();
                                        self.send_privkey.clear();
                                    }
                                    KeyCode::Esc => {
                                        self.mode = UIMode::View;
                                        self.input_buffer.clear();
                                        self.add_log("Send cancelled".to_string());
                                    }
                                    _ => {}
                                }
                            }
                    }
                }
            }

            // Draw UI at ~10 FPS
            if last_draw.elapsed() >= Duration::from_millis(100) {
                self.draw(&wallet, &blockchain)?;
                last_draw = Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
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

impl Drop for WalletUI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn draw_wallet_info(f: &mut ratatui::Frame, area: Rect, wallet: &Wallet, blockchain: &Blockchain) {
    let balance = wallet.get_balance(blockchain);
    let balance_owe = balance as f64;

    let wallet_info = format!(
        "Address:  {}\n\
         Balance:  {} OWE\n\
         Chain Height: {} blocks",
        &wallet.address,
        format!("{:.3}", balance_owe),
        blockchain.chain.len() - 1
    );

    let wallet_widget = Paragraph::new(wallet_info)
        .block(Block::default().borders(Borders::ALL).title("Wallet Info"))
        .wrap(Wrap { trim: true });
    f.render_widget(wallet_widget, area);
}

fn draw_logs(f: &mut ratatui::Frame, area: Rect, logs: &[String]) {
    let log_text: Vec<Line> = logs.iter().rev().take(20).rev().map(|log| {
        Line::from(vec![Span::styled(log.clone(), Style::default().fg(Color::White))])
    }).collect();

    let logs_widget = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: true });
    f.render_widget(logs_widget, area);
}

fn draw_footer(f: &mut ratatui::Frame, area: Rect) {
    let footer_text = "Press 's' to send | 'r' to refresh | 'q' to quit";
    let footer_widget = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer_widget, area);
}

fn draw_input(f: &mut ratatui::Frame, area: Rect, prompt: &str, input: &str) {
    let text = format!("{}\n> {}", prompt, input);
    let input_widget = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(input_widget, area);
}

async fn send_transaction(tx: &crate::blockchain::Transaction, node_addr: &str) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    
    let mut stream = tokio::net::TcpStream::connect(node_addr).await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);

    // Skip greeting
    let mut greeting = String::new();
    reader.read_line(&mut greeting).await?;

    // Send submittx command
    writer.write_all(b"submittx\n").await?;
    let tx_json = serde_json::to_string(&tx)?;
    writer.write_all(format!("{}\n", tx_json).as_bytes()).await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    let resp = response.trim();
    if resp == "ok" {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Node response: {}", resp))
    }
}
