// =============================================================================
// FEDERATION CORE — transport.rs
// PHASE 4 / STEP 6 — «Transport Layer»
// =============================================================================
//
// Физический слой доставки пакетов под управлением MutationEngine.
// Обеспечивает:
//   1. TransportFrame     — обёртка пакета с метаданными синхронизации
//   2. MicroClock         — микросекундный таймер для FocusTiming
//   3. SyncBarrier        — барьер синхронизации для CumulativeStrike
//   4. TransportChannel   — канал с мутацией и jitter
//   5. TransportScheduler — планировщик синхронных ударов
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const FRAME_VERSION: u8 = 1;
pub const MAX_FRAME_SIZE: usize = 65535;
pub const MIN_JITTER_US: u64 = 100;    // минимальный джиттер 100 мкс
pub const MAX_JITTER_US: u64 = 50_000; // максимальный 50 мс
pub const SYNC_WINDOW_US: u64 = 1_000; // окно синхронизации 1 мс

// -----------------------------------------------------------------------------
// MicroClock — микросекундный таймер
// -----------------------------------------------------------------------------

pub struct MicroClock {
    pub epoch: Instant,
    pub rng: u64,
}

impl MicroClock {
    pub fn new() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        MicroClock {
            epoch: Instant::now(),
            rng: now.as_nanos() as u64 ^ 0xcafe_babe_dead_beef,
        }
    }

    /// Текущее время в микросекундах от запуска
    pub fn now_us(&self) -> u64 {
        self.epoch.elapsed().as_micros() as u64
    }

    /// Текущее время в наносекундах
    pub fn now_ns(&self) -> u128 {
        self.epoch.elapsed().as_nanos()
    }

    /// Случайный джиттер в мкс
    pub fn jitter_us(&mut self, min: u64, max: u64) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        min + (self.rng % (max - min + 1))
    }

    /// Синхронная метка — округление до ближайшего окна
    pub fn sync_mark(&self, window_us: u64) -> u64 {
        let now = self.now_us();
        (now / window_us) * window_us
    }
}

impl Default for MicroClock { fn default() -> Self { Self::new() } }

// -----------------------------------------------------------------------------
// TransportFrame — физический пакет Федерации
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportFrame {
    pub version: u8,
    pub frame_id: u64,
    pub src_node: String,
    pub dst_node: String,
    pub payload: Vec<u8>,
    pub mask_type: String,
    pub created_us: u64,
    pub scheduled_us: u64,        // когда отправить (мкс)
    pub sync_mark: u64,            // метка синхронизации
    pub is_decoy: bool,
    pub strike_group: Option<u64>, // группа синхронного удара
    pub jitter_us: u64,
    pub hop_count: u8,
    pub ttl: u8,
    pub checksum: u32,
}

impl TransportFrame {
    pub fn new(src: &str, dst: &str, payload: Vec<u8>,
               clock: &MicroClock) -> Self {
        let now = clock.now_us();
        let mut f = TransportFrame {
            version: FRAME_VERSION,
            frame_id: clock.now_ns() as u64 ^ 0xfeed_face,
            src_node: src.to_string(),
            dst_node: dst.to_string(),
            checksum: 0,
            payload: payload.clone(),
            mask_type: "raw".into(),
            created_us: now,
            scheduled_us: now,
            sync_mark: 0,
            is_decoy: false,
            strike_group: None,
            jitter_us: 0,
            hop_count: 0,
            ttl: 16,
        };
        f.checksum = f.compute_checksum();
        f
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut h: u32 = 0x811c9dc5;
        for &b in &self.payload {
            h ^= b as u32;
            h = h.wrapping_mul(0x01000193);
        }
        for b in self.src_node.bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x01000193);
        }
        h
    }

    pub fn verify(&self) -> bool {
        self.compute_checksum() == self.checksum
            && self.payload.len() <= MAX_FRAME_SIZE
            && self.ttl > 0
            && self.version == FRAME_VERSION
    }

    pub fn size_bytes(&self) -> usize {
        self.payload.len() + 64 // overhead заголовка
    }

    pub fn latency_us(&self, clock: &MicroClock) -> u64 {
        clock.now_us().saturating_sub(self.created_us)
    }
}

