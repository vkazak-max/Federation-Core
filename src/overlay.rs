// =============================================================================
// FEDERATION CORE — overlay.rs
// PHASE 2 / WEEK 8 — «Overlay MVP»
// =============================================================================
//
// Реализует MVP-оверлей:
//   1) BootstrapManager — подключение к seed-узлам
//   2) SSAU broadcast loop — реальные рассылки SsauUpdate по TCP peers
//   3) Node discovery loop — обмен NodeDiscovered
//   4) Router audit loop — мониторинг энтропии и нестабильных маршрутов
//   5) FederationMVP — собирает всё вместе
//
// Важно:
//   - полноценной onion-relay пересылки здесь ещё нет, т.к. в network.rs пока нет
//     сообщения для передачи OnionPacket. Сейчас send_onion() строит пакет + anti-replay.
// =============================================================================

use crate::dag::FederationDag;
use crate::mirage::MirageNode;
use crate::network::{
    create_ssau_update_packet, FederationMessage, NodeCapabilities, NodeInfo, PacketBuilder,
};
use crate::p2p::{FederationNode, NodeConfig};
use crate::routing::{build_route_candidates, AiRouter, UserPriorities};
use crate::tensor::{SsauTensor, TrustRegistry};
use crate::zkp::{OnionBuilder, NullifierSet};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, sleep, Duration};

// -----------------------------------------------------------------------------
// Константы
// -----------------------------------------------------------------------------

pub const PEER_EXCHANGE_INTERVAL_SECS: u64 = 60;
pub const SSAU_BROADCAST_INTERVAL_SECS: u64 = 15;
pub const ROUTE_AUDIT_INTERVAL_SECS: u64 = 30;

pub const MAX_SEED_NODES: usize = 8;

// -----------------------------------------------------------------------------
// SeedNode — известный узел для bootstrap
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedNode {
    pub address: String,   // host:port
    pub node_id: String,
    pub public_key: String,
    pub region: String,
}

impl SeedNode {
    pub fn new(address: &str, node_id: &str, region: &str) -> Self {
        SeedNode {
            address: address.to_string(),
            node_id: node_id.to_string(),
            public_key: format!("pubkey_{}", node_id),
            region: region.to_string(),
        }
    }
}

/// Дефолтный список seed’ов (если не задан FEDERATION_SEEDS)
pub fn default_seed_nodes() -> Vec<SeedNode> {
    vec![SeedNode::new("78.47.246.100:7777", "nexus-core-01", "EU-DE")]
}

/// Чтение seed’ов из env.
/// Формат:
///   FEDERATION_SEEDS="host:port,node_id,region;host:port,node_id,region"
pub fn seeds_from_env() -> Vec<SeedNode> {
    let raw = std::env::var("FEDERATION_SEEDS").unwrap_or_default();
    if raw.trim().is_empty() {
        return default_seed_nodes();
    }

    let mut out = Vec::new();
    for item in raw.split(';') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let parts: Vec<&str> = item.split(',').map(|s| s.trim()).collect();
        if parts.len() < 3 {
            log::warn!("FEDERATION_SEEDS entry ignored (need 3 parts): {}", item);
            continue;
        }
        out.push(SeedNode::new(parts[0], parts[1], parts[2]));
        if out.len() >= MAX_SEED_NODES {
            break;
        }
    }

    if out.is_empty() {
        default_seed_nodes()
    } else {
        out
    }
}

// -----------------------------------------------------------------------------
// BootstrapManager
// -----------------------------------------------------------------------------

pub struct BootstrapManager {
    pub seeds: Vec<SeedNode>,
    pub connected_seeds: Vec<String>,
    pub bootstrap_complete: bool,
}

impl BootstrapManager {
    pub fn new(seeds: Vec<SeedNode>) -> Self {
        BootstrapManager {
            seeds,
            connected_seeds: vec![],
            bootstrap_complete: false,
        }
    }

    pub async fn bootstrap(&mut self, node: Arc<FederationNode>) -> usize {
        let mut connected = 0;

        for seed in &self.seeds {
            log::info!("🌱 Bootstrap: connect seed {} ({})", seed.node_id, seed.address);

            match tokio::time::timeout(
                Duration::from_secs(5),
                node.clone().connect_to_peer(&seed.address),
            )
            .await
            {
                Ok(Ok(peer_id)) => {
                    log::info!("✅ Bootstrap: connected to {}", peer_id);
                    self.connected_seeds.push(peer_id);
                    connected += 1;
                }
                Ok(Err(e)) => {
                    log::warn!("⚠️ Bootstrap: failed {}: {}", seed.node_id, e);
                }
                Err(_) => {
                    log::warn!("⚠️ Bootstrap: timeout {}", seed.node_id);
                }
            }
        }

        self.bootstrap_complete = connected > 0;
        connected
    }
}

