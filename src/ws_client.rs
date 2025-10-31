use anyhow::anyhow;
use futures::SinkExt;
use futures::stream::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Connect to a WebSocket server and send a JSON-RPC-like command, returning the response.
pub async fn ws_command(
    addr: &str,
    method: &str,
    params: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let url = format!("ws://{}", addr);
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Skip greeting if present
    if let Some(greeting_msg) = read.next().await {
        if let Ok(Message::Text(text)) = greeting_msg {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if json.get("type").and_then(|t| t.as_str()) == Some("greeting") {
                    // Greeting received and skipped, continue
                } else {
                    // Not a greeting, we need to parse it as a response
                    // This shouldn't happen in normal flow, but just in case
                }
            }
        }
    }

    // Send command
    let cmd = serde_json::json!({"method": method, "params": params});
    write.send(Message::Text(cmd.to_string())).await?;

    // Read response
    if let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => serde_json::from_str(&text)
                .map_err(|e| anyhow!("Failed to parse WebSocket response: {}", e)),
            _ => Err(anyhow!("Unexpected WebSocket message type")),
        }
    } else {
        Err(anyhow!("No response from server"))
    }
}

/// Convenience: fetch chain from node via WebSocket
pub async fn fetch_chain(node_addr: &str) -> anyhow::Result<crate::blockchain::Blockchain> {
    let resp = ws_command(node_addr, "getchain", serde_json::json!({})).await?;
    if let Some(data) = resp.get("data") {
        serde_json::from_value(data.clone()).map_err(|e| anyhow!("Failed to parse chain: {}", e))
    } else {
        Err(anyhow!("No data in response"))
    }
}

/// Convenience: fetch latest block from node via WebSocket
pub async fn fetch_latest_block(node_addr: &str) -> anyhow::Result<crate::blockchain::Block> {
    let resp = ws_command(node_addr, "getlatest", serde_json::json!({})).await?;
    if let Some(data) = resp.get("data") {
        serde_json::from_value(data.clone()).map_err(|e| anyhow!("Failed to parse block: {}", e))
    } else {
        Err(anyhow!("No data in response"))
    }
}

/// Convenience: fetch mempool from node via WebSocket
pub async fn fetch_mempool(node_addr: &str) -> anyhow::Result<Vec<crate::blockchain::Transaction>> {
    let resp = ws_command(node_addr, "getmempool", serde_json::json!({})).await?;
    if let Some(data) = resp.get("data") {
        serde_json::from_value(data.clone()).map_err(|e| anyhow!("Failed to parse mempool: {}", e))
    } else {
        Err(anyhow!("No data in response"))
    }
}

/// Convenience: submit transaction to node via WebSocket
pub async fn submit_tx(
    node_addr: &str,
    tx: &crate::blockchain::Transaction,
) -> anyhow::Result<String> {
    let resp = ws_command(node_addr, "submittx", serde_json::json!({"tx": tx})).await?;
    if let Some(msg) = resp.get("message") {
        Ok(msg.as_str().unwrap_or("error").to_string())
    } else if let Some(status) = resp.get("status") {
        Ok(status.as_str().unwrap_or("error").to_string())
    } else if resp.get("type").and_then(|t| t.as_str()) == Some("error") {
        Err(anyhow!("Transaction rejected"))
    } else {
        Ok("ok".to_string())
    }
}

/// Convenience: submit block to node via WebSocket
pub async fn submit_block(
    node_addr: &str,
    block: &crate::blockchain::Block,
) -> anyhow::Result<String> {
    let resp = ws_command(
        node_addr,
        "submitblock",
        serde_json::json!({"block": block}),
    )
    .await?;
    if let Some(msg) = resp.get("message") {
        Ok(msg.as_str().unwrap_or("error").to_string())
    } else if let Some(status) = resp.get("status") {
        Ok(status.as_str().unwrap_or("error").to_string())
    } else if resp.get("type").and_then(|t| t.as_str()) == Some("error") {
        Err(anyhow!("Block rejected"))
    } else {
        Ok("ok".to_string())
    }
}