// -----------------------------------------------------------------------------
// SyncBarrier — барьер синхронизации для CumulativeStrike
// -----------------------------------------------------------------------------

pub struct SyncBarrier {
    pub group_id: u64,
    pub expected: usize,
    pub arrived: Vec<(String, u64)>, // (node_id, arrived_us)
    pub window_us: u64,
    pub created_us: u64,
    clock: MicroClock,
}

#[derive(Debug, Clone)]
pub struct StrikeResult {
    pub group_id: u64,
    pub participants: usize,
    pub sync_deviation_us: u64,   // максимальное отклонение между узлами
    pub total_payload_bytes: usize,
    pub fired_at_us: u64,
    pub is_synchronized: bool,    // все узлы попали в окно?
}

impl SyncBarrier {
    pub fn new(group_id: u64, expected: usize, window_us: u64) -> Self {
        let clock = MicroClock::new();
        let now = clock.now_us();
        SyncBarrier {
            group_id, expected, window_us, arrived: vec![],
            created_us: now, clock,
        }
    }

    pub fn arrive(&mut self, node_id: &str) -> bool {
        let now = self.clock.now_us();
        self.arrived.push((node_id.to_string(), now));
        self.arrived.len() >= self.expected
    }

    pub fn fire(&self, frames: &[TransportFrame]) -> StrikeResult {
        let now = self.clock.now_us();
        let times: Vec<u64> = self.arrived.iter().map(|(_, t)| *t).collect();
        let min_t = times.iter().cloned().min().unwrap_or(now);
        let max_t = times.iter().cloned().max().unwrap_or(now);
        let deviation = max_t - min_t;
        let total_bytes: usize = frames.iter().map(|f| f.payload.len()).sum();

        StrikeResult {
            group_id: self.group_id,
            participants: self.arrived.len(),
            sync_deviation_us: deviation,
            total_payload_bytes: total_bytes,
            fired_at_us: now,
            is_synchronized: deviation <= self.window_us,
        }
    }
}

// -----------------------------------------------------------------------------
// TransportChannel — канал с мутацией и jitter
// -----------------------------------------------------------------------------

pub struct TransportChannel {
    pub channel_id: String,
    pub src: String,
    pub dst: String,
    pub clock: MicroClock,
    pub frames_sent: u64,
    pub bytes_sent: u64,
    pub decoys_sent: u64,
    pub avg_latency_us: f64,
    pub queue: Vec<TransportFrame>,
    pub jitter_history: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    pub frame_id: u64,
    pub scheduled_us: u64,
    pub jitter_applied_us: u64,
    pub is_decoy: bool,
    pub mask_type: String,
    pub channel_id: String,
}

impl TransportChannel {
    pub fn new(src: &str, dst: &str) -> Self {
        TransportChannel {
            channel_id: format!("{}->{}", src, dst),
            src: src.to_string(),
            dst: dst.to_string(),
            clock: MicroClock::new(),
            frames_sent: 0,
            bytes_sent: 0,
            decoys_sent: 0,
            avg_latency_us: 0.0,
            queue: vec![],
            jitter_history: vec![],
        }
    }

