use crate::network::{
    deserialize_packet, serialize_packet, FederationMessage, FederationPacket,
    HandshakeAckPayload, PacketBuilder,
};
use crate::tensor::{SsauTensor, TrustRegistry};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};

pub const DEFAULT_FEDERATION_PORT: u16 = 7777;
pub const HANDSHAKE_TIMEOUT_SECS: u64 = 10;
pub const MAX_PACKET_SIZE: usize = 4 * 1024 * 1024;
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Handshaking,
    Active,
    Closing,
    Failed(String),
}

pub struct PeerConnection {
    pub peer_id: String,
    pub peer_addr: SocketAddr,
    pub state: ConnectionState,
    pub stream: Arc<Mutex<TcpStream>>,
    pub connected_at: std::time::Instant,
    pub last_heartbeat: std::time::Instant,
    pub packets_rx: u64,
    pub packets_tx: u64,
}

impl PeerConnection {
    pub fn new(peer_id: String, peer_addr: SocketAddr, stream: TcpStream) -> Self {
        PeerConnection {
            peer_id,
            peer_addr,
            state: ConnectionState::Active,
            stream: Arc::new(Mutex::new(stream)),
            connected_at: std::time::Instant::now(),
            last_heartbeat: std::time::Instant::now(),
            packets_rx: 0,
            packets_tx: 0,
        }
    }

    pub async fn send_packet(&mut self, packet: &FederationPacket) -> Result<(), String> {
        let data = serialize_packet(packet).map_err(|e| e.to_string())?;
        let len = data.len() as u32;
        if len as usize > MAX_PACKET_SIZE {
            return Err(format!("Packet too large: {}", len));
        }
        let mut stream = self.stream.lock().await;
        stream.write_all(&len.to_be_bytes()).await.map_err(|e| e.to_string())?;
        stream.write_all(&data).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;
        self.packets_tx += 1;
        Ok(())
    }

    pub async fn recv_packet(&mut self) -> Result<FederationPacket, String> {
        let mut stream = self.stream.lock().await;
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.map_err(|e| format!("Read len error: {}", e))?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_PACKET_SIZE {
            return Err(format!("Packet too large: {}", len));
        }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await.map_err(|e| format!("Read payload error: {}", e))?;
        let packet = deserialize_packet(&buf).map_err(|e| e.to_string())?;
        self.packets_rx += 1;
        Ok(packet)
    }

    pub fn uptime_secs(&self) -> u64 {
        self.connected_at.elapsed().as_secs()
    }
    pub async fn recv_from_stream(stream_arc: Arc<Mutex<TcpStream>>) -> Result<FederationPacket, String> {
        use tokio::io::AsyncReadExt;
        let mut stream = stream_arc.lock().await;
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.map_err(|e| format!("Read len error: {}", e))?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_PACKET_SIZE {
            return Err(format!("Packet too large: {}", len));
        }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await.map_err(|e| format!("Read payload error: {}", e))?;
        deserialize_packet(&buf).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub node_id: String,
    pub listen_addr: SocketAddr,
    pub public_key: String,
    pub is_relay: bool,
    pub max_peers: usize,
}

impl NodeConfig {
    pub fn new(node_id: &str, port: u16) -> Self {
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        NodeConfig {
            node_id: node_id.to_string(),
            listen_addr: addr,
            public_key: format!("pubkey_{}", node_id),
            is_relay: true,
            max_peers: 50,
        }
    }
}

pub struct FederationNode {
    pub config: NodeConfig,
    pub connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
    pub ssau_table: Arc<RwLock<HashMap<String, SsauTensor>>>,
    pub trust_registry: Arc<RwLock<TrustRegistry>>,
    pub packets_processed: Arc<Mutex<u64>>,
    pub started_at: std::time::Instant,
}

impl FederationNode {
    pub fn new(config: NodeConfig) -> Arc<Self> {
        Arc::new(FederationNode {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            ssau_table: Arc::new(RwLock::new(HashMap::new())),
            trust_registry: Arc::new(RwLock::new(TrustRegistry::new())),
            packets_processed: Arc::new(Mutex::new(0)),
            started_at: std::time::Instant::now(),
        })
    }

