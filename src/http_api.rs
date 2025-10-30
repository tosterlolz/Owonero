use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Clone)]
pub struct AppState {
    pub daemon_addr: String,
}

#[derive(Serialize, Deserialize)]
pub struct WalletHashrateResponse {
    pub wallet: String,
    pub hashrate: f64,
    pub last_update: u64,
}

#[derive(Serialize, Deserialize)]
pub struct WalletBalanceQuery {
    pub addr: String,
}

async fn connect_to_daemon(daemon_addr: &str, command: &str) -> anyhow::Result<String> {
    let mut stream = TcpStream::connect(daemon_addr).await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let mut greeting = String::new();
    reader.read_line(&mut greeting).await?;

    // Send command
    writer.write_all(format!("{}\n", command).as_bytes()).await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    Ok(response.trim().to_string())
}

pub async fn get_stats(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let mut response = json!({});

    // Get height
    if let Ok(height_str) = connect_to_daemon(&state.daemon_addr, "getheight").await {
        if let Ok(height) = height_str.parse::<u64>() {
            response["height"] = json!(height);
        }
    }

    // Get peers
    if let Ok(peers_str) = connect_to_daemon(&state.daemon_addr, "getpeers").await {
        if let Ok(peers) = serde_json::from_str::<Vec<String>>(&peers_str) {
            response["peers"] = json!(peers);
        }
    }

    // Get latest block
    if let Ok(latest_str) = connect_to_daemon(&state.daemon_addr, "getlatest").await {
        if let Ok(latest) = serde_json::from_str::<Value>(&latest_str) {
            response["latest_block"] = latest;
        }
    }

    // Get mempool
    if let Ok(mempool_str) = connect_to_daemon(&state.daemon_addr, "getmempool").await {
        if let Ok(mempool) = serde_json::from_str::<Vec<Value>>(&mempool_str) {
            response["mempool"] = json!(mempool);
        }
    }

    // Get network hashrate
    if let Ok(hashrate_str) = connect_to_daemon(&state.daemon_addr, "getnetworkhashrate").await {
        if let Ok(hashrate_obj) = serde_json::from_str::<Value>(&hashrate_str) {
            if let Some(hr) = hashrate_obj.get("network_hashrate").and_then(|v| v.as_f64()) {
                response["network_hashrate"] = json!(hr);
            }
        }
    }

    Ok(Json(response))
}

pub async fn get_wallet_hashrate(
    State(state): State<AppState>,
    Query(query): Query<WalletBalanceQuery>,
) -> Result<Json<WalletHashrateResponse>, StatusCode> {
    let command = format!("getwallethashrate {}", query.addr);
    match connect_to_daemon(&state.daemon_addr, &command).await {
        Ok(response_str) => {
            match serde_json::from_str::<WalletHashrateResponse>(&response_str) {
                Ok(resp) => Ok(Json(resp)),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

pub async fn get_wallet_balance(
    State(state): State<AppState>,
    Query(query): Query<WalletBalanceQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Get the blockchain to calculate balance
    match connect_to_daemon(&state.daemon_addr, "getchain").await {
        Ok(chain_str) => {
            match serde_json::from_str::<Value>(&chain_str) {
                Ok(_chain_obj) => {
                    // For now, return the chain info
                    // In a real implementation, you'd calculate the balance from the chain
                    Ok(Json(json!({
                        "wallet": query.addr,
                        "balance": 0.0,
                        "last_sync": chrono::Utc::now().timestamp()
                    })))
                }
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn get_index() -> Result<String, StatusCode> {
    match tokio::fs::read_to_string("index.html").await {
        Ok(content) => Ok(content),
        Err(_) => {
            // If index.html not found, return a simple HTML page with the stats UI
            Ok(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Owonero - Blockchain Stats</title>
  <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css" rel="stylesheet">
  <style>
    body { font-family: 'Roboto', sans-serif; background: #f5f5f5; }
    .hero { background: linear-gradient(135deg, #4a90e2, #50e3c2); color: white; padding: 3rem 2rem; text-align: center; }
    section { padding: 2rem 0; }
    footer { background: #1a1a1a; color: white; padding: 2rem 0; text-align: center; }
  </style>
</head>
<body>
<section class="hero">
  <div class="container">
    <h1>Owonero Blockchain Stats</h1>
    <p>Real-time statistics from the Owonero blockchain</p>
  </div>
</section>

<section id="stats">
  <div class="container">
    <h2 class="text-center mb-4">Network Statistics</h2>
    <div id="statsContainer" class="row justify-content-center">
      <div class="col-md-8">
        <div class="card p-3 mb-3">
          <h5>Block Height</h5>
          <p id="heightDisplay" class="lead">Loading...</p>
        </div>
        <div class="card p-3 mb-3">
          <h5>Network Hashrate</h5>
          <p id="hashrateDisplay" class="lead">Loading...</p>
        </div>
        <div class="card p-3 mb-3">
          <h5>Peers Connected</h5>
          <p id="peersDisplay" class="lead">Loading...</p>
        </div>
        <div id="statsError" class="alert alert-warning d-none" role="alert"></div>
      </div>
    </div>
  </div>
</section>

<footer>
  <div class="container">
    <p>&copy; 2025 Owonero. <a href="https://github.com/tosterlolz/Owonero" class="text-white">GitHub</a></p>
  </div>
</footer>

<script src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/js/bootstrap.bundle.min.js"></script>
<script>
async function pollStats() {
  try {
    const res = await fetch('/api/stats');
    if (!res.ok) throw new Error('HTTP ' + res.status);
    const stats = await res.json();
    
    document.getElementById('statsError').classList.add('d-none');
    document.getElementById('heightDisplay').textContent = (stats.height || 0) + ' blocks';
    document.getElementById('hashrateDisplay').textContent = (stats.network_hashrate || 0).toFixed(2) + ' H/s';
    document.getElementById('peersDisplay').textContent = (stats.peers ? stats.peers.length : 0) + ' peers';
  } catch (e) {
    console.error('Error fetching stats:', e);
    const err = document.getElementById('statsError');
    err.textContent = 'Failed to fetch stats: ' + e.message;
    err.classList.remove('d-none');
  }
}

// Poll every 2 seconds
pollStats();
setInterval(pollStats, 2000);
</script>
</body>
</html>"#.to_string())
        }
    }
}

pub fn create_router(daemon_addr: String) -> Router {
    let state = AppState { daemon_addr };

    Router::new()
        .route("/", get(get_index))
        .route("/index.html", get(get_index))
        .route("/stats", get(get_stats))
        .route("/api/stats", get(get_stats))
        .route("/api/wallethashrate", get(get_wallet_hashrate))
        .route("/api/walletbalance", get(get_wallet_balance))
        .with_state(state)
}

pub async fn run_http_server(port: u16, daemon_addr: String) -> anyhow::Result<()> {
    let app = create_router(daemon_addr);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("HTTP API listening on :{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
