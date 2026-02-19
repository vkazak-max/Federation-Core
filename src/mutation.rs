// =============================================================================
// FEDERATION CORE — mutation.rs
// PHASE 4 / STEP 3 — «Mutation Engine»
// =============================================================================
//
// Три слоя воздействия:
//   1. Decoy Shells      — «пустые коробочки» (Standoff Padding + Ghost Jitter)
//   2. Cumulative Jet    — «кумулятивный прорыв» (Packet Shaping + Focus Timing)
//   3. Aiki Reflection   — «айкидо» (Signature Mirroring + Resource Exhaustion)
//
// Философия: не ломать стену — стать водой.
// =============================================================================

use serde::{Deserialize, Serialize};

pub const MAX_DECOY_SHELLS: usize = 16;
pub const JITTER_WINDOW_MS: u64 = 50;
pub const MIMICRY_OVERHEAD: f64 = 0.15;
pub const AIKI_REFLECTION_TTL: u8 = 3;
pub const FOCUS_SYNC_WINDOW_MS: u64 = 100;

// -----------------------------------------------------------------------------
// MutationStrategy — три стратегии мутации
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MutationStrategy {
    CumulativeStrike {
        window_ms: u64,
        payload_density: f64,
    },
    StandoffDecoy {
        shell_count: usize,
        jitter_amplitude: f64,
    },
    AikiReflection {
        mirror_depth: u8,
        exhaust_factor: f64,
    },
    Hybrid {
        primary: Box<MutationStrategy>,
        secondary: Box<MutationStrategy>,
    },
}

impl MutationStrategy {
    pub fn name(&self) -> &str {
        match self {
            MutationStrategy::CumulativeStrike { .. } => "CumulativeStrike",
            MutationStrategy::StandoffDecoy { .. }   => "StandoffDecoy",
            MutationStrategy::AikiReflection { .. }  => "AikiReflection",
            MutationStrategy::Hybrid { .. }          => "Hybrid",
        }
    }

    pub fn default_strike() -> Self {
        MutationStrategy::CumulativeStrike {
            window_ms: FOCUS_SYNC_WINDOW_MS,
            payload_density: 0.85,
        }
    }

    pub fn default_decoy() -> Self {
        MutationStrategy::StandoffDecoy {
            shell_count: 8,
            jitter_amplitude: 0.4,
        }
    }

    pub fn default_aiki() -> Self {
        MutationStrategy::AikiReflection {
            mirror_depth: AIKI_REFLECTION_TTL,
            exhaust_factor: 0.7,
        }
    }
}

// -----------------------------------------------------------------------------
// TrafficMimicry — маски для пакетов
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrafficMask {
    VideoStream { codec: String, bitrate_kbps: u32 },
    HttpsRequest { host: String, path: String },
    DnsQuery { domain: String },
    TlsHandshake { version: String },
    WhiteNoise,
}

impl TrafficMask {
    pub fn header_bytes(&self) -> Vec<u8> {
        match self {
            TrafficMask::VideoStream { codec, bitrate_kbps } => {
                // RTP-подобный заголовок
                let mut h = vec![0x80, 0x60]; // V=2, PT=96 (dynamic)
                h.extend_from_slice(&(*bitrate_kbps as u16).to_be_bytes());
                h.extend_from_slice(codec.as_bytes());
                h
            }
            TrafficMask::HttpsRequest { host, path } => {
                format!("GET {} HTTP/1.1\r\nHost: {}\r\n", path, host)
                    .into_bytes()
            }
            TrafficMask::DnsQuery { domain } => {
                let mut h = vec![0x00, 0x01, 0x01, 0x00]; // DNS header
                h.extend_from_slice(domain.as_bytes());
                h
            }
            TrafficMask::TlsHandshake { version } => {
                vec![0x16, 0x03, if version == "1.3" { 0x04 } else { 0x03 },
                     0x00, 0x00]
            }
            TrafficMask::WhiteNoise => {
                (0..16u8).map(|i| i.wrapping_mul(37)).collect()
            }
        }
    }

    pub fn overhead_bytes(&self) -> usize {
        self.header_bytes().len()
    }
}

