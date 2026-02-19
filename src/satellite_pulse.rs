// =============================================================================
// FEDERATION CORE — satellite_pulse.rs
// PHASE 6 / STEP 8 — «Cosmic Pacemaker — Satellite Pulse Protocol»
// =============================================================================
//
// Когда наземный интернет мёртв — Федерация дышит через спутники.
//
// Pulse = сверхсжатый снимок состояния сети:
//   ModelWeights (нейросеть) + ReputationDigest + MintBlock + DAGHead
//   → сжимаем до <256 байт → кодируем в RadioFrame → Starlink/Iridium
//
// Протокол:
//   1. PulseEncoder   — сжать состояние до минимума
//   2. RadioFrame     — обернуть для спутникового канала
//   3. SatelliteLink  — симуляция канала (задержка, потери, пропускная способность)
//   4. PulseDecoder   — восстановить состояние из Pulse
//   5. BlackoutMode   — режим выживания при полном блэкауте
// =============================================================================

use serde::{Deserialize, Serialize};

pub const PULSE_MAX_BYTES: usize      = 256;   // максимум байт на Pulse
pub const RADIO_FRAME_OVERHEAD: usize = 32;    // заголовок RadioFrame
pub const SAT_LATENCY_MS: u64         = 600;   // задержка Starlink ~600мс
pub const SAT_BANDWIDTH_BPS: u64      = 9_600; // 9.6 kbps — Iridium минимум
pub const PULSE_INTERVAL_SECS: u64    = 300;   // пульс каждые 5 минут
pub const BLACKOUT_THRESHOLD: f64     = 0.95;  // >95% узлов недоступны

// -----------------------------------------------------------------------------
// SatelliteProvider — тип спутниковой связи
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SatelliteProvider {
    Starlink,   // 600мс, 50Mbps, глобальный
    Iridium,    // 1500мс, 9.6kbps, полюса
    Viasat,     // 800мс, 25Mbps, геостационар
    Amateur,    // 2000мс, 1.2kbps, ham radio
    Proprietary(String),
}

impl SatelliteProvider {
    pub fn latency_ms(&self) -> u64 {
        match self {
            SatelliteProvider::Starlink     => 600,
            SatelliteProvider::Iridium      => 1500,
            SatelliteProvider::Viasat       => 800,
            SatelliteProvider::Amateur      => 2000,
            SatelliteProvider::Proprietary(_) => 1000,
        }
    }
    pub fn bandwidth_bps(&self) -> u64 {
        match self {
            SatelliteProvider::Starlink     => 50_000_000,
            SatelliteProvider::Iridium      =>      9_600,
            SatelliteProvider::Viasat       => 25_000_000,
            SatelliteProvider::Amateur      =>      1_200,
            SatelliteProvider::Proprietary(_) =>  100_000,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            SatelliteProvider::Starlink     => "🛰️  Starlink",
            SatelliteProvider::Iridium      => "📡 Iridium",
            SatelliteProvider::Viasat       => "🌐 Viasat",
            SatelliteProvider::Amateur      => "📻 Amateur",
            SatelliteProvider::Proprietary(n) => n.as_str(),
        }
    }
    pub fn max_pulse_bytes(&self) -> usize {
        // Iridium и Amateur — только минимальный Pulse
        match self {
            SatelliteProvider::Starlink   => 65536,
            SatelliteProvider::Iridium    => 256,
            SatelliteProvider::Viasat     => 32768,
            SatelliteProvider::Amateur    => 64,
            SatelliteProvider::Proprietary(_) => 4096,
        }
    }
}

// -----------------------------------------------------------------------------
// FederationPulse — сверхсжатый снимок состояния
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPulse {
    pub pulse_id: u64,
    pub timestamp: i64,
    pub sender_node: String,

    // Дайджест нейросети (32 байта — hash весов)
    pub model_digest: [u8; 8],

    // Дайджест репутации (топ-5 узлов, 4 байта каждый)
    pub rep_digest: Vec<(u32, u16)>,  // (node_hash, score_u16)

    // Mint состояние (12 байт)
    pub mint_block: u64,    // номер блока
    pub total_supply: u32,  // supply / 1000 (в тысячах)

    // DAG голова (8 байт)
    pub dag_head: u64,

    // Тактическая сводка (4 байта)
    pub active_tactic: u8,    // 0=Passive 1=Decoy 2=Strike 3=Aiki
    pub threat_level: u8,     // 0-255
    pub connected_nodes: u16, // кол-во живых узлов

    // Подпись (8 байт)
    pub signature: u64,
}