    pub async fn start_listener(self: Arc<Self>) -> Result<(), String> {
        let listener = TcpListener::bind(&self.config.listen_addr)
            .await
            .map_err(|e| format!("Failed to bind {}: {}", self.config.listen_addr, e))?;
        log::info!("🌐 Node [{}] listening on {}", self.config.node_id, self.config.listen_addr);
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    log::info!("📡 Incoming connection from {}", peer_addr);
                    let node_clone = Arc::clone(&self);
                    tokio::spawn(async move {
                        if let Err(e) = node_clone.handle_incoming(stream, peer_addr).await {
                            log::error!("❌ Connection error {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => { log::error!("❌ Accept error: {}", e); }
            }
        }
    }

    async fn handle_incoming(self: Arc<Self>, stream: TcpStream, peer_addr: SocketAddr) -> Result<(), String> {
        let peer_addr_str = peer_addr.to_string();
        let mut conn = PeerConnection::new(format!("unknown_{}", peer_addr_str), peer_addr, stream);
        let packet = tokio::time::timeout(
            Duration::from_secs(HANDSHAKE_TIMEOUT_SECS),
            conn.recv_packet(),
        ).await.map_err(|_| "Handshake timeout".to_string())??;

        match packet.message {
            FederationMessage::Handshake(ref h) => {
                log::info!("🤝 Handshake from [{}] ({})", h.node_id, peer_addr);
                if h.protocol_version != crate::network::PROTOCOL_VERSION {
                    let reject = PacketBuilder::new(&self.config.node_id)
                        .to(&h.node_id)
                        .build(FederationMessage::HandshakeAck(HandshakeAckPayload {
                            node_id: self.config.node_id.clone(),
                            accepted: false,
                            rejection_reason: Some(format!("Incompatible protocol: {}", h.protocol_version)),
                            assigned_session_id: String::new(),
                            your_public_ip: peer_addr_str.clone(),
                        }));
                    conn.send_packet(&reject).await?;
                    return Err("Incompatible protocol".to_string());
                }
                let session_id = uuid::Uuid::new_v4().to_string();
                let ack = PacketBuilder::new(&self.config.node_id)
                    .to(&h.node_id)
                    .build(FederationMessage::HandshakeAck(HandshakeAckPayload {
                        node_id: self.config.node_id.clone(),
                        accepted: true,
                        rejection_reason: None,
                        assigned_session_id: session_id,
                        your_public_ip: peer_addr_str,
                    }));
                conn.send_packet(&ack).await?;
                conn.peer_id = h.node_id.clone();
                conn.state = ConnectionState::Active;
                log::info!("✅ Handshake complete! Peer [{}] active.", conn.peer_id);
                let peer_id = conn.peer_id.clone();
                self.connections.write().await.insert(peer_id.clone(), conn);
                self.peer_message_loop(peer_id).await;
                Ok(())
            }
            _ => Err("First packet must be Handshake".to_string()),
        }
    }

    pub async fn connect_to_peer(self: Arc<Self>, peer_addr: &str) -> Result<String, String> {
        log::info!("[{}] 🔌 Connecting to {}...", self.config.node_id, peer_addr);
        let stream = tokio::time::timeout(
            Duration::from_secs(HANDSHAKE_TIMEOUT_SECS),
            TcpStream::connect(peer_addr),
        ).await.map_err(|_| format!("Connection timeout: {}", peer_addr))?
         .map_err(|e| format!("TCP error: {}", e))?;

        let addr: SocketAddr = stream.peer_addr().unwrap();
        let mut conn = PeerConnection::new(format!("pending_{}", peer_addr), addr, stream);

        let handshake = crate::network::create_handshake_packet(
            &self.config.node_id, "unknown",
            &self.config.public_key,
            self.connections.read().await.len() as u32,
        );
        conn.send_packet(&handshake).await?;
        log::info!("[{}] 📤 Handshake sent", self.config.node_id);

        let ack_packet = tokio::time::timeout(
            Duration::from_secs(HANDSHAKE_TIMEOUT_SECS),
            conn.recv_packet(),
        ).await.map_err(|_| "HandshakeAck timeout".to_string())??;

        match ack_packet.message {
            FederationMessage::HandshakeAck(ref ack) => {
                if !ack.accepted {
                    return Err(format!("Handshake rejected: {}",
                        ack.rejection_reason.as_deref().unwrap_or("no reason")));
                }
                let peer_id = ack.node_id.clone();
                log::info!("[{}] ✅ Connected to [{}]! Session: {}", self.config.node_id, peer_id, ack.assigned_session_id);
                conn.peer_id = peer_id.clone();
                conn.state = ConnectionState::Active;
                self.connections.write().await.insert(peer_id.clone(), conn);
                let node = Arc::clone(&self);
                let pid = peer_id.clone();
                tokio::spawn(async move { node.peer_message_loop(pid).await; });
                Ok(peer_id)
            }
            _ => Err("Expected HandshakeAck".to_string()),
        }
    }

    async fn peer_message_loop(self: Arc<Self>, peer_id: String) {
        log::info!("[{}] 🔄 Message loop started for [{}]", self.config.node_id, peer_id);
        loop {
            // Клонируем Arc<Mutex<TcpStream>> без удержания connections lock
            let stream_arc = {
                let conns = self.connections.read().await;
                match conns.get(&peer_id) {
                    Some(conn) => conn.stream.clone(),
                    None => break,
                }
            };
            // Читаем пакет без удержания connections lock
            let packet = PeerConnection::recv_from_stream(stream_arc).await;
            match packet {
                Ok(p) => {
                    *self.packets_processed.lock().await += 1;
                    self.clone().dispatch_message(p).await;
                }
                Err(e) => {
                    log::warn!("[{}] ⚠️ Peer [{}] disconnected: {}", self.config.node_id, peer_id, e);
                    self.trust_registry.write().await.penalize_unreachable(&peer_id);
                    self.connections.write().await.remove(&peer_id);
                    break;
                }
            }
        }
        log::info!("[{}] Message loop for [{}] ended", self.config.node_id, peer_id);
    }

    async fn dispatch_message(self: Arc<Self>, packet: FederationPacket) {
        match &packet.message {
            FederationMessage::Heartbeat(hb) => {
                log::info!("💓 Heartbeat from [{}] uptime={}s load={:.2}",
                    hb.node_id, hb.uptime_seconds, hb.load_factor);
            }
            FederationMessage::SsauUpdate(update) => {
                log::info!("📊 SSAU Update from [{}]: {} tensors",
                    update.reporter_node_id, update.tensors.len());
                let mut table = self.ssau_table.write().await;
                for t_msg in &update.tensors {
                    let key = format!("{}→{}", t_msg.from_node, t_msg.to_node);
                    use crate::tensor::LatencyDistribution;
                    let tensor = SsauTensor {
                        from_node: t_msg.from_node.clone(),
                        to_node: t_msg.to_node.clone(),
                        latency: LatencyDistribution {
                            mean: t_msg.latency_mean_ms,
                            std_dev: t_msg.latency_std_dev_ms,
                            samples: vec![],
                        },
                        jitter: t_msg.jitter_ms,
                        bandwidth: t_msg.bandwidth_mbps,
                        reliability: t_msg.reliability,
                        energy_cost: t_msg.energy_cost,
                        updated_at: 0,
                        version: t_msg.version,
                    };
                    table.insert(key, tensor);
                }
            }
            FederationMessage::Goodbye { node_id, reason } => {
                log::info!("👋 Node [{}] leaving: {}", node_id, reason);
                self.connections.write().await.remove(node_id);
            }
            _ => {}
        }
    }

    pub async fn start_heartbeat_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        log::info!("[{}] 💓 Heartbeat loop started", self.config.node_id);
        loop {
            ticker.tick().await;
            let uptime = self.started_at.elapsed().as_secs();
            let peer_count = self.connections.read().await.len();
            let hb = PacketBuilder::new(&self.config.node_id).build(
                FederationMessage::Heartbeat(crate::network::HeartbeatPayload {
                    node_id: self.config.node_id.clone(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    uptime_seconds: uptime,
                    load_factor: peer_count as f64 / self.config.max_peers as f64,
                }),
            );
            let peer_ids: Vec<String> = self.connections.read().await.keys().cloned().collect();
            for peer_id in peer_ids {
                let mut conns = self.connections.write().await;
                if let Some(conn) = conns.get_mut(&peer_id) {
                    if let Err(e) = conn.send_packet(&hb).await {
                        log::warn!("Heartbeat failed for {}: {}", peer_id, e);
                    }
                }
            }
        }
    }

    pub async fn status(&self) -> NodeStatus {
        let conns = self.connections.read().await;
        let ssau = self.ssau_table.read().await;
        let processed = *self.packets_processed.lock().await;
        let trust = self.trust_registry.read().await;
        NodeStatus {
            node_id: self.config.node_id.clone(),
            listen_addr: self.config.listen_addr.to_string(),
            active_peers: conns.len(),
            peer_ids: conns.keys().cloned().collect(),
            ssau_entries: ssau.len(),
            packets_processed: processed,
            uptime_seconds: self.started_at.elapsed().as_secs(),
            trust_stats: trust.stats(),
        }
    }
}

#[derive(Debug)]
pub struct NodeStatus {
    pub node_id: String,
    pub listen_addr: String,
    pub active_peers: usize,
    pub peer_ids: Vec<String>,
    pub ssau_entries: usize,
    pub packets_processed: u64,
    pub uptime_seconds: u64,
    pub trust_stats: String,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "═══════════════════════════════════════\n\
             FEDERATION NODE STATUS\n\
             ═══════════════════════════════════════\n\
             ID:       {}\n\
             Addr:     {}\n\
             Peers:    {} active: {:?}\n\
             SSAU:     {} tensors\n\
             Packets:  {} processed\n\
             Uptime:   {}s\n\
             Trust:    {}\n\
             ═══════════════════════════════════════",
            self.node_id, self.listen_addr, self.active_peers, self.peer_ids,
            self.ssau_entries, self.packets_processed, self.uptime_seconds, self.trust_stats)
    }
}