    /// Применить мутацию и поставить в очередь
    pub fn enqueue(&mut self, payload: &[u8], mask_type: &str,
                   is_decoy: bool, strike_group: Option<u64>) -> SendResult {
        let jitter = self.clock.jitter_us(MIN_JITTER_US, MAX_JITTER_US);
        let now = self.clock.now_us();
        let sync = self.clock.sync_mark(SYNC_WINDOW_US);

        let mut frame = TransportFrame::new(&self.src, &self.dst,
            payload.to_vec(), &self.clock);
        frame.mask_type = mask_type.to_string();
        frame.is_decoy = is_decoy;
        frame.strike_group = strike_group;
        frame.jitter_us = jitter;
        frame.scheduled_us = now + jitter;
        frame.sync_mark = sync;

        let result = SendResult {
            frame_id: frame.frame_id,
            scheduled_us: frame.scheduled_us,
            jitter_applied_us: jitter,
            is_decoy,
            mask_type: mask_type.to_string(),
            channel_id: self.channel_id.clone(),
        };

        self.queue.push(frame);
        self.jitter_history.push(jitter);
        if self.jitter_history.len() > 100 {
            self.jitter_history.remove(0);
        }
        result
    }

    /// Применить StandoffDecoy — обернуть реальный пакет в ложные
    pub fn send_with_decoys(&mut self, payload: &[u8], mask_type: &str,
                             decoy_count: usize) -> Vec<SendResult> {
        let mut results = vec![];
        // Ложные пакеты ДО
        for _ in 0..decoy_count/2 {
            let decoy_payload: Vec<u8> = (0..64).map(|i| {
                self.clock.rng ^= self.clock.rng << 13;
                (self.clock.rng as u8).wrapping_add(i)
            }).collect();
            results.push(self.enqueue(&decoy_payload, mask_type, true, None));
        }
        // Реальный пакет
        results.push(self.enqueue(payload, mask_type, false, None));
        // Ложные пакеты ПОСЛЕ
        for _ in decoy_count/2..decoy_count {
            let decoy_payload: Vec<u8> = (0..64).map(|i| {
                self.clock.rng ^= self.clock.rng << 17;
                (self.clock.rng as u8).wrapping_add(i)
            }).collect();
            results.push(self.enqueue(&decoy_payload, mask_type, true, None));
        }
        results
    }

    /// Применить CumulativeStrike — синхронизированная отправка
    pub fn send_strike(&mut self, payload: &[u8], mask_type: &str,
                        group_id: u64) -> SendResult {
        self.enqueue(payload, mask_type, false, Some(group_id))
    }

    /// Сфлашить очередь (симуляция отправки)
    pub fn flush(&mut self) -> Vec<TransportFrame> {
        let now = self.clock.now_us();
        // Разделяем — готовые и ещё не время
        let (ready, pending): (Vec<_>, Vec<_>) = self.queue.drain(..)
            .partition(|f| f.scheduled_us <= now + 1000);
        self.queue = pending;

        for f in &ready {
            self.frames_sent += 1;
            self.bytes_sent += f.payload.len() as u64;
            if f.is_decoy { self.decoys_sent += 1; }
            let lat = f.latency_us(&self.clock) as f64;
            self.avg_latency_us = self.avg_latency_us * 0.9 + lat * 0.1;
        }
        ready
    }

    pub fn jitter_entropy(&self) -> f64 {
        if self.jitter_history.len() < 2 { return 0.0; }
        let mean = self.jitter_history.iter().sum::<u64>() as f64
            / self.jitter_history.len() as f64;
        let var = self.jitter_history.iter()
            .map(|&v| (v as f64 - mean).powi(2)).sum::<f64>()
            / self.jitter_history.len() as f64;
        var.sqrt() / mean.max(1.0)
    }