// -----------------------------------------------------------------------------
// OverlayStats
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayStats {
    pub node_id: String,
    pub uptime_secs: u64,
    pub active_peers: usize,
    pub known_nodes: usize,
    pub ssau_tensors: usize,
    pub dag_nodes: usize,
    pub dag_total_rewards: f64,
    pub routes_computed: u64,
    pub packets_onion_wrapped: u64,
    pub mirage_activations: u64,
    pub nullifiers_seen: usize,
    pub avg_route_latency_ms: f64,
    pub network_health: f64,
}

impl std::fmt::Display for OverlayStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "╔══════════════════════════════════════════════════════════════╗\n\
             ║  FEDERATION OVERLAY MVP — NODE STATUS                        ║\n\
             ╠══════════════════════════════════════════════════════════════╣\n\
             ║  ID:        {:<50} ║\n\
             ║  Uptime:    {:<8}s  Peers: {:<6}  Known: {:<6}           ║\n\
             ║  SSAU:      {:<6} тензоров  DAG: {:<6} вершин             ║\n\
             ║  Rewards:   {:<10.4} монет  Health: {:.3}               ║\n\
             ║  Routes:    {:<10}  Onion: {:<10}                     ║\n\
             ║  Mirage:    {:<6} активаций  Nullifiers: {:<6}            ║\n\
             ║  Avg route: {:<8.1}ms                                      ║\n\
             ╚══════════════════════════════════════════════════════════════╝",
            self.node_id,
            self.uptime_secs,
            self.active_peers,
            self.known_nodes,
            self.ssau_tensors,
            self.dag_nodes,
            self.dag_total_rewards,
            self.network_health,
            self.routes_computed,
            self.packets_onion_wrapped,
            self.mirage_activations,
            self.nullifiers_seen,
            self.avg_route_latency_ms,
        )
    }
}

// -----------------------------------------------------------------------------
// FederationMVP
// -----------------------------------------------------------------------------

pub struct FederationMVP {
    pub node: Arc<FederationNode>,
    pub dag: Arc<Mutex<FederationDag>>,
    pub router: Arc<Mutex<AiRouter>>,
    pub trust: Arc<RwLock<TrustRegistry>>,
    pub mirage: Arc<Mutex<MirageNode>>,
    pub nullifiers: Arc<Mutex<NullifierSet>>,
    pub known_nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,

    pub routes_computed: Arc<Mutex<u64>>,
    pub packets_onion_wrapped: Arc<Mutex<u64>>,

    pub started_at: std::time::Instant,
}

impl FederationMVP {
    pub fn new(node_id: &str, port: u16) -> Arc<Self> {
        let config = NodeConfig::new(node_id, port);
        let node = FederationNode::new(config);

        Arc::new(FederationMVP {
            node,
            dag: Arc::new(Mutex::new(FederationDag::new())),
            router: Arc::new(Mutex::new(AiRouter::new())),
            trust: Arc::new(RwLock::new(TrustRegistry::new())),
            mirage: Arc::new(Mutex::new(MirageNode::new(node_id))),
            nullifiers: Arc::new(Mutex::new(NullifierSet::new())),
            known_nodes: Arc::new(RwLock::new(HashMap::new())),
            routes_computed: Arc::new(Mutex::new(0)),
            packets_onion_wrapped: Arc::new(Mutex::new(0)),
            started_at: std::time::Instant::now(),
        })
    }

    // -------------------------------------------------------------------------
    // START
    // -------------------------------------------------------------------------

