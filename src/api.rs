use axum::{routing::get, Router, Json};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::p2p::FederationNode;

#[derive(Serialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub status: String,
    pub listen_addr: String,
    pub active_peers: usize,
    pub version: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkNodeInfo {
    pub node_id: String,
    pub status: String,
    pub location: String,
    pub active_peers: usize,
    pub version: String,
    pub timestamp: String,
    pub online: bool,
}

#[derive(Serialize)]
pub struct NetworkStatus {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub version: String,
    pub nodes: Vec<NetworkNodeInfo>,
    pub timestamp: String,
}

// Список известных узлов сети
fn known_nodes() -> Vec<(&'static str, &'static str, &'static str)> {
    // (node_id, api_url, location)
    vec![
        ("nexus-core-01", "http://localhost:8080/v1/status", "Nuremberg, DE"),
        ("nexus-core-02", "http://77.42.80.137:8080/v1/status", "Helsinki, FI"),
        ("nexus-core-03", "http://5.223.45.7:8080/v1/status",   "Singapore, SG"),
        ("nexus-core-04", "http://178.156.248.31:8080/v1/status", "Ashburn, US"),
        ("nexus-core-05", "http://5.78.182.12:8080/v1/status",    "Hillsboro, US"),
        ("nexus-core-06", "http://167.71.65.20:8080/v1/status",  "Amsterdam, NL"),
    ]
}

pub async fn status_handler(
    axum::extract::State(node): axum::extract::State<Arc<FederationNode>>
) -> Json<NodeStatus> {
    let status = node.status().await;
    Json(NodeStatus {
        node_id: std::env::var("NODE_ID").unwrap_or_else(|_| status.node_id),
        status: "active".to_string(),
        listen_addr: status.listen_addr,
        active_peers: status.active_peers,
        version: "1.0.0-alpha".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

pub async fn network_handler() -> Json<NetworkStatus> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .unwrap();

    let mut nodes = Vec::new();
    for (node_id, url, location) in known_nodes() {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<serde_json::Value>().await {
                    Ok(data) => nodes.push(NetworkNodeInfo {
                        node_id: data["node_id"].as_str().unwrap_or(node_id).to_string(),
                        status: data["status"].as_str().unwrap_or("unknown").to_string(),
                        location: location.to_string(),
                        active_peers: data["active_peers"].as_u64().unwrap_or(0) as usize,
                        version: data["version"].as_str().unwrap_or("unknown").to_string(),
                        timestamp: data["timestamp"].as_str().unwrap_or("").to_string(),
                        online: true,
                    }),
                    Err(_) => nodes.push(NetworkNodeInfo {
                        node_id: node_id.to_string(),
                        status: "error".to_string(),
                        location: location.to_string(),
                        active_peers: 0,
                        version: "unknown".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        online: false,
                    }),
                }
            }
            _ => nodes.push(NetworkNodeInfo {
                node_id: node_id.to_string(),
                status: "offline".to_string(),
                location: location.to_string(),
                active_peers: 0,
                version: "unknown".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                online: false,
            }),
        }
    }

    let online = nodes.iter().filter(|n| n.online).count();
    Json(NetworkStatus {
        total_nodes: nodes.len(),
        online_nodes: online,
        version: "1.0.0-alpha".to_string(),
        nodes,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

pub fn router(node: Arc<FederationNode>) -> Router {
    Router::new()
        .route("/v1/status", get(status_handler))
        .route("/v1/peers", get(peers_handler))
        .route("/v1/network", get(network_handler))
        .with_state(node)
}

async fn peers_handler(
    axum::extract::State(node): axum::extract::State<Arc<FederationNode>>
) -> Json<serde_json::Value> {
    let status = node.status().await;
    Json(serde_json::json!({
        "peers": status.peer_ids,
        "count": status.active_peers
    }))
}
