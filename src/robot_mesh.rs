// =============================================================================
// FEDERATION CORE — robot_mesh.rs
// PHASE 6 / STEP 9 — «Droid Symbiosis — Home Bastion Protocol»
// =============================================================================
//
// Домашние роботы — невидимая инфраструктура Федерации.
// Пылесос знает план квартиры. Колонка слышит всё. Холодильник онлайн 24/7.
//
// Протокол:
//   DroidDriver    — абстракция над BT/Zigbee/Z-Wave/Matter
//   HomeBastion    — квартира как узел меш-сети
//   MeshRelay      — передача данных между квартирами
//   StealthCarrier — Pulse спрятан в служебном трафике дроида
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const BT_MAX_PAYLOAD: usize    = 512;   // Bluetooth ATT MTU
pub const ZIGBEE_MAX_PAYLOAD: usize= 84;    // Zigbee frame payload
pub const MESH_HOP_TTL: u8         = 7;     // максимум хопов
pub const STEALTH_INTERVAL_SECS: u64 = 60;  // раз в минуту в служебном трафике
pub const BASTION_SCAN_RADIUS_M: u32 = 30;  // радиус BT сканирования

// -----------------------------------------------------------------------------
// RadioProtocol — беспроводной протокол дроида
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RadioProtocol {
    Bluetooth5,   // 512б, 10м, 2Mbps
    BluetoothLE,  // 244б, 100м, 1Mbps
    Zigbee,       // 84б,  100м, 250kbps
    ZWave,        // 64б,  100м, 100kbps
    Matter,       // 1280б, 100м, WiFi/BT/Thread
    Thread,       // 1280б, 300м, меш
    WiFiDirect,   // 65535б, 200м, 250Mbps
}

impl RadioProtocol {
    pub fn max_payload(&self) -> usize {
        match self {
            RadioProtocol::Bluetooth5  => 512,
            RadioProtocol::BluetoothLE => 244,
            RadioProtocol::Zigbee      => 84,
            RadioProtocol::ZWave       => 64,
            RadioProtocol::Matter      => 1280,
            RadioProtocol::Thread      => 1280,
            RadioProtocol::WiFiDirect  => 65535,
        }
    }
    pub fn range_m(&self) -> u32 {
        match self {
            RadioProtocol::Bluetooth5  => 10,
            RadioProtocol::BluetoothLE => 100,
            RadioProtocol::Zigbee      => 100,
            RadioProtocol::ZWave       => 100,
            RadioProtocol::Matter      => 100,
            RadioProtocol::Thread      => 300,
            RadioProtocol::WiFiDirect  => 200,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            RadioProtocol::Bluetooth5  => "BT5",
            RadioProtocol::BluetoothLE => "BLE",
            RadioProtocol::Zigbee      => "Zigbee",
            RadioProtocol::ZWave       => "Z-Wave",
            RadioProtocol::Matter      => "Matter",
            RadioProtocol::Thread      => "Thread",
            RadioProtocol::WiFiDirect  => "WiFi-D",
        }
    }
}

// -----------------------------------------------------------------------------
// DroidType — тип домашнего устройства
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DroidType {
    Vacuum,        // пылесос — знает карту квартиры, постоянно движется
    Speaker,       // колонка — всегда онлайн, хороший радиус
    Fridge,        // холодильник — 24/7, стабильный
    Thermostat,    // термостат — низкое энергопотребление, Zigbee
    DoorLock,      // замок — критичный узел входа/выхода
    Hub,           // хаб умного дома — агрегатор
    TV,            // телевизор — WiFi Direct, большая полоса
    WashingMachine,// стиралка — периодически онлайн
}