    /// Запустить узел. Если seeds пустой — возьмём из env/дефолта.
    pub async fn start(self: Arc<Self>, seeds: Vec<SeedNode>) {
        let node_id = self.node.config.node_id.clone();
        let port = self.node.config.listen_addr.port();

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  🚀 FEDERATION MVP NODE STARTING                             ║");
        println!("║  ID: {:<56} ║", node_id);
        println!("║  Port: {:<54} ║", port);
        println!("╚══════════════════════════════════════════════════════════════╝\n");

        // 1) TCP listener
        {
            let n = Arc::clone(&self.node);
            tokio::spawn(async move {
                let _ = n.start_listener().await;
            });
        }

        // 2) Heartbeat loop
        {
            let n = Arc::clone(&self.node);
            tokio::spawn(async move {
                n.start_heartbeat_loop().await;
            });
        }

        sleep(Duration::from_millis(150)).await;
        println!("✅ TCP listener запущен на порту {}", port);

        // 3) Bootstrap
        let seeds = if seeds.is_empty() { seeds_from_env() } else { seeds };

        if !seeds.is_empty() {
            println!("🌱 Bootstrap: {} seed-узлов...", seeds.len());
            let mut bootstrap = BootstrapManager::new(seeds);
            let connected = bootstrap.bootstrap(Arc::clone(&self.node)).await;
            println!("✅ Bootstrap: подключились к {} seed-узлам", connected);
        }

        // 4) SSAU broadcast loop (реальная рассылка)
        {
            let mvp = Arc::clone(&self);
            tokio::spawn(async move {
                mvp.ssau_broadcast_loop().await;
            });
        }

        // 5) Node discovery loop
        {
            let mvp = Arc::clone(&self);
            tokio::spawn(async move {
                mvp.node_discovery_loop().await;
            });
        }

        // 6) Route audit loop
        {
            let mvp = Arc::clone(&self);
            tokio::spawn(async move {
                mvp.route_audit_loop().await;
            });
        }

        // 7) Status loop
        {
            let mvp = Arc::clone(&self);
            tokio::spawn(async move {
                let mut ticker = interval(Duration::from_secs(10));
                loop {
                    ticker.tick().await;
                    let stats = mvp.collect_stats().await;
                    println!("{}", stats);
                }
            });
        }

        println!("\n✅ Все подсистемы запущены:");
        println!("   Phase 1: SSAU Tensor ✓  Packet Protocol ✓  TCP P2P ✓  AI Router ✓");
        println!("   Phase 2: DAG Consensus ✓  ZKP Onion ✓  Active Mimicry ✓");
        println!("\n⏳ Узел работает. Ctrl+C для остановки.\n");

        loop {
            sleep(Duration::from_secs(60)).await;
        }
    }

    // -------------------------------------------------------------------------
    // Operations
    // -------------------------------------------------------------------------

    /// MVP: строит onion + anti-replay.
    /// Пересылка по сети появится, когда добавим сообщение OnionRelay в network.rs.
    pub async fn send_onion(self: Arc<Self>, route: Vec<String>, payload: &[u8]) -> Result<String, String> {
        if route.len() < 2 {
            return Err("Маршрут должен содержать минимум 2 узла".to_string());
        }

        let (packet, _keys) = OnionBuilder::new()
            .with_route(route.clone())
            .build(payload)
            .map_err(|e| e.to_string())?;

        // anti-replay
        let nullifier = packet.outer_layer.nullifier.clone();
        {
            let mut nulls = self.nullifiers.lock().await;
            if !nulls.check_and_add(&nullifier) {
                return Err("Replay attack detected — пакет отброшен".to_string());
            }
        }

        *self.packets_onion_wrapped.lock().await += 1;

        Ok(format!(
            "Onion пакет собран: слоёв={}, маршрут={:?}, nullifier={}",
            packet.layer_count,
            route,
            &nullifier[..8.min(nullifier.len())]
        ))
    }

    /// Вычислить маршрут через AI Router
    pub async fn compute_route(self: Arc<Self>, destination: &str, priorities: UserPriorities) -> Option<Vec<String>> {
        let ssau_table = self.node.ssau_table.read().await;
        let trust = self.trust.read().await;
        let our_id = self.node.config.node_id.clone();

        let candidates = build_route_candidates(&ssau_table, &our_id, destination, &trust, 5);
        if candidates.is_empty() {
            return None;
        }

        let mut router = self.router.lock().await;
        let decision = router.select_route(destination, candidates, &priorities);

        *self.routes_computed.lock().await += 1;

        decision.chosen_route.map(|r| r.path)
    }

    // -------------------------------------------------------------------------
    // Background loops
    // -------------------------------------------------------------------------

