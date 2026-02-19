// =============================================================================
// FEDERATION CORE — inventory.rs
// PHASE 5 / STEP 6 — «Iron Discipline — Hardware Classification Protocol»
// =============================================================================
//
// Каждое устройство автоматически получает роль по характеристикам железа.
// Роль определяет: какие модули запускать, сколько трафика держать,
// в каком слое сети работать.
//
// Классификация:
//   Sentinel   — мощный сервер (≥16 CPU, ≥32GB RAM) → ядро Федерации
//   Citadel    — средний сервер (≥8 CPU, ≥16GB RAM) → региональный хаб
//   Workstation— десктоп (≥4 CPU, ≥8GB RAM) → полный узел
//   Mobile     — телефон/планшет (≥2 CPU, ≥2GB RAM) → лёгкий узел
//   Ghost      — старое железо (любое) → шум + приманки
//   Droid      — IoT/роутер (≤2 CPU, ≤512MB RAM) → меш-реле
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// HardwareProfile — характеристики железа
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub device_id: String,
    pub cpu_cores: u32,
    pub cpu_mhz: u32,
    pub ram_mb: u32,
    pub storage_gb: u32,
    pub bandwidth_mbps: u32,
    pub has_gpu: bool,
    pub battery_powered: bool,
    pub arch: CpuArch,
    pub os: OsType,
    pub uptime_days: u32,
    pub is_tor_capable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CpuArch {
    X86_64,
    Arm64,
    ArmV7,
    Mips,
    RiscV,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OsType {
    Linux,
    Windows,
    MacOs,
    Android,
    Ios,
    OpenWrt,
    FreeBsd,
    Unknown,
}

impl HardwareProfile {
    pub fn compute_score(&self) -> f64 {
        let cpu_score  = (self.cpu_cores as f64 * self.cpu_mhz as f64 / 1000.0).min(100.0);
        let ram_score  = (self.ram_mb as f64 / 1024.0).min(64.0);
        let bw_score   = (self.bandwidth_mbps as f64 / 100.0).min(10.0);
        let gpu_bonus  = if self.has_gpu { 10.0 } else { 0.0 };
        let uptime_bonus = (self.uptime_days as f64).sqrt().min(10.0);
        cpu_score * 0.4 + ram_score * 0.3 + bw_score * 0.2 + gpu_bonus + uptime_bonus
    }

    pub fn is_stable(&self) -> bool {
        !self.battery_powered && self.uptime_days > 7
    }
}

// -----------------------------------------------------------------------------
// DeviceRole — роль устройства в Федерации
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceRole {
    Sentinel,    // ядро — мощный сервер
    Citadel,     // региональный хаб
    Workstation, // полный узел
    Mobile,      // лёгкий узел
    Ghost,       // шум + приманки (старое железо)
    Droid,       // меш-реле (IoT/роутер)
}

impl DeviceRole {
    pub fn name(&self) -> &str {
        match self {
            DeviceRole::Sentinel    => "⚔️  Sentinel",
            DeviceRole::Citadel     => "🏰 Citadel",
            DeviceRole::Workstation => "🖥️  Workstation",
            DeviceRole::Mobile      => "📱 Mobile",
            DeviceRole::Ghost       => "👻 Ghost",
            DeviceRole::Droid       => "🤖 Droid",
        }
    }

    pub fn max_connections(&self) -> u32 {
        match self {
            DeviceRole::Sentinel    => 10_000,
            DeviceRole::Citadel     => 1_000,
            DeviceRole::Workstation => 100,
            DeviceRole::Mobile      => 10,
            DeviceRole::Ghost       => 5,
            DeviceRole::Droid       => 50,
        }
    }

    pub fn bandwidth_allocation(&self) -> f64 {
        match self {
            DeviceRole::Sentinel    => 1.00,  // 100% доступной полосы
            DeviceRole::Citadel     => 0.80,
            DeviceRole::Workstation => 0.50,
            DeviceRole::Mobile      => 0.20,
            DeviceRole::Ghost       => 0.05,  // минимум — только шум
            DeviceRole::Droid       => 0.30,  // меш реле
        }
    }

    pub fn enabled_modules(&self) -> Vec<&str> {
        match self {
            DeviceRole::Sentinel => vec![
                "neural_node", "federated", "mutation", "transport",
                "dag", "zkp", "overlay", "governance", "oracle",
                "credits", "market", "reputation", "mint", "vault",
            ],
            DeviceRole::Citadel => vec![
                "neural_node", "federated", "mutation", "transport",
                "dag", "zkp", "overlay", "credits", "market",
            ],
            DeviceRole::Workstation => vec![
                "neural_node", "mutation", "transport", "dag", "credits",
            ],
            DeviceRole::Mobile => vec![
                "transport", "mutation", "credits",
            ],
            DeviceRole::Ghost => vec![
                "transport",  // только шум и приманки
            ],
            DeviceRole::Droid => vec![
                "transport", "p2p",  // только relay
            ],
        }
    }

    pub fn primary_function(&self) -> &str {
        match self {
            DeviceRole::Sentinel    => "Ядро Федерации — полный стек",
            DeviceRole::Citadel     => "Региональный хаб — агрегация",
            DeviceRole::Workstation => "Полный узел — маршрутизация",
            DeviceRole::Mobile      => "Лёгкий узел — доставка",
            DeviceRole::Ghost       => "Шум + приманки — маскировка",
            DeviceRole::Droid       => "Меш-реле — расширение сети",
        }
    }

    pub fn layer(&self) -> u8 {
        match self {
            DeviceRole::Sentinel    => 1,  // L1 — ядро
            DeviceRole::Citadel     => 2,  // L2 — хабы
            DeviceRole::Workstation => 3,  // L3 — узлы
            DeviceRole::Mobile      => 4,  // L4 — клиенты
            DeviceRole::Ghost       => 5,  // L5 — шум
            DeviceRole::Droid       => 3,  // L3 — реле (наравне с узлами)
        }
    }
}

// -----------------------------------------------------------------------------
// RoleClassifier — автоматическое назначение ролей
// -----------------------------------------------------------------------------

pub struct RoleClassifier;

impl RoleClassifier {
    pub fn classify(hw: &HardwareProfile) -> DeviceRole {
        // IoT/роутер — определяем по RAM и ОС
        if hw.ram_mb <= 512 || hw.os == OsType::OpenWrt {
            return DeviceRole::Droid;
        }
        // Мобильные устройства
        if hw.battery_powered || hw.os == OsType::Android || hw.os == OsType::Ios {
            if hw.cpu_cores >= 2 && hw.ram_mb >= 2048 {
                return DeviceRole::Mobile;
            }
            return DeviceRole::Ghost;
        }
        // Классификация по мощности
        let score = hw.compute_score();
        if hw.cpu_cores >= 16 && hw.ram_mb >= 32768 && hw.is_stable() {
            DeviceRole::Sentinel
        } else if hw.cpu_cores >= 8 && hw.ram_mb >= 16384 && hw.is_stable() {
            DeviceRole::Citadel
        } else if hw.cpu_cores >= 4 && hw.ram_mb >= 8192 {
            DeviceRole::Workstation
        } else if score > 5.0 {
            DeviceRole::Ghost  // старое железо → шум
        } else {
            DeviceRole::Ghost
        }
    }

    pub fn classify_batch(devices: &[HardwareProfile]) -> Vec<(&HardwareProfile, DeviceRole)> {
        devices.iter().map(|hw| (hw, Self::classify(hw))).collect()
    }
}

// -----------------------------------------------------------------------------
// NodeCapacity — вычисляемая мощность узла
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapacity {
    pub device_id: String,
    pub role: DeviceRole,
    pub hardware_score: f64,
    pub max_connections: u32,
    pub bandwidth_alloc_mbps: f64,
    pub enabled_modules: Vec<String>,
    pub layer: u8,
    pub estimated_bypass_rate: f64,  // прорывов в секунду
    pub decoy_capacity: u32,         // кол-во одновременных коробочек
    pub can_run_aiki: bool,
    pub can_run_zk: bool,
}

impl NodeCapacity {
    pub fn from_profile(hw: &HardwareProfile) -> Self {
        let role = RoleClassifier::classify(hw);
        let score = hw.compute_score();
        let bw_alloc = hw.bandwidth_mbps as f64 * role.bandwidth_allocation();
        let bypass_rate = match &role {
            DeviceRole::Sentinel    => 1000.0,
            DeviceRole::Citadel     => 200.0,
            DeviceRole::Workstation => 50.0,
            DeviceRole::Mobile      => 10.0,
            DeviceRole::Ghost       => 2.0,
            DeviceRole::Droid       => 20.0,
        };
        let decoy_cap = (hw.ram_mb / 256).min(1000);
        let can_aiki = hw.cpu_cores >= 4 && hw.ram_mb >= 4096;
        let can_zk   = hw.cpu_cores >= 2 && hw.ram_mb >= 1024;

        NodeCapacity {
            device_id: hw.device_id.clone(),
            role: role.clone(),
            hardware_score: score,
            max_connections: role.max_connections(),
            bandwidth_alloc_mbps: bw_alloc,
            enabled_modules: role.enabled_modules().iter()
                .map(|s| s.to_string()).collect(),
            layer: role.layer(),
            estimated_bypass_rate: bypass_rate,
            decoy_capacity: decoy_cap,
            can_run_aiki: can_aiki,
            can_run_zk: can_zk,
        }
    }
}

// -----------------------------------------------------------------------------
// FederationInventory — реестр всего железа
// -----------------------------------------------------------------------------

pub struct FederationInventory {
    pub devices: HashMap<String, HardwareProfile>,
    pub capacities: HashMap<String, NodeCapacity>,
    pub role_counts: HashMap<String, u32>,
}

impl FederationInventory {
    pub fn new() -> Self {
        FederationInventory {
            devices: HashMap::new(),
            capacities: HashMap::new(),
            role_counts: HashMap::new(),
        }
    }

    pub fn register(&mut self, hw: HardwareProfile) -> &NodeCapacity {
        let capacity = NodeCapacity::from_profile(&hw);
        let role_name = capacity.role.name().to_string();
        *self.role_counts.entry(role_name).or_insert(0) += 1;
        self.devices.insert(hw.device_id.clone(), hw);
        self.capacities.insert(capacity.device_id.clone(), capacity);
        self.capacities.values().last().unwrap()
    }

    pub fn get_by_role(&self, role: &DeviceRole) -> Vec<&NodeCapacity> {
        self.capacities.values()
            .filter(|c| &c.role == role)
            .collect()
    }

    pub fn network_topology(&self) -> TopologyStats {
        let total = self.capacities.len();
        let total_bw: f64 = self.capacities.values()
            .map(|c| c.bandwidth_alloc_mbps).sum();
        let total_bypass: f64 = self.capacities.values()
            .map(|c| c.estimated_bypass_rate).sum();
        let sentinels  = self.get_by_role(&DeviceRole::Sentinel).len();
        let citadels   = self.get_by_role(&DeviceRole::Citadel).len();
        let workers    = self.get_by_role(&DeviceRole::Workstation).len();
        let mobiles    = self.get_by_role(&DeviceRole::Mobile).len();
        let ghosts     = self.get_by_role(&DeviceRole::Ghost).len();
        let droids     = self.get_by_role(&DeviceRole::Droid).len();
        let noise_ratio = (ghosts + droids) as f64 / total.max(1) as f64;

        TopologyStats {
            total_devices: total,
            sentinels, citadels, workers, mobiles, ghosts, droids,
            total_bandwidth_mbps: total_bw,
            total_bypass_rate: total_bypass,
            noise_ratio,
            aiki_capable: self.capacities.values()
                .filter(|c| c.can_run_aiki).count(),
            zk_capable: self.capacities.values()
                .filter(|c| c.can_run_zk).count(),
        }
    }

    pub fn auto_assign_regions(&self) -> Vec<RegionAssignment> {
        // Sentinel и Citadel становятся региональными координаторами
        let mut assignments = vec![];
        let regions = ["EU", "AS", "AM", "AF", "OC"];
        let hubs: Vec<&NodeCapacity> = self.capacities.values()
            .filter(|c| c.role == DeviceRole::Sentinel
                     || c.role == DeviceRole::Citadel)
            .collect();
        for (i, hub) in hubs.iter().enumerate() {
            assignments.push(RegionAssignment {
                device_id: hub.device_id.clone(),
                role: hub.role.clone(),
                region: regions[i % regions.len()].to_string(),
                layer: hub.layer,
            });
        }
        assignments
    }
}

impl Default for FederationInventory { fn default() -> Self { Self::new() } }

#[derive(Debug, Serialize, Deserialize)]
pub struct TopologyStats {
    pub total_devices: usize,
    pub sentinels: usize,
    pub citadels: usize,
    pub workers: usize,
    pub mobiles: usize,
    pub ghosts: usize,
    pub droids: usize,
    pub total_bandwidth_mbps: f64,
    pub total_bypass_rate: f64,
    pub noise_ratio: f64,
    pub aiki_capable: usize,
    pub zk_capable: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegionAssignment {
    pub device_id: String,
    pub role: DeviceRole,
    pub region: String,
    pub layer: u8,
}

impl std::fmt::Display for TopologyStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════════════╗\n\
             ║  FEDERATION INVENTORY — TOPOLOGY                     ║\n\
             ╠══════════════════════════════════════════════════════╣\n\
             ║  Всего: {:>4}  BW: {:>8.1}Mbps  Bypass: {:>6.0}/s  ║\n\
             ║  ⚔️  Sentinel:{:>3}  🏰 Citadel:{:>3}  🖥️  Work:{:>3}   ║\n\
             ║  📱 Mobile: {:>3}  👻 Ghost: {:>3}  🤖 Droid:{:>3}   ║\n\
             ║  Шум: {:.0}%  Aiki: {:>3}  ZK: {:>3}              ║\n\
             ╚══════════════════════════════════════════════════════╝",
            self.total_devices, self.total_bandwidth_mbps,
            self.total_bypass_rate,
            self.sentinels, self.citadels, self.workers,
            self.mobiles, self.ghosts, self.droids,
            self.noise_ratio * 100.0,
            self.aiki_capable, self.zk_capable,
        )
    }
}