// -----------------------------------------------------------------------------
// DecoyShell — одна «пустая коробочка»
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoyShell {
    pub id: String,
    pub payload: Vec<u8>,
    pub send_at_offset_ms: u64,
    pub mask: TrafficMask,
    pub is_decoy: bool,
    pub size_bytes: usize,
}

impl DecoyShell {
    pub fn generate(index: usize, rng: &mut u64, mask: TrafficMask) -> Self {
        // xorshift64
        *rng ^= *rng << 13; *rng ^= *rng >> 7; *rng ^= *rng << 17;
        let size = 64 + (*rng % 512) as usize;
        let jitter = *rng % JITTER_WINDOW_MS;
        let payload: Vec<u8> = (0..size).map(|i| {
            *rng ^= *rng << 13; *rng ^= *rng >> 7; *rng ^= *rng << 17;
            (*rng as u8).wrapping_add(i as u8)
        }).collect();
        DecoyShell {
            id: format!("decoy_{:03}", index),
            size_bytes: payload.len(),
            payload,
            send_at_offset_ms: jitter,
            mask,
            is_decoy: true,
        }
    }
}

// -----------------------------------------------------------------------------
// StandoffLayer — слой «разнесённой брони»
// -----------------------------------------------------------------------------

pub struct StandoffLayer {
    pub rng: u64,
    pub shells_generated: u64,
    pub jitter_history: Vec<u64>,
}

impl StandoffLayer {
    pub fn new(seed: u64) -> Self {
        StandoffLayer { rng: seed, shells_generated: 0, jitter_history: vec![] }
    }

    /// Standoff Padding: обернуть реальный пакет в слой ложных пакетов
    pub fn wrap_with_decoys(&mut self, real_payload: &[u8],
        count: usize, mask: TrafficMask) -> DecoyBundle {

        let mut shells = vec![];
        let n = count.min(MAX_DECOY_SHELLS);

        // Ложные пакеты ПЕРЕД реальным
        for i in 0..n/2 {
            shells.push(DecoyShell::generate(i, &mut self.rng, mask.clone()));
        }

        // Реальный пакет с маской
        let header = mask.header_bytes();
        let mut masked = header.clone();
        masked.extend_from_slice(real_payload);

        // Ложные пакеты ПОСЛЕ реального
        for i in n/2..n {
            shells.push(DecoyShell::generate(i, &mut self.rng, mask.clone()));
        }

        self.shells_generated += shells.len() as u64;

        DecoyBundle {
            decoys: shells,
            real_payload: masked,
            real_index: n / 2,
            total_decoy_bytes: 0,
            mask,
        }
    }

    /// Ghost Jitter: смещаем интервалы отправки
    pub fn apply_jitter(&mut self, base_interval_ms: u64) -> Vec<u64> {
        let mut intervals = vec![];
        for _ in 0..8 {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            self.rng ^= self.rng << 17;
            let noise = (self.rng % (JITTER_WINDOW_MS * 2)) as i64
                - JITTER_WINDOW_MS as i64;
            let interval = (base_interval_ms as i64 + noise).max(1) as u64;
            intervals.push(interval);
            self.jitter_history.push(interval);
        }
        intervals
    }