impl FederationPulse {
    pub fn encode(&self) -> Vec<u8> {
        // Упакованная бинарная сериализация — минимум байт
        let mut buf = Vec::with_capacity(PULSE_MAX_BYTES);

        // Header (16 байт)
        buf.extend_from_slice(&self.pulse_id.to_le_bytes());
        buf.extend_from_slice(&(self.timestamp as u64).to_le_bytes());

        // Model digest (8 байт)
        buf.extend_from_slice(&self.model_digest);

        // Rep digest (5 * 6 = 30 байт)
        for (hash, score) in self.rep_digest.iter().take(5) {
            buf.extend_from_slice(&hash.to_le_bytes());
            buf.extend_from_slice(&score.to_le_bytes());
        }

        // Mint (12 байт)
        buf.extend_from_slice(&self.mint_block.to_le_bytes());
        buf.extend_from_slice(&self.total_supply.to_le_bytes());

        // DAG (8 байт)
        buf.extend_from_slice(&self.dag_head.to_le_bytes());

        // Tactic (4 байта)
        buf.push(self.active_tactic);
        buf.push(self.threat_level);
        buf.extend_from_slice(&self.connected_nodes.to_le_bytes());

        // Signature (8 байт)
        buf.extend_from_slice(&self.signature.to_le_bytes());

        // Node ID — сжато до 16 байт
        let node_bytes: Vec<u8> = self.sender_node.bytes().take(16).collect();
        buf.extend_from_slice(&node_bytes);
        while buf.len() % 8 != 0 { buf.push(0); }

        buf
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 86 { return None; }
        let mut pos = 0;
        let pulse_id = u64::from_le_bytes(bytes[pos..pos+8].try_into().ok()?); pos+=8;
        let timestamp = u64::from_le_bytes(bytes[pos..pos+8].try_into().ok()?) as i64; pos+=8;
        let model_digest: [u8; 8] = bytes[pos..pos+8].try_into().ok()?; pos+=8;
        let mut rep_digest = vec![];
        for _ in 0..5 {
            let h = u32::from_le_bytes(bytes[pos..pos+4].try_into().ok()?); pos+=4;
            let s = u16::from_le_bytes(bytes[pos..pos+2].try_into().ok()?); pos+=2;
            rep_digest.push((h, s));
        }
        let mint_block = u64::from_le_bytes(bytes[pos..pos+8].try_into().ok()?); pos+=8;
        let total_supply = u32::from_le_bytes(bytes[pos..pos+4].try_into().ok()?); pos+=4;
        let dag_head = u64::from_le_bytes(bytes[pos..pos+8].try_into().ok()?); pos+=8;
        let active_tactic = bytes[pos]; pos+=1;
        let threat_level = bytes[pos]; pos+=1;
        let connected_nodes = u16::from_le_bytes(bytes[pos..pos+2].try_into().ok()?); pos+=2;
        let signature = u64::from_le_bytes(bytes[pos..pos+8].try_into().ok()?); pos+=8;
        let sender_node = String::from_utf8_lossy(
            &bytes[pos..bytes.len().min(pos+16)]).trim_end_matches('\0').to_string();

        Some(FederationPulse {
            pulse_id, timestamp, sender_node, model_digest,
            rep_digest, mint_block, total_supply, dag_head,
            active_tactic, threat_level, connected_nodes, signature,
        })
    }

    pub fn size_bytes(&self) -> usize { self.encode().len() }

    pub fn verify_signature(&self) -> bool {
        // Упрощённая проверка — в prod заменить на Ed25519
        let checksum: u64 = self.model_digest.iter()
            .fold(self.pulse_id, |a, &b| a.wrapping_add(b as u64));
        self.signature == checksum ^ 0xFEDE_0001_0000_C0DE
    }

    pub fn tactic_name(&self) -> &str {
        match self.active_tactic {
            0 => "Passive", 1 => "StandoffDecoy",
            2 => "CumulativeStrike", 3 => "AikiReflection",
            _ => "Unknown",
        }
    }
}