    pub fn stats(&self) -> ChannelStats {
        ChannelStats {
            channel_id: self.channel_id.clone(),
            frames_sent: self.frames_sent,
            bytes_sent: self.bytes_sent,
            decoys_sent: self.decoys_sent,
            queue_depth: self.queue.len(),
            avg_latency_us: self.avg_latency_us,
            jitter_entropy: self.jitter_entropy(),
            decoy_ratio: if self.frames_sent > 0 {
                self.decoys_sent as f64 / self.frames_sent as f64
            } else { 0.0 },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub channel_id: String,
    pub frames_sent: u64,
    pub bytes_sent: u64,
    pub decoys_sent: u64,
    pub queue_depth: usize,
    pub avg_latency_us: f64,
    pub jitter_entropy: f64,
    pub decoy_ratio: f64,
}

impl std::fmt::Display for ChannelStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "Channel [{}]\n  \
             Фреймов: {:>6}  Байт: {:>8}  Коробочек: {:>5}\n  \
             Очередь: {:>3}  Задержка: {:>8.1}мкс  \
             Джиттер: {:>6.4}  Decoy%: {:.1}%",
            self.channel_id,
            self.frames_sent, self.bytes_sent, self.decoys_sent,
            self.queue_depth, self.avg_latency_us,
            self.jitter_entropy, self.decoy_ratio * 100.0,
        )
    }
}

// -----------------------------------------------------------------------------
// TransportScheduler — планировщик синхронных ударов
// -----------------------------------------------------------------------------

pub struct TransportScheduler {
    pub channels: HashMap<String, TransportChannel>,
    pub barriers: HashMap<u64, SyncBarrier>,
    pub clock: MicroClock,
    pub strike_results: Vec<StrikeResult>,
    rng: u64,
}

impl TransportScheduler {
    pub fn new() -> Self {
        let clock = MicroClock::new();
        let rng = clock.now_ns() as u64 ^ 0x1337_c0de;
        TransportScheduler {
            channels: HashMap::new(),
            barriers: HashMap::new(),
            clock, strike_results: vec![], rng,
        }
    }

    pub fn add_channel(&mut self, src: &str, dst: &str) {
        let ch = TransportChannel::new(src, dst);
        self.channels.insert(ch.channel_id.clone(), ch);
    }

    /// Запланировать CumulativeStrike из нескольких каналов
    pub fn plan_strike(&mut self, group_id: u64, channels: &[&str],
                        payload: &[u8], window_us: u64) -> Vec<SendResult> {
        let barrier = SyncBarrier::new(group_id, channels.len(), window_us);
        self.barriers.insert(group_id, barrier);

        let mut results = vec![];
        for &ch_id in channels {
            if let Some(ch) = self.channels.get_mut(ch_id) {
                results.push(ch.send_strike(payload, "VideoStream", group_id));
            }
        }
        results
    }

    /// Выполнить барьер — собрать прибывших и выстрелить
    pub fn execute_barrier(&mut self, group_id: u64) -> Option<StrikeResult> {
        let channel_ids: Vec<String> = self.channels.keys().cloned().collect();
        if let Some(barrier) = self.barriers.get_mut(&group_id) {
            for ch_id in &channel_ids {
                barrier.arrive(ch_id);
            }
            // Собираем фреймы этой группы
            let frames: Vec<TransportFrame> = self.channels.values()
                .flat_map(|ch| ch.queue.iter()
                    .filter(|f| f.strike_group == Some(group_id))
                    .cloned())
                .collect();
            let result = barrier.fire(&frames);
            self.strike_results.push(result.clone());
            self.barriers.remove(&group_id);
            Some(result)
        } else { None }
    }

    /// Флашим все каналы
    pub fn flush_all(&mut self) -> FlushResult {
        let mut total_frames = 0;
        let mut total_bytes = 0;
        let mut total_decoys = 0;
        for ch in self.channels.values_mut() {
            let flushed = ch.flush();
            total_frames += flushed.len();
            total_bytes += flushed.iter().map(|f| f.payload.len()).sum::<usize>();
            total_decoys += flushed.iter().filter(|f| f.is_decoy).count();
        }
        FlushResult { total_frames, total_bytes, total_decoys,
            flushed_at_us: self.clock.now_us() }
    }