impl DroidType {
    pub fn icon(&self) -> &str {
        match self {
            DroidType::Vacuum         => "🤖",
            DroidType::Speaker        => "🔊",
            DroidType::Fridge         => "🧊",
            DroidType::Thermostat     => "🌡️ ",
            DroidType::DoorLock       => "🔒",
            DroidType::Hub            => "📡",
            DroidType::TV             => "📺",
            DroidType::WashingMachine => "🫧",
        }
    }
    pub fn uptime_pct(&self) -> f64 {
        match self {
            DroidType::Vacuum         => 0.30, // только во время уборки
            DroidType::Speaker        => 0.95,
            DroidType::Fridge         => 1.00,
            DroidType::Thermostat     => 1.00,
            DroidType::DoorLock       => 1.00,
            DroidType::Hub            => 0.99,
            DroidType::TV             => 0.40,
            DroidType::WashingMachine => 0.10,
        }
    }
    pub fn stealth_cover(&self) -> &str {
        match self {
            DroidType::Vacuum    => "маршрутные данные уборки",
            DroidType::Speaker   => "аудио метаданные",
            DroidType::Fridge    => "температурные логи",
            DroidType::Thermostat=> "климатические данные",
            DroidType::DoorLock  => "события доступа",
            DroidType::Hub       => "служебный heartbeat",
            DroidType::TV        => "метаданные контента",
            DroidType::WashingMachine => "цикл программы",
        }
    }
}

// -----------------------------------------------------------------------------
// DroidNode — один дроид как узел меша
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroidNode {
    pub droid_id: String,
    pub droid_type: DroidType,
    pub protocols: Vec<RadioProtocol>,
    pub apartment_id: String,
    pub floor: i32,
    pub position_x: f32,       // позиция в квартире (метры)
    pub position_y: f32,
    pub battery_pct: u8,       // 255 = сеть питания
    pub firmware_patched: bool,// патч установлен
    pub mesh_enabled: bool,
    pub relay_count: u64,      // сколько пакетов пересланы
    pub bytes_relayed: u64,
}

impl DroidNode {
    pub fn best_protocol(&self) -> Option<&RadioProtocol> {
        // Приоритет: WiFiDirect > Thread > Matter > BT5 > BLE > Zigbee > ZWave
        let priority = [
            RadioProtocol::WiFiDirect,
            RadioProtocol::Thread,
            RadioProtocol::Matter,
            RadioProtocol::Bluetooth5,
            RadioProtocol::BluetoothLE,
            RadioProtocol::Zigbee,
            RadioProtocol::ZWave,
        ];
        for p in &priority {
            if self.protocols.contains(p) { return Some(
                self.protocols.iter().find(|x| *x == p).unwrap()); }
        }
        None
    }

    pub fn can_relay(&self, payload_size: usize) -> bool {
        self.mesh_enabled && self.firmware_patched &&
        self.protocols.iter().any(|p| p.max_payload() >= payload_size)
    }

    pub fn signal_strength_to(&self, other: &DroidNode) -> f32 {
        let dx = self.position_x - other.position_x;
        let dy = self.position_y - other.position_y;
        let dist = (dx*dx + dy*dy).sqrt();
        // RSSI упрощённо: -40 dBm на 1м, -6 dBm на удвоение
        let max_range = self.protocols.iter()
            .map(|p| p.range_m() as f32).fold(0.0f32, f32::max);
        if dist > max_range { return -100.0; }
        -40.0 - 20.0 * (dist.max(0.1)).log10()
    }
}

// -----------------------------------------------------------------------------
// StealthPacket — данные Федерации спрятаны в служебном трафике
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthPacket {
    pub packet_id: u64,
    pub cover_type: String,      // маскировка под служебный трафик
    pub cover_data: Vec<u8>,     // легитимные данные (температура, маршрут...)
    pub hidden_payload: Vec<u8>, // данные Федерации
    pub hidden_offset: usize,    // смещение в cover_data
    pub hop_ttl: u8,
    pub next_hop: Option<String>,
    pub checksum: u32,
}