    pub fn jitter_entropy(&self) -> f64 {
        if self.jitter_history.len() < 2 { return 0.0; }
        let mean = self.jitter_history.iter().sum::<u64>() as f64
            / self.jitter_history.len() as f64;
        let var = self.jitter_history.iter()
            .map(|&v| (v as f64 - mean).powi(2)).sum::<f64>()
            / self.jitter_history.len() as f64;
        var.sqrt() / mean
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecoyBundle {
    pub decoys: Vec<DecoyShell>,
    pub real_payload: Vec<u8>,
    pub real_index: usize,
    pub total_decoy_bytes: usize,
    pub mask: TrafficMask,
}

impl DecoyBundle {
    pub fn total_packets(&self) -> usize { self.decoys.len() + 1 }
    pub fn noise_ratio(&self) -> f64 {
        let decoy_bytes: usize = self.decoys.iter().map(|d| d.size_bytes).sum();
        if decoy_bytes + self.real_payload.len() == 0 { return 0.0; }
        decoy_bytes as f64 / (decoy_bytes + self.real_payload.len()) as f64
    }
}

// -----------------------------------------------------------------------------
// CumulativeJet — слой «кумулятивного прорыва»
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusWindow {
    pub open_at_ms: u64,
    pub duration_ms: u64,
    pub target_node: String,
    pub shard_ids: Vec<String>,
    pub payload_density: f64,
    pub synchronized: bool,
}

pub struct CumulativeJet {
    pub windows: Vec<FocusWindow>,
    pub strikes_executed: u64,
    pub total_payload_bytes: u64,
    rng: u64,
}

impl CumulativeJet {
    pub fn new(seed: u64) -> Self {
        CumulativeJet { windows: vec![], strikes_executed: 0,
            total_payload_bytes: 0, rng: seed }
    }

    /// Packet Shaping: упаковать данные максимально плотно под маску
    pub fn shape_packet(&mut self, data: &[u8], mask: &TrafficMask,
        density: f64) -> ShapedPacket {
        let header = mask.header_bytes();
        let max_payload = ((data.len() as f64 * density) as usize).max(1);
        let chunk = &data[..max_payload.min(data.len())];

        // XOR с маской для маскировки паттернов
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let key = (self.rng & 0xff) as u8;
        let masked_data: Vec<u8> = chunk.iter().map(|&b| b ^ key).collect();

        let mut packet = header.clone();
        packet.extend_from_slice(&masked_data);

        self.total_payload_bytes += masked_data.len() as u64;

        ShapedPacket {
            raw: packet,
            mask: mask.clone(),
            density,
            original_size: data.len(),
            shaped_size: masked_data.len(),
            overhead: header.len(),
            xor_key: key,
        }
    }

    /// Focus Timing: синхронизировать удар из нескольких шардов
    pub fn plan_synchronized_strike(&mut self, target: &str,
        shards: Vec<String>, window_ms: u64) -> FocusWindow {
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let open_at = self.rng % 1000;

        let window = FocusWindow {
            open_at_ms: open_at,
            duration_ms: window_ms,
            target_node: target.to_string(),
            shard_ids: shards,
            payload_density: 0.90,
            synchronized: true,
        };
        self.windows.push(window.clone());
        self.strikes_executed += 1;
        window
    }

    pub fn effective_throughput(&self, base_mbps: f64) -> f64 {
        if self.windows.is_empty() { return base_mbps; }
        let avg_density = self.windows.iter()
            .map(|w| w.payload_density).sum::<f64>() / self.windows.len() as f64;
        base_mbps * avg_density * (1.0 - MIMICRY_OVERHEAD)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShapedPacket {
    pub raw: Vec<u8>,
    pub mask: TrafficMask,
    pub density: f64,
    pub original_size: usize,
    pub shaped_size: usize,
    pub overhead: usize,
    pub xor_key: u8,
}

impl ShapedPacket {
    pub fn efficiency(&self) -> f64 {
        if self.original_size == 0 { return 0.0; }
        self.shaped_size as f64 / self.original_size as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CensorProbe {
    pub signature: Vec<u8>,
    pub source_ip: String,
    pub probe_type: ProbeType,
    pub timestamp: i64,
    pub cpu_complexity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProbeType {
    DpiSignatureScan,
    TcpRstInjection,
    DnsPoison,
    BgpHijackProbe,
    SslStripAttempt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AikiResponse {
    pub response_payload: Vec<u8>,
    pub strategy: AikiTactic,
    pub estimated_cpu_cost: f64,
    pub reflection_depth: u8,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AikiTactic {
    SignatureMirror,
    ResourceExhaustion,
    FalsePositiveFlood,
    LoopInduction,
}

pub struct AikiLayer {
    pub reflections_sent: u64,
    pub estimated_cpu_wasted: f64,
    pub probe_history: Vec<CensorProbe>,
    rng: u64,
}

impl AikiLayer {
    pub fn new(seed: u64) -> Self {
        AikiLayer { reflections_sent: 0, estimated_cpu_wasted: 0.0,
            probe_history: vec![], rng: seed }
    }

    pub fn mirror_signature(&mut self, probe: &CensorProbe) -> AikiResponse {
        let mut reflected = probe.signature.clone();
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7; self.rng ^= self.rng << 17;
        let key = (self.rng & 0xff) as u8;
        for b in &mut reflected { *b ^= key; }
        let mut payload = reflected.clone();
        for _ in 0..4 { payload.extend_from_slice(&reflected); }
        let cpu_cost = probe.cpu_complexity * 2.5;
        self.estimated_cpu_wasted += cpu_cost;
        self.reflections_sent += 1;
        AikiResponse {
            response_payload: payload,
            strategy: AikiTactic::SignatureMirror,
            estimated_cpu_cost: cpu_cost,
            reflection_depth: AIKI_REFLECTION_TTL,
            description: format!(
                "Отражаем сигнатуру DPI обратно. Цензор тратит {:.2}x CPU на анализ себя.",
                cpu_cost / probe.cpu_complexity),
        }
    }

    pub fn exhaust_resources(&mut self, probe: &CensorProbe, factor: f64) -> AikiResponse {
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7; self.rng ^= self.rng << 17;
        let payload: Vec<u8> = (0..512).map(|_| {
            self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7; self.rng ^= self.rng << 17;
            (self.rng & 0xff) as u8
        }).collect();
        let cpu_cost = probe.cpu_complexity * factor * 8.0;
        self.estimated_cpu_wasted += cpu_cost;
        self.reflections_sent += 1;
        AikiResponse {
            response_payload: payload,
            strategy: AikiTactic::ResourceExhaustion,
            estimated_cpu_cost: cpu_cost,
            reflection_depth: 1,
            description: format!(
                "Травма: цензор тратит {:.1}x ресурсов на анализ случайных данных.", factor * 8.0),
        }
    }

    pub fn handle_probe(&mut self, probe: CensorProbe) -> AikiResponse {
        let response = match probe.probe_type {
            ProbeType::DpiSignatureScan  => self.mirror_signature(&probe),
            ProbeType::TcpRstInjection   => self.exhaust_resources(&probe, 0.5),
            ProbeType::SslStripAttempt   => self.mirror_signature(&probe),
            ProbeType::DnsPoison         => self.exhaust_resources(&probe, 0.3),
            ProbeType::BgpHijackProbe    => self.exhaust_resources(&probe, 1.0),
        };
        self.probe_history.push(probe);
        response
    }
}

pub struct MutationEngine {
    pub strategy: MutationStrategy,
    pub standoff: StandoffLayer,
    pub jet: CumulativeJet,
    pub aiki: AikiLayer,
    pub mutations_applied: u64,
    pub active_mask: TrafficMask,
}

impl MutationEngine {
    pub fn new(node_id: &str, strategy: MutationStrategy) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in node_id.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        MutationEngine {
            strategy,
            standoff: StandoffLayer::new(h),
            jet: CumulativeJet::new(h ^ 0xc0ffee),
            aiki: AikiLayer::new(h ^ 0xdeadbeef),
            mutations_applied: 0,
            active_mask: TrafficMask::HttpsRequest {
                host: "www.google.com".into(), path: "/generate_204".into(),
            },
        }
    }

    pub fn mutate(&mut self, payload: &[u8], neural_congestion: f64) -> MutationResult {
        self.mutations_applied += 1;
        let effective = if neural_congestion > 0.7 {
            MutationStrategy::CumulativeStrike { window_ms: FOCUS_SYNC_WINDOW_MS, payload_density: 0.90 }
        } else if neural_congestion > 0.4 {
            MutationStrategy::StandoffDecoy { shell_count: 6, jitter_amplitude: 0.5 }
        } else {
            self.strategy.clone()
        };
        match &effective {
            MutationStrategy::CumulativeStrike { window_ms, payload_density } => {
                let shaped = self.jet.shape_packet(payload, &self.active_mask, *payload_density);
                let _w = self.jet.plan_synchronized_strike("target",
                    vec!["shard_0".into(), "shard_1".into()], *window_ms);
                MutationResult {
                    strategy_used: "CumulativeStrike".into(),
                    output_bytes: shaped.raw.len(), efficiency: shaped.efficiency(),
                    decoy_count: 0, noise_ratio: 0.0, aiki_cpu_cost: 0.0,
                    description: format!("Кумулятивный прорыв: плотность={:.0}% эфф={:.1}% окно={}мс",
                        payload_density*100.0, shaped.efficiency()*100.0, window_ms),
                }
            }
            MutationStrategy::StandoffDecoy { shell_count, .. } => {
                let bundle = self.standoff.wrap_with_decoys(payload, *shell_count, self.active_mask.clone());
                let jitter = self.standoff.apply_jitter(20);
                MutationResult {
                    strategy_used: "StandoffDecoy".into(),
                    output_bytes: bundle.real_payload.len(), efficiency: 1.0/(1.0+bundle.noise_ratio()),
                    decoy_count: bundle.decoys.len(), noise_ratio: bundle.noise_ratio(),
                    aiki_cpu_cost: 0.0,
                    description: format!("Разнесённая броня: {} коробочек шум={:.1}% джиттер={}мс",
                        shell_count, bundle.noise_ratio()*100.0,
                        jitter.iter().sum::<u64>()/jitter.len() as u64),
                }
            }
            MutationStrategy::AikiReflection { exhaust_factor: _, .. } => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
                let probe = CensorProbe {
                    signature: payload[..payload.len().min(16)].to_vec(),
                    source_ip: "censor.gov".into(), probe_type: ProbeType::DpiSignatureScan,
                    timestamp: now, cpu_complexity: 1.0,
                };
                let response = self.aiki.handle_probe(probe);
                let cpu = response.estimated_cpu_cost;
                MutationResult {
                    strategy_used: "AikiReflection".into(),
                    output_bytes: response.response_payload.len(), efficiency: 1.0,
                    decoy_count: 0, noise_ratio: 0.0, aiki_cpu_cost: cpu,
                    description: response.description,
                }
            }
            MutationStrategy::Hybrid { .. } => {
                let bundle = self.standoff.wrap_with_decoys(payload, 4, self.active_mask.clone());
                MutationResult {
                    strategy_used: "Hybrid".into(), output_bytes: bundle.real_payload.len(),
                    efficiency: 0.8, decoy_count: bundle.decoys.len(),
                    noise_ratio: bundle.noise_ratio(), aiki_cpu_cost: 0.0,
                    description: "Гибридный режим: коробочки + маска".into(),
                }
            }
        }
    }

    pub fn stats(&self) -> MutationStats {
        MutationStats {
            mutations_applied: self.mutations_applied,
            shells_generated: self.standoff.shells_generated,
            strikes_executed: self.jet.strikes_executed,
            reflections_sent: self.aiki.reflections_sent,
            cpu_wasted_on_censor: self.aiki.estimated_cpu_wasted,
            jitter_entropy: self.standoff.jitter_entropy(),
            total_payload_bytes: self.jet.total_payload_bytes,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MutationResult {
    pub strategy_used: String,
    pub output_bytes: usize,
    pub efficiency: f64,
    pub decoy_count: usize,
    pub noise_ratio: f64,
    pub aiki_cpu_cost: f64,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MutationStats {
    pub mutations_applied: u64,
    pub shells_generated: u64,
    pub strikes_executed: u64,
    pub reflections_sent: u64,
    pub cpu_wasted_on_censor: f64,
    pub jitter_entropy: f64,
    pub total_payload_bytes: u64,
}

impl std::fmt::Display for MutationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  MUTATION ENGINE — STATS                     ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Мутаций:     {:>6}  Коробочек: {:>6}      ║\n\
             ║  Удары:       {:>6}  Отражений: {:>6}      ║\n\
             ║  CPU цензора: {:>8.2}x  Джиттер: {:>7.4}   ║\n\
             ║  Payload:     {:>6} байт                    ║\n\
             ╚══════════════════════════════════════════════╝",
            self.mutations_applied, self.shells_generated,
            self.strikes_executed, self.reflections_sent,
            self.cpu_wasted_on_censor, self.jitter_entropy,
            self.total_payload_bytes,
        )
    }
}
