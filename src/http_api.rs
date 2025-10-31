use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct AppState {
    pub daemon_addr: String,
}

// #[derive(Serialize, Deserialize)]
// pub struct StatsResponse {
//     pub chain: Option<Value>,
//     pub height: Option<u64>,
//     pub peers: Option<Vec<String>>,
//     pub latest_block: Option<Value>,
//     pub mempool: Option<Vec<Value>>,
//     pub network_hashrate: Option<f64>,
// }

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
    writer
        .write_all(format!("{}\n", command).as_bytes())
        .await?;

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
            if let Some(hr) = hashrate_obj
                .get("network_hashrate")
                .and_then(|v| v.as_f64())
            {
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
        Ok(response_str) => match serde_json::from_str::<WalletHashrateResponse>(&response_str) {
            Ok(resp) => Ok(Json(resp)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
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
                Ok(chain_obj) => {
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

// Handler for /api/chain: returns the full blockchain as JSON

pub async fn get_chain(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match connect_to_daemon(&state.daemon_addr, "getchain").await {
        Ok(chain_str) => match serde_json::from_str::<Value>(&chain_str) {
            Ok(chain_obj) => Ok(Json(chain_obj)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub fn create_router(daemon_addr: String) -> Router {
    let state = AppState { daemon_addr };

    Router::new()
        .route("/stats", get(get_stats))
        .route("/api/stats", get(get_stats))
        .route("/api/chain", get(get_chain))
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