pub const FEDERATION_KEY: u64 = 0xFEDE_0001_0000_C0DE;

// -----------------------------------------------------------------------------
// RadioFrame — обёртка для спутникового канала
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioFrame {
    pub frame_id: u64,
    pub provider: SatelliteProvider,
    pub payload: Vec<u8>,          // сжатый Pulse
    pub checksum: u32,
    pub hop_count: u8,
    pub priority: u8,              // 0=low 255=emergency
    pub compression_ratio: f64,
    pub original_size: usize,
}

impl RadioFrame {
    pub fn wrap(pulse: &FederationPulse, provider: SatelliteProvider,
                rng: &mut u64) -> Self {
        *rng ^= *rng << 13; *rng ^= *rng >> 7; *rng ^= *rng << 17;
        let encoded = pulse.encode();
        let original_size = encoded.len();

        // Применяем лёгкое RLE сжатие для нулей
        let compressed = Self::rle_compress(&encoded);
        let ratio = original_size as f64 / compressed.len() as f64;
        let checksum = compressed.iter().fold(0u32,
            |a, &b| a.wrapping_add(b as u32));

        RadioFrame {
            frame_id: *rng,
            provider, payload: compressed,
            checksum, hop_count: 0,
            priority: if pulse.threat_level > 200 { 255 } else { 128 },
            compression_ratio: ratio,
            original_size,
        }
    }

    fn rle_compress(data: &[u8]) -> Vec<u8> {
        // Pulse уже компактен (104б < 256б лимита) — передаём as-is
        data.to_vec()
    }

    fn rle_decompress(data: &[u8]) -> Vec<u8> {
        data.to_vec()
    }

    pub fn unwrap(&self) -> Option<FederationPulse> {
        // Проверка checksum
        let actual = self.payload.iter().fold(0u32,
            |a, &b| a.wrapping_add(b as u32));
        if actual != self.checksum { return None; }
        let decompressed = Self::rle_decompress(&self.payload);
        FederationPulse::decode(&decompressed)
    }

    pub fn fits_channel(&self, provider: &SatelliteProvider) -> bool {
        self.payload.len() + RADIO_FRAME_OVERHEAD <= provider.max_pulse_bytes()
    }

    pub fn transmission_time_ms(&self, provider: &SatelliteProvider) -> u64 {
        let bits = (self.payload.len() + RADIO_FRAME_OVERHEAD) as u64 * 8;
        let transfer_ms = bits * 1000 / provider.bandwidth_bps().max(1);
        provider.latency_ms() + transfer_ms
    }
}

// -----------------------------------------------------------------------------
// SatelliteLink — симуляция спутникового канала
// -----------------------------------------------------------------------------

pub struct SatelliteLink {
    pub provider: SatelliteProvider,
    pub ground_station: String,
    pub frames_sent: u64,
    pub frames_lost: u64,
    pub bytes_transmitted: u64,
    pub is_blackout: bool,
    rng: u64,
}

impl SatelliteLink {
    pub fn new(provider: SatelliteProvider, station: &str) -> Self {
        SatelliteLink {
            provider, ground_station: station.to_string(),
            frames_sent: 0, frames_lost: 0,
            bytes_transmitted: 0, is_blackout: false,
            rng: 0x5A71_1337_FEED_0000,
        }
    }