    /// Реальная рассылка SSAU всем peers.
    async fn ssau_broadcast_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(SSAU_BROADCAST_INTERVAL_SECS));
        log::info!("[{}] 📡 SSAU broadcast loop started", self.node.config.node_id);

        let mut sequence: u64 = 1;

        loop {
            ticker.tick().await;

            // соберём тензоры
            let tensors: Vec<SsauTensor> = {
                let table = self.node.ssau_table.read().await;
                table.values().cloned().collect()
            };

            // если нечего слать — пропускаем
            if tensors.is_empty() {
                continue;
            }

            // packet
            let tensor_refs: Vec<&SsauTensor> = tensors.iter().collect();
            let packet = create_ssau_update_packet(&self.node.config.node_id, &tensor_refs, sequence);
            sequence = sequence.wrapping_add(1);

            // peer ids snapshot
            let peer_ids: Vec<String> = {
                let conns = self.node.connections.read().await;
                conns.keys().cloned().collect()
            };

            let mut sent = 0usize;
            for peer_id in peer_ids {
                let conn_arc = {
                    let conns = self.node.connections.read().await;
                    conns.get(&peer_id).cloned()
                };

                if let Some(conn_arc) = conn_arc {
                    let mut conn = conn_arc.lock().await;
                    if conn.send_packet(&packet).await.is_ok() {
                        sent += 1;
                    }
                }
            }

            log::info!(
                "[{}] 📡 SSAU broadcast: tensors={} sent_to_peers={}",
                self.node.config.node_id,
                tensors.len(),
                sent
            );
        }
    }

    /// MVP discovery: раз в интервал отправляем NodeDiscovered всем peers.
    async fn node_discovery_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(PEER_EXCHANGE_INTERVAL_SECS));
        log::info!("[{}] 🧭 Node discovery loop started", self.node.config.node_id);

        loop {
            ticker.tick().await;

            let status = self.node.status().await;

            let info = NodeInfo {
                node_id: status.node_id.clone(),
                address: status.listen_addr.clone(),
                public_key: self.node.config.public_key.clone(),
                trust_weight: 1.0,
                capabilities: NodeCapabilities {
                    is_relay: true,
                    max_bandwidth_mbps: 100,
                    supports_storage: false,
                    supports_consensus: true,
                },
            };

            // локально сохраним
            {
                let mut known = self.known_nodes.write().await;
                known.insert(info.node_id.clone(), info.clone());
            }

            // рассылаем
            let packet = PacketBuilder::new(&self.node.config.node_id)
                .build(FederationMessage::NodeDiscovered(info));

            let peer_ids: Vec<String> = {
                let conns = self.node.connections.read().await;
                conns.keys().cloned().collect()
            };

            for peer_id in peer_ids {
                let conn_arc = {
                    let conns = self.node.connections.read().await;
                    conns.get(&peer_id).cloned()
                };

                if let Some(conn_arc) = conn_arc {
                    let mut conn = conn_arc.lock().await;
                    let _ = conn.send_packet(&packet).await;
                }
            }

            log::info!(
                "[{}] 🧭 Node discovery advertised. known_nodes={}",
                self.node.config.node_id,
                self.known_nodes.read().await.len()
            );
        }
    }

    async fn route_audit_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(ROUTE_AUDIT_INTERVAL_SECS));
        log::info!("[{}] 🔍 Route audit loop started", self.node.config.node_id);

        loop {
            ticker.tick().await;

            let router = self.router.lock().await;
            let unstable = router.audit_active_routes();
            if !unstable.is_empty() {
                log::warn!("⚠️ Unstable routes detected: {:?}", unstable);
            }
        }
    }

    // -------------------------------------------------------------------------
    // Stats
    // -------------------------------------------------------------------------

    pub async fn collect_stats(self: Arc<Self>) -> OverlayStats {
        let node_status = self.node.status().await;

        let dag_stats = {
            let dag = self.dag.lock().await;
            dag.stats()
        };

        let mirage_activations = {
            let mirage = self.mirage.lock().await;
            mirage.detector.mirage_activations
        };

        let nullifiers_seen = {
            let nullifiers = self.nullifiers.lock().await;
            nullifiers.size()
        };

        let known_count = {
            let known = self.known_nodes.read().await;
            known.len()
        };

        // грубая оценка avg_route_latency по SSAU таблице
        let avg_latency = {
            let ssau = self.node.ssau_table.read().await;
            if ssau.is_empty() {
                0.0
            } else {
                ssau.values().map(|t| t.latency.mean).sum::<f64>() / ssau.len() as f64
            }
        };

        OverlayStats {
            node_id: node_status.node_id,
            uptime_secs: node_status.uptime_seconds,
            active_peers: node_status.active_peers,
            known_nodes: known_count,
            ssau_tensors: node_status.ssau_entries,
            dag_nodes: dag_stats.total_nodes,
            dag_total_rewards: dag_stats.total_rewards_issued,
            routes_computed: *self.routes_computed.lock().await,
            packets_onion_wrapped: *self.packets_onion_wrapped.lock().await,
            mirage_activations,
            nullifiers_seen,
            avg_route_latency_ms: avg_latency,
            network_health: dag_stats.avg_honesty_score,
        }
    }
}