impl StealthPacket {
    pub fn embed(federation_data: &[u8], droid: &DroidNode,
                 rng: &mut u64) -> Option<Self> {
        let proto = droid.best_protocol()?;
        if federation_data.len() + 16 > proto.max_payload() {
            return None; // данные не влезают
        }

        *rng ^= *rng << 13; *rng ^= *rng >> 7; *rng ^= *rng << 17;

        // Генерируем правдоподобные служебные данные
        let cover_size = proto.max_payload().min(128);
        let mut cover_data: Vec<u8> = (0..cover_size)
            .map(|_| { *rng ^= *rng << 17; (*rng & 0xff) as u8 }).collect();

        // Прячем федеральные данные в середину cover_data
        let offset = cover_size / 3;
        let end = (offset + federation_data.len()).min(cover_size);
        cover_data[offset..end].copy_from_slice(
            &federation_data[..end-offset]);

        let checksum = cover_data.iter()
            .fold(0u32, |a, &b| a.wrapping_add(b as u32));

        Some(StealthPacket {
            packet_id: *rng,
            cover_type: droid.droid_type.stealth_cover().to_string(),
            cover_data, hidden_payload: federation_data.to_vec(),
            hidden_offset: offset, hop_ttl: MESH_HOP_TTL,
            next_hop: None, checksum,
        })
    }

    pub fn extract(&self) -> Vec<u8> {
        self.hidden_payload.clone()
    }

    pub fn total_size(&self) -> usize {
        self.cover_data.len() + 32 // overhead
    }
}

// -----------------------------------------------------------------------------
// HomeBastion — квартира как узел меш-сети
// -----------------------------------------------------------------------------

pub struct HomeBastion {
    pub apartment_id: String,
    pub owner_node: String,      // Federation node ID хозяина
    pub floor: i32,
    pub droids: HashMap<String, DroidNode>,
    pub mesh_active: bool,
    pub packets_relayed: u64,
    pub bytes_relayed: u64,
    pub neighbors: Vec<String>,  // соседние квартиры
    rng: u64,
}

impl HomeBastion {
    pub fn new(apt_id: &str, owner: &str, floor: i32) -> Self {
        HomeBastion {
            apartment_id: apt_id.to_string(),
            owner_node: owner.to_string(),
            floor,
            droids: HashMap::new(),
            mesh_active: false,
            packets_relayed: 0,
            bytes_relayed: 0,
            neighbors: vec![],
            rng: 0xBA57_F33D_CAFE_0000,
        }
    }

    pub fn add_droid(&mut self, droid: DroidNode) {
        if droid.mesh_enabled && droid.firmware_patched {
            self.mesh_active = true;
        }
        self.droids.insert(droid.droid_id.clone(), droid);
    }

    pub fn best_relay(&self, payload_size: usize) -> Option<&DroidNode> {
        self.droids.values()
            .filter(|d| d.can_relay(payload_size))
            .max_by(|a, b| {
                let score_a = a.droid_type.uptime_pct()
                    * a.best_protocol().map(|p| p.max_payload() as f64).unwrap_or(0.0);
                let score_b = b.droid_type.uptime_pct()
                    * b.best_protocol().map(|p| p.max_payload() as f64).unwrap_or(0.0);
                score_a.partial_cmp(&score_b).unwrap()
            })
    }

    pub fn relay_packet(&mut self, data: &[u8]) -> RelayResult {
        // Собираем данные без borrow на self
        let relay_info = self.best_relay(data.len()).map(|droid| {
            let proto = droid.best_protocol().unwrap();
            let latency = match proto {
                RadioProtocol::Bluetooth5  => 5,
                RadioProtocol::BluetoothLE => 15,
                RadioProtocol::Zigbee      => 30,
                RadioProtocol::Thread      => 20,
                RadioProtocol::WiFiDirect  => 2,
                _                          => 25,
            };
            (droid.droid_id.clone(), proto.name().to_string(),
             latency, droid.droid_type.stealth_cover().to_string())
        });

        match relay_info {
            None => RelayResult {
                success: false, droid_id: "none".into(),
                protocol: "none".into(), latency_ms: 0,
                stealth_cover: "none".into(),
                reason: "нет подходящего дроида".into(),
            },
            Some((droid_id, protocol, latency_ms, stealth_cover)) => {
                self.packets_relayed += 1;
                self.bytes_relayed += data.len() as u64;
                RelayResult {
                    success: true, droid_id, protocol,
                    latency_ms, stealth_cover, reason: "OK".into(),
                }
            }
        }
    }

