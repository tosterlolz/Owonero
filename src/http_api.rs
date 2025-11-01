use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct AppState {
    pub daemon_addr: String,
}

#[derive(Serialize, Deserialize)]
pub struct WalletBalanceQuery {
    pub addr: String,
}

pub async fn get_stats(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let mut response = json!({});

    // Get chain info (includes height)
    if let Ok(chain) = crate::ws_client::fetch_chain(&state.daemon_addr).await {
        response["height"] = json!(chain.chain.len().saturating_sub(1));
    } else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Get latest block
    if let Ok(latest) = crate::ws_client::fetch_latest_block(&state.daemon_addr).await {
        match serde_json::to_value(&latest) {
            Ok(val) => response["latest_block"] = val,
            Err(_) => {}
        }
    }

    // Get mempool
    if let Ok(mempool) = crate::ws_client::fetch_mempool(&state.daemon_addr).await {
        let mempool_vals: Vec<Value> = mempool
            .iter()
            .filter_map(|tx| serde_json::to_value(tx).ok())
            .collect();
        response["mempool"] = json!(mempool_vals);
        response["mempool_size"] = json!(mempool.len());
    }

    Ok(Json(response))
}

pub async fn get_wallet_balance(
    State(state): State<AppState>,
    Query(query): Query<WalletBalanceQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Fetch blockchain and calculate balance for wallet
    match crate::ws_client::fetch_chain(&state.daemon_addr).await {
        Ok(chain) => {
            let wallet = crate::wallet::Wallet {
                address: query.addr.clone(),
                pub_key: String::new(),
                priv_key: String::new(),
                node_address: None,
            };
            let balance = wallet.get_balance(&chain);
            let balance_owe = (balance as f64) / 1000.0;

            Ok(Json(json!({
                "wallet": query.addr,
                "balance_milli": balance,
                "balance": balance_owe,
                "currency": "OWE"
            })))
        }
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

pub async fn get_chain(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match crate::ws_client::fetch_chain(&state.daemon_addr).await {
        Ok(chain) => {
            match serde_json::to_value(&chain) {
                Ok(val) => Ok(Json(val)),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

pub fn create_router(daemon_addr: String) -> Router {
    let state = AppState { daemon_addr };

    Router::new()
        .route("/stats", get(get_stats))
        .route("/api/stats", get(get_stats))
        .route("/api/chain", get(get_chain))
        .route("/api/walletbalance", get(get_wallet_balance))
        .with_state(state)
}

pub async fn run_http_server(port: u16, daemon_addr: String) -> anyhow::Result<()> {
    let app = create_router(daemon_addr);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Stats server listening on :{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