    pub fn global_stats(&self) -> GlobalTransportStats {
        let total_sent: u64 = self.channels.values().map(|c| c.frames_sent).sum();
        let total_bytes: u64 = self.channels.values().map(|c| c.bytes_sent).sum();
        let total_decoys: u64 = self.channels.values().map(|c| c.decoys_sent).sum();
        let avg_jitter = if !self.channels.is_empty() {
            self.channels.values().map(|c| c.jitter_entropy()).sum::<f64>()
                / self.channels.len() as f64
        } else { 0.0 };

        GlobalTransportStats {
            channels: self.channels.len(),
            total_frames: total_sent,
            total_bytes,
            total_decoys,
            avg_jitter_entropy: avg_jitter,
            strikes_executed: self.strike_results.len(),
            avg_sync_deviation_us: if self.strike_results.is_empty() { 0 } else {
                self.strike_results.iter()
                    .map(|r| r.sync_deviation_us).sum::<u64>()
                    / self.strike_results.len() as u64
            },
        }
    }
}

impl Default for TransportScheduler { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct FlushResult {
    pub total_frames: usize,
    pub total_bytes: usize,
    pub total_decoys: usize,
    pub flushed_at_us: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalTransportStats {
    pub channels: usize,
    pub total_frames: u64,
    pub total_bytes: u64,
    pub total_decoys: u64,
    pub avg_jitter_entropy: f64,
    pub strikes_executed: usize,
    pub avg_sync_deviation_us: u64,
}

impl std::fmt::Display for GlobalTransportStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  TRANSPORT LAYER — GLOBAL STATS              ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Каналов:  {:>4}  Фреймов:   {:>8}         ║\n\
             ║  Байт:     {:>8}  Коробочек: {:>6}         ║\n\
             ║  Ударов:   {:>4}  Откл.синхр: {:>5}мкс     ║\n\
             ║  Джиттер-энтропия: {:>8.4}                  ║\n\
             ╚══════════════════════════════════════════════╝",
            self.channels, self.total_frames,
            self.total_bytes, self.total_decoys,
            self.strikes_executed, self.avg_sync_deviation_us,
            self.avg_jitter_entropy,
        )
    }
}

// =============================================================================
// HIERARCHICAL ROUTING — Phase 8 Patch
// Пакеты выбирают путь по классу железа из inventory.rs
//
//   Sentinel/Citadel → FastLane  (скорость, прямой путь)
//   Workstation      → Standard  (баланс)
//   Ghost/Droid      → NoiseLane (шум, скрытность)
//   Mobile           → LowPower  (минимум трафика)
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum RouteLane {
    FastLane,   // Sentinel/Citadel — минимум хопов, максимум скорость
    Standard,   // Workstation — обычный путь
    NoiseLane,  // Ghost/Droid — через шум, скрытность важнее скорости
    LowPower,   // Mobile — минимум батареи
}