    pub fn mesh_coverage(&self) -> f64 {
        // Процент площади квартиры покрытой меш-сетью
        let active = self.droids.values()
            .filter(|d| d.mesh_enabled).count();
        (active as f64 / self.droids.len().max(1) as f64).min(1.0)
    }

    pub fn bastion_stats(&self) -> BastionStats {
        let active_droids = self.droids.values()
            .filter(|d| d.mesh_enabled && d.firmware_patched).count();
        let total_uptime: f64 = self.droids.values()
            .map(|d| d.droid_type.uptime_pct()).sum::<f64>()
            / self.droids.len().max(1) as f64;

        BastionStats {
            apartment_id: self.apartment_id.clone(),
            total_droids: self.droids.len(),
            active_droids,
            mesh_coverage: self.mesh_coverage(),
            avg_uptime: total_uptime,
            packets_relayed: self.packets_relayed,
            bytes_relayed: self.bytes_relayed,
            neighbors: self.neighbors.len(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RelayResult {
    pub success: bool,
    pub droid_id: String,
    pub protocol: String,
    pub latency_ms: u32,
    pub stealth_cover: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BastionStats {
    pub apartment_id: String,
    pub total_droids: usize,
    pub active_droids: usize,
    pub mesh_coverage: f64,
    pub avg_uptime: f64,
    pub packets_relayed: u64,
    pub bytes_relayed: u64,
    pub neighbors: usize,
}

// -----------------------------------------------------------------------------
// CityMesh — городская меш-сеть из бастионов
// -----------------------------------------------------------------------------

pub struct CityMesh {
    pub city: String,
    pub bastions: HashMap<String, HomeBastion>,
    pub total_relayed: u64,
    pub active_routes: Vec<(String, String, Vec<String>)>, // from→to via droids
}

impl CityMesh {
    pub fn new(city: &str) -> Self {
        CityMesh { city: city.to_string(),
            bastions: HashMap::new(),
            total_relayed: 0, active_routes: vec![] }
    }

    pub fn add_bastion(&mut self, bastion: HomeBastion) {
        self.bastions.insert(bastion.apartment_id.clone(), bastion);
    }

    pub fn connect_neighbors(&mut self, apt_a: &str, apt_b: &str) {
        if let Some(a) = self.bastions.get_mut(apt_a) {
            a.neighbors.push(apt_b.to_string());
        }
        if let Some(b) = self.bastions.get_mut(apt_b) {
            b.neighbors.push(apt_a.to_string());
        }
    }

    pub fn route_through_mesh(&mut self, from: &str, to: &str,
                               _data: &[u8]) -> MeshRouteResult {
        // BFS по бастионам
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((from.to_string(), vec![from.to_string()]));
        visited.insert(from.to_string());

        while let Some((current, path)) = queue.pop_front() {
            if current == to {
                self.total_relayed += 1;
                return MeshRouteResult {
                    success: true, hops: path.len() as u8 - 1,
                    path: path.clone(), latency_ms: path.len() as u32 * 15,
                    reason: "route_found".into(),
                };
            }
            if path.len() >= MESH_HOP_TTL as usize { continue; }

            let neighbors = self.bastions.get(&current)
                .map(|b| b.neighbors.clone()).unwrap_or_default();
            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor.clone());
                    let mut new_path = path.clone();
                    new_path.push(neighbor.clone());
                    queue.push_back((neighbor, new_path));
                }
            }
        }
        MeshRouteResult {
            success: false, hops: 0, path: vec![],
            latency_ms: 0, reason: "no_route".into(),
        }
    }

    pub fn city_stats(&self) -> CityStats {
        let total_droids: usize = self.bastions.values()
            .map(|b| b.droids.len()).sum();
        let active_bastions = self.bastions.values()
            .filter(|b| b.mesh_active).count();
        CityStats {
            city: self.city.clone(),
            total_bastions: self.bastions.len(),
            active_bastions, total_droids,
            total_relayed: self.total_relayed,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeshRouteResult {
    pub success: bool, pub hops: u8,
    pub path: Vec<String>, pub latency_ms: u32,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CityStats {
    pub city: String,
    pub total_bastions: usize,
    pub active_bastions: usize,
    pub total_droids: usize,
    pub total_relayed: u64,
}