    fn next_rng(&mut self) -> f64 {
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng & 0xffff) as f64 / 65535.0
    }

    pub fn transmit(&mut self, frame: &RadioFrame) -> TransmitResult {
        if self.is_blackout {
            return TransmitResult::blackout();
        }
        if !frame.fits_channel(&self.provider) {
            return TransmitResult::too_large(
                frame.payload.len(), self.provider.max_pulse_bytes());
        }

        // Симуляция потерь пакетов (5% для Starlink, 15% для Iridium)
        let loss_rate = match self.provider {
            SatelliteProvider::Starlink => 0.05,
            SatelliteProvider::Iridium  => 0.15,
            SatelliteProvider::Amateur  => 0.30,
            _                           => 0.08,
        };

        self.frames_sent += 1;
        if self.next_rng() < loss_rate {
            self.frames_lost += 1;
            return TransmitResult::lost(frame.frame_id);
        }

        self.bytes_transmitted += frame.payload.len() as u64;
        let tx_time = frame.transmission_time_ms(&self.provider);

        TransmitResult {
            success: true, frame_id: frame.frame_id,
            latency_ms: tx_time, bytes: frame.payload.len(),
            provider: self.provider.name().to_string(),
            reason: "OK".into(),
        }
    }

    pub fn link_stats(&self) -> LinkStats {
        let reliability = if self.frames_sent > 0 {
            1.0 - self.frames_lost as f64 / self.frames_sent as f64
        } else { 1.0 };
        LinkStats {
            provider: self.provider.name().to_string(),
            station: self.ground_station.clone(),
            frames_sent: self.frames_sent,
            frames_lost: self.frames_lost,
            reliability,
            bytes_transmitted: self.bytes_transmitted,
            latency_ms: self.provider.latency_ms(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransmitResult {
    pub success: bool, pub frame_id: u64,
    pub latency_ms: u64, pub bytes: usize,
    pub provider: String, pub reason: String,
}

impl TransmitResult {
    pub fn blackout() -> Self {
        TransmitResult { success:false, frame_id:0, latency_ms:0,
            bytes:0, provider:"NONE".into(), reason:"BLACKOUT".into() }
    }
    pub fn lost(id: u64) -> Self {
        TransmitResult { success:false, frame_id:id, latency_ms:0,
            bytes:0, provider:"LOST".into(), reason:"packet_loss".into() }
    }
    pub fn too_large(size: usize, max: usize) -> Self {
        TransmitResult { success:false, frame_id:0, latency_ms:0,
            bytes:0, provider:"ERR".into(),
            reason: format!("too_large: {}>{}", size, max) }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkStats {
    pub provider: String, pub station: String,
    pub frames_sent: u64, pub frames_lost: u64,
    pub reliability: f64, pub bytes_transmitted: u64,
    pub latency_ms: u64,
}

// -----------------------------------------------------------------------------
// BlackoutMode — режим выживания
// -----------------------------------------------------------------------------

pub struct BlackoutMode {
    pub is_active: bool,
    pub online_nodes: u32,
    pub total_nodes: u32,
    pub last_pulse: Option<FederationPulse>,
    pub pulses_missed: u32,
    pub survival_strategy: SurvivalStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurvivalStrategy {
    Normal,
    ReducedPulse,   // пульс реже, только критичные данные
    SatelliteOnly,  // только спутник
    GhostMesh,      // только Ghost-узлы + Droid реле
    LastResort,     // ham radio + физические носители
}

impl BlackoutMode {
    pub fn new(total_nodes: u32) -> Self {
        BlackoutMode {
            is_active: false, online_nodes: total_nodes,
            total_nodes, last_pulse: None, pulses_missed: 0,
            survival_strategy: SurvivalStrategy::Normal,
        }
    }

    pub fn update_connectivity(&mut self, online: u32) {
        self.online_nodes = online;
        let ratio = 1.0 - online as f64 / self.total_nodes as f64;
        self.is_active = ratio >= BLACKOUT_THRESHOLD;
        self.survival_strategy = if ratio >= 0.99 {
            SurvivalStrategy::LastResort
        } else if ratio >= 0.95 {
            SurvivalStrategy::GhostMesh
        } else if ratio >= 0.80 {
            SurvivalStrategy::SatelliteOnly
        } else if ratio >= 0.50 {
            SurvivalStrategy::ReducedPulse
        } else {
            SurvivalStrategy::Normal
        };
    }

    pub fn connectivity_pct(&self) -> f64 {
        self.online_nodes as f64 / self.total_nodes as f64 * 100.0
    }

    pub fn strategy_name(&self) -> &str {
        match self.survival_strategy {
            SurvivalStrategy::Normal       => "🟢 Normal",
            SurvivalStrategy::ReducedPulse => "🟡 ReducedPulse",
            SurvivalStrategy::SatelliteOnly=> "🟠 SatelliteOnly",
            SurvivalStrategy::GhostMesh    => "🔴 GhostMesh",
            SurvivalStrategy::LastResort   => "💀 LastResort",
        }
    }
}