impl RouteLane {
    pub fn from_role(role: &str) -> Self {
        match role {
            "Sentinel" | "Citadel"    => RouteLane::FastLane,
            "Workstation"             => RouteLane::Standard,
            "Ghost" | "Droid"         => RouteLane::NoiseLane,
            "Mobile"                  => RouteLane::LowPower,
            _                         => RouteLane::Standard,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            RouteLane::FastLane  => "⚡ FastLane",
            RouteLane::Standard  => "🔄 Standard",
            RouteLane::NoiseLane => "👻 NoiseLane",
            RouteLane::LowPower  => "🔋 LowPower",
        }
    }
    pub fn max_hops(&self) -> u8 {
        match self {
            RouteLane::FastLane  => 2,
            RouteLane::Standard  => 4,
            RouteLane::NoiseLane => 7,  // больше хопов = больше шума
            RouteLane::LowPower  => 3,
        }
    }
    pub fn decoy_ratio(&self) -> f64 {
        match self {
            RouteLane::FastLane  => 0.0,  // нет приманок — только скорость
            RouteLane::Standard  => 0.3,
            RouteLane::NoiseLane => 3.0,  // 3 приманки на 1 реальный
            RouteLane::LowPower  => 0.1,
        }
    }
    pub fn latency_mult(&self) -> f64 {
        match self {
            RouteLane::FastLane  => 0.5,  // вдвое быстрее
            RouteLane::Standard  => 1.0,
            RouteLane::NoiseLane => 2.5,  // медленнее но скрытнее
            RouteLane::LowPower  => 1.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HierarchicalRoute {
    pub src: String,
    pub dst: String,
    pub lane: RouteLane,
    pub hops: Vec<String>,       // промежуточные узлы
    pub decoy_paths: Vec<Vec<String>>, // ложные маршруты
    pub estimated_latency_ms: u32,
    pub stealth_score: f64,      // 0=открыто 1=полностью скрыт
}

impl HierarchicalRoute {
    pub fn build(src: &str, src_role: &str, dst: &str,
                 available_nodes: &[(String, String)], // (node_id, role)
                 base_latency_ms: u32) -> Self {
        let lane = RouteLane::from_role(src_role);
        let max_hops = lane.max_hops() as usize;

        // Выбираем промежуточные узлы по роли
        let preferred_roles: Vec<&str> = match lane {
            RouteLane::FastLane  => vec!["Sentinel", "Citadel"],
            RouteLane::Standard  => vec!["Workstation", "Citadel"],
            RouteLane::NoiseLane => vec!["Ghost", "Droid", "Mobile"],
            RouteLane::LowPower  => vec!["Mobile", "Workstation"],
        };

        let mut hops: Vec<String> = available_nodes.iter()
            .filter(|(id, role)| {
                id != src && id != dst &&
                preferred_roles.contains(&role.as_str())
            })
            .take(max_hops.saturating_sub(1))
            .map(|(id, _)| id.clone())
            .collect();
        hops.push(dst.to_string());

        // Ложные маршруты для NoiseLane
        let decoy_count = (lane.decoy_ratio() as usize).max(0);
        let mut decoy_paths = vec![];
        for i in 0..decoy_count {
            let decoy: Vec<String> = available_nodes.iter()
                .filter(|(id, _)| id != src)
                .skip(i * 2)
                .take(3)
                .map(|(id, _)| id.clone())
                .collect();
            if !decoy.is_empty() { decoy_paths.push(decoy); }
        }

        let latency = (base_latency_ms as f64 * lane.latency_mult()
            * (1.0 + hops.len() as f64 * 0.1)) as u32;
        let stealth = match lane {
            RouteLane::FastLane  => 0.2,
            RouteLane::Standard  => 0.5,
            RouteLane::NoiseLane => 0.9,
            RouteLane::LowPower  => 0.4,
        };

        HierarchicalRoute {
            src: src.to_string(), dst: dst.to_string(),
            lane, hops, decoy_paths,
            estimated_latency_ms: latency, stealth_score: stealth,
        }
    }

    pub fn total_traffic_ratio(&self) -> f64 {
        // Сколько трафика генерируется включая приманки
        1.0 + self.decoy_paths.len() as f64 * self.lane.decoy_ratio()
    }
}

pub struct HierarchicalRouter {
    pub routes: Vec<HierarchicalRoute>,
    pub packets_routed: u64,
    pub total_decoys_sent: u64,
}

impl HierarchicalRouter {
    pub fn new() -> Self {
        HierarchicalRouter { routes: vec![], packets_routed:0, total_decoys_sent:0 }
    }

    pub fn route(&mut self, src: &str, src_role: &str, dst: &str,
                 nodes: &[(String, String)], base_latency: u32) -> &HierarchicalRoute {
        let route = HierarchicalRoute::build(src, src_role, dst, nodes, base_latency);
        self.packets_routed += 1;
        self.total_decoys_sent += route.decoy_paths.len() as u64;
        self.routes.push(route);
        self.routes.last().unwrap()
    }

    pub fn stats(&self) -> String {
        format!("routed={} decoys={} routes={}",
            self.packets_routed, self.total_decoys_sent, self.routes.len())
    }
}

impl Default for HierarchicalRouter { fn default() -> Self { Self::new() } }
