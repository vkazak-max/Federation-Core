cat > src/network.rs << 'EOF'
use crate::tensor::SsauTensor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum FederationMessage {
    Handshake(HandshakePayload),
    HandshakeAck(HandshakeAckPayload),
    SsauUpdate(SsauUpdatePayload),
    RouteRequest(RouteRequestPayload),
    RouteResponse(RouteResponsePayload),
    Heartbeat(HeartbeatPayload),
    Goodbye { node_id: String, reason: String },
    NodeDiscovered(NodeInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandshakePayload {
    pub node_id: String,
    pub protocol_version: u8,
    pub public_key: String,
    pub capabilities: NodeCapabilities,
    pub timestamp: i64,
    pub known_peers_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandshakeAckPayload {
    pub node_id: String,
    pub accepted: bool,
    pub rejection_reason: Option<String>,
    pub assigned_session_id: String,
    pub your_public_ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeCapabilities {
    pub is_relay: bool,
    pub max_bandwidth_mbps: u32,
    pub supports_storage: bool,
    pub supports_consensus: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SsauUpdatePayload {
    pub reporter_node_id: String,
    pub tensors: Vec<SsauTensorMessage>,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SsauTensorMessage {
    pub from_node: String,
    pub to_node: String,
    pub latency_mean_ms: f64,
    pub latency_std_dev_ms: f64,
    pub jitter_ms: f64,
    pub bandwidth_mbps: f64,
    pub reliability: f64,
    pub energy_cost: f64,
    pub version: u64,
}

impl From<&SsauTensor> for SsauTensorMessage {
    fn from(t: &SsauTensor) -> Self {
        SsauTensorMessage {
            from_node: t.from_node.clone(),
            to_node: t.to_node.clone(),
            latency_mean_ms: t.latency.mean,
            latency_std_dev_ms: t.latency.std_dev,
            jitter_ms: t.jitter,
            bandwidth_mbps: t.bandwidth,
            reliability: t.reliability,
            energy_cost: t.energy_cost,
            version: t.version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteRequestPayload {
    pub request_id: String,
    pub destination_node_id: String,
    pub priority_latency: f64,
    pub priority_anonymity: f64,
    pub priority_cost: f64,
    pub max_latency_ms: Option<f64>,
    pub ttl: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteResponsePayload {
    pub request_id: String,
    pub path: Vec<String>,
    pub total_latency_ms: f64,
    pub stability_score: f64,
    pub cost: f64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeartbeatPayload {
    pub node_id: String,
    pub timestamp: i64,
    pub uptime_seconds: u64,
    pub load_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeInfo {
    pub node_id: String,
    pub address: String,
    pub public_key: String,
    pub trust_weight: f64,
    pub capabilities: NodeCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPacket {
    pub header: PacketHeader,
    pub message: FederationMessage,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketHeader {
    pub packet_id: String,
    pub sender_id: String,
    pub recipient_id: Option<String>,
    pub created_at: i64,
    pub protocol_version: u8,
    pub ttl: u8,
    pub payload_size: u32,
}

pub struct PacketBuilder {
    sender_id: String,
    recipient_id: Option<String>,
    ttl: u8,
}

impl PacketBuilder {
    pub fn new(sender_id: &str) -> Self {
        Self { sender_id: sender_id.to_string(), recipient_id: None, ttl: 64 }
    }

    pub fn to(mut self, recipient_id: &str) -> Self {
        self.recipient_id = Some(recipient_id.to_string());
        self
    }

    pub fn with_ttl(mut self, ttl: u8) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn build(self, message: FederationMessage) -> FederationPacket {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let payload_json = serde_json::to_string(&message).unwrap_or_default();
        let payload_size = payload_json.len() as u32;
        FederationPacket {
            header: PacketHeader {
                packet_id: Uuid::new_v4().to_string(),
                sender_id: self.sender_id,
                recipient_id: self.recipient_id,
                created_at: now,
                protocol_version: PROTOCOL_VERSION,
                ttl: self.ttl,
                payload_size,
            },
            message,
            signature: "placeholder_sig_v1".to_string(),
        }
    }
}

pub fn serialize_packet(packet: &FederationPacket) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(packet)
}

pub fn deserialize_packet(bytes: &[u8]) -> Result<FederationPacket, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub type PacketSender = mpsc::Sender<FederationPacket>;
pub type PacketReceiver = mpsc::Receiver<FederationPacket>;

pub struct NodeContext {
    pub node_id: String,
    pub inbox: PacketReceiver,
    pub outbox: PacketSender,
    pub peers: HashMap<String, PacketSender>,
    pub packets_processed: u64,
    pub packets_sent: u64,
}

impl NodeContext {
    pub fn new(node_id: &str) -> (Self, PacketSender) {
        let (inbox_tx, inbox_rx) = mpsc::channel::<FederationPacket>(256);
        let (outbox_tx, _outbox_rx) = mpsc::channel::<FederationPacket>(256);
        let ctx = NodeContext {
            node_id: node_id.to_string(),
            inbox: inbox_rx,
            outbox: outbox_tx,
            peers: HashMap::new(),
            packets_processed: 0,
            packets_sent: 0,
        };
        (ctx, inbox_tx)
    }

    pub fn add_peer(&mut self, peer_id: &str, sender: PacketSender) {
        self.peers.insert(peer_id.to_string(), sender);
    }

    pub async fn send_to_peer(&mut self, peer_id: &str, packet: FederationPacket) -> Result<(), String> {
        match self.peers.get(peer_id) {
            Some(sender) => {
                sender.send(packet).await.map_err(|e| format!("Send error: {}", e))?;
                self.packets_sent += 1;
                Ok(())
            }
            None => Err(format!("Peer {} not found", peer_id)),
        }
    }

    pub async fn broadcast(&mut self, packet: FederationPacket) -> usize {
        let mut sent = 0;
        let peer_ids: Vec<String> = self.peers.keys().cloned().collect();
        for peer_id in peer_ids {
            let mut p = packet.clone();
            p.header.recipient_id = Some(peer_id.clone());
            if let Some(sender) = self.peers.get(&peer_id) {
                if sender.send(p).await.is_ok() {
                    sent += 1;
                    self.packets_sent += 1;
                }
            }
        }
        sent
    }

    pub async fn process_next(&mut self) -> Option<FederationPacket> {
        match self.inbox.try_recv() {
            Ok(packet) => { self.packets_processed += 1; Some(packet) }
            Err(_) => None,
        }
    }
}

pub fn create_handshake_packet(our_node_id: &str, their_node_id: &str, public_key: &str, known_peers: u32) -> FederationPacket {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    PacketBuilder::new(our_node_id).to(their_node_id).build(FederationMessage::Handshake(HandshakePayload {
        node_id: our_node_id.to_string(),
        protocol_version: PROTOCOL_VERSION,
        public_key: public_key.to_string(),
        capabilities: NodeCapabilities {
            is_relay: true,
            max_bandwidth_mbps: 100,
            supports_storage: false,
            supports_consensus: false,
        },
        timestamp: now,
        known_peers_count: known_peers,
    }))
}

pub fn create_ssau_update_packet(our_node_id: &str, tensors: &[&SsauTensor], sequence: u64) -> FederationPacket {
    let tensor_messages: Vec<SsauTensorMessage> = tensors.iter().map(|t| SsauTensorMessage::from(*t)).collect();
    PacketBuilder::new(our_node_id).build(FederationMessage::SsauUpdate(SsauUpdatePayload {
        reporter_node_id: our_node_id.to_string(),
        tensors: tensor_messages,
        sequence,
    }))
}
EOF
