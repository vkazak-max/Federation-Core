// =============================================================================
// FEDERATION CORE — vault.rs
// PHASE 5 / STEP 5 — «Cryptographic Vault & Sharded Key Storage»
// =============================================================================
//
// Hot/Cold хранилища с ZK-доказательствами доступа.
// Sharded Keys — ключи Ветеранов дробятся на N осколков (Shamir's Secret Sharing)
// и прячутся в памяти тысяч Ghost-узлов.
//
// Схема Шамира: секрет S делится на N осколков, любые K восстанавливают S.
// Ghost узлы: хранят осколок не зная что это и чей он.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const HOT_VAULT_LIMIT: usize   = 1_000;   // max записей в Hot
pub const COLD_VAULT_LIMIT: usize  = 100_000;  // max записей в Cold
pub const DEFAULT_SHARDS: usize    = 5;        // N осколков
pub const DEFAULT_THRESHOLD: usize = 3;        // K для восстановления
pub const GHOST_MEMORY_KB: usize   = 4;        // размер осколка в памяти Ghost
pub const ZK_PROOF_SIZE: usize     = 32;       // байт ZK-доказательства

// -----------------------------------------------------------------------------
// VaultTier — уровень хранилища
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VaultTier {
    Hot,   // быстрый доступ, онлайн, зашифрован AES
    Cold,  // офлайн, максимальная защита, ZK-доступ
    Ghost, // осколочное хранение в сети Ghost-узлов
}

impl VaultTier {
    pub fn name(&self) -> &str {
        match self {
            VaultTier::Hot   => "🔥 Hot",
            VaultTier::Cold  => "🧊 Cold",
            VaultTier::Ghost => "👻 Ghost",
        }
    }
    pub fn access_time_ms(&self) -> u32 {
        match self {
            VaultTier::Hot   => 1,
            VaultTier::Cold  => 5000,
            VaultTier::Ghost => 30000,
        }
    }
}

// -----------------------------------------------------------------------------
// ZkProof — нулевое знание доступа
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProof {
    pub proof_hash: String,
    pub commitment: String,
    pub nullifier: String,
    pub valid: bool,
    pub expires_at: i64,
}

impl ZkProof {
    pub fn generate(owner_id: &str, secret: &[u8], rng: &mut u64) -> Self {
        // Упрощённый ZK — в production заменить на настоящий ZKP
        *rng ^= *rng << 13; *rng ^= *rng >> 7; *rng ^= *rng << 17;
        let commitment = format!("commit_{:016x}", *rng ^ owner_id.len() as u64);
        *rng ^= *rng << 13; *rng ^= *rng >> 7;
        let nullifier = format!("null_{:016x}", *rng);
        let proof_hash = format!("zkp_{:08x}{:08x}",
            secret.iter().fold(0u32, |a, &b| a.wrapping_add(b as u32)),
            owner_id.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32)));

        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64;

        ZkProof {
            proof_hash, commitment, nullifier,
            valid: true, expires_at: now + 3_600_000, // 1 час
        }
    }

    pub fn verify(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64;
        self.valid && now < self.expires_at
    }
}

// -----------------------------------------------------------------------------
// VaultEntry — запись в хранилище
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub key_id: String,
    pub owner_id: String,
    pub tier: VaultTier,
    pub encrypted_payload: Vec<u8>,  // XOR-зашифровано (в prod — ChaCha20)
    pub proof: ZkProof,
    pub created_at: i64,
    pub accessed_at: i64,
    pub access_count: u32,
    pub reputation_required: f64,    // минимальная репутация для доступа
}

impl VaultEntry {
    pub fn size_bytes(&self) -> usize {
        self.encrypted_payload.len() + 256 // overhead
    }
}

// -----------------------------------------------------------------------------
// KeyShard — осколок ключа для Ghost хранения
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShard {
    pub shard_id: u8,
    pub total_shards: u8,
    pub threshold: u8,
    pub ghost_node_id: String,
    pub shard_data: Vec<u8>,         // осколок данных
    pub key_commitment: String,      // commitment к исходному ключу
    pub owner_commitment: String,    // commitment к владельцу (Ghost не знает чей)
    pub is_decoy: bool,              // ложный осколок для маскировки
}

impl KeyShard {
    pub fn memory_kb(&self) -> f64 {
        self.shard_data.len() as f64 / 1024.0
    }
}


// GF(256) арифметика для схемы Шамира
fn gf_add(a: u8, b: u8) -> u8 { a ^ b }

fn gf_mul(a: u8, b: u8) -> u8 {
    let mut p = 0u8;
    let mut a = a;
    let mut b = b;
    for _ in 0..8 {
        if b & 1 != 0 { p ^= a; }
        let carry = a & 0x80;
        a <<= 1;
        if carry != 0 { a ^= 0x1b; } // примитивный полином x^8+x^4+x^3+x+1
        b >>= 1;
    }
    p
}

fn gf_inv(a: u8) -> u8 {
    if a == 0 { return 0; }
    let mut result = 1u8;
    let mut base = a;
    let mut exp = 254u8; // a^(256-2) = a^(-1) по малой теореме Ферма
    while exp > 0 {
        if exp & 1 != 0 { result = gf_mul(result, base); }
        base = gf_mul(base, base);
        exp >>= 1;
    }
    result
}

// Схема Шамира (упрощённая над GF(256))
pub struct ShamirScheme;

impl ShamirScheme {
    /// Разделить секрет на N осколков, K достаточно для восстановления
    pub fn split(secret: &[u8], n: usize, k: usize,
                 rng: &mut u64) -> Vec<Vec<u8>> {
        let mut shards = vec![vec![0u8; secret.len()]; n];

        // Генерируем k-1 случайных полиномов
        let coeffs: Vec<Vec<u8>> = (0..k-1).map(|_| {
            (0..secret.len()).map(|_| {
                *rng ^= *rng << 13; *rng ^= *rng >> 7; *rng ^= *rng << 17;
                (*rng & 0xff) as u8
            }).collect()
        }).collect();

        for i in 0..n {
            let x = (i + 1) as u8;
            for j in 0..secret.len() {
                // P(x) = secret XOR c1*x XOR c2*x^2 XOR ... в GF(256)
                let mut val = secret[j];
                let mut xpow: u8 = x;
                for coeff in &coeffs {
                    val = gf_add(val, gf_mul(coeff[j], xpow));
                    xpow = gf_mul(xpow, x);
                }
                shards[i][j] = val;
            }
        }
        shards
    }

    /// Восстановить секрет из K осколков — GF(256) арифметика
    pub fn reconstruct(shards: &[(u8, Vec<u8>)]) -> Vec<u8> {
        if shards.is_empty() { return vec![]; }
        let len = shards[0].1.len();
        let mut secret = vec![0u8; len];

        for j in 0..len {
            let mut val: u8 = 0;
            for (i, (xi, yi)) in shards.iter().enumerate() {
                let mut num: u8 = yi[j];
                let mut den: u8 = 1;
                for (k, (xk, _)) in shards.iter().enumerate() {
                    if i != k {
                        num = gf_mul(num, *xk);
                        den = gf_mul(den, gf_add(*xi, *xk));
                    }
                }
                val = gf_add(val, gf_mul(num, gf_inv(den)));
            }
            secret[j] = val;
        }
        secret
    }
}

// -----------------------------------------------------------------------------
// GhostNetwork — сеть Ghost-узлов хранящих осколки
// -----------------------------------------------------------------------------

pub struct GhostNetwork {
    pub nodes: HashMap<String, Vec<KeyShard>>,  // ghost_id → осколки
    pub total_shards: u64,
    pub total_decoys: u64,
}

impl GhostNetwork {
    pub fn new() -> Self {
        GhostNetwork {
            nodes: HashMap::new(),
            total_shards: 0,
            total_decoys: 0,
        }
    }

    pub fn register_ghost(&mut self, ghost_id: &str) {
        self.nodes.entry(ghost_id.to_string()).or_default();
    }

    pub fn store_shard(&mut self, ghost_id: &str, shard: KeyShard) {
        if shard.is_decoy { self.total_decoys += 1; }
        else { self.total_shards += 1; }
        self.nodes.entry(ghost_id.to_string())
            .or_default().push(shard);
    }

    pub fn retrieve_shard(&self, ghost_id: &str,
                           key_commitment: &str) -> Option<&KeyShard> {
        self.nodes.get(ghost_id)?
            .iter()
            .find(|s| s.key_commitment == key_commitment && !s.is_decoy)
    }

    pub fn total_memory_kb(&self) -> f64 {
        self.nodes.values()
            .flat_map(|shards| shards.iter())
            .map(|s| s.memory_kb()).sum()
    }

    pub fn ghost_count(&self) -> usize { self.nodes.len() }
}

impl Default for GhostNetwork { fn default() -> Self { Self::new() } }

// -----------------------------------------------------------------------------
// CryptoVault — главное хранилище
// -----------------------------------------------------------------------------

pub struct CryptoVault {
    pub hot: HashMap<String, VaultEntry>,
    pub cold: HashMap<String, VaultEntry>,
    pub ghost_network: GhostNetwork,
    pub shard_index: HashMap<String, Vec<(u8, String)>>, // key_id → [(shard_id, ghost_id)]
    pub total_entries: u64,
    pub total_zk_proofs: u64,
    rng: u64,
}

impl CryptoVault {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_nanos() as u64;
        CryptoVault {
            hot: HashMap::new(), cold: HashMap::new(),
            ghost_network: GhostNetwork::new(),
            shard_index: HashMap::new(),
            total_entries: 0, total_zk_proofs: 0,
            rng: seed ^ 0xdeadbeef_cafebabe,
        }
    }

    fn encrypt(&mut self, data: &[u8]) -> Vec<u8> {
        // XOR stream cipher (в prod — ChaCha20)
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let key = self.rng.to_le_bytes();
        data.iter().enumerate()
            .map(|(i, &b)| b ^ key[i % 8]).collect()
    }

    fn now() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64
    }

    /// Положить в Hot vault
    pub fn store_hot(&mut self, key_id: &str, owner_id: &str,
                     payload: &[u8], rep_required: f64) -> ZkProof {
        let encrypted = self.encrypt(payload);
        let proof = ZkProof::generate(owner_id, payload, &mut self.rng);
        let now = Self::now();
        let entry = VaultEntry {
            key_id: key_id.to_string(),
            owner_id: owner_id.to_string(),
            tier: VaultTier::Hot,
            encrypted_payload: encrypted,
            proof: proof.clone(),
            created_at: now, accessed_at: now,
            access_count: 0, reputation_required: rep_required,
        };
        if self.hot.len() < HOT_VAULT_LIMIT {
            self.hot.insert(key_id.to_string(), entry);
            self.total_entries += 1;
            self.total_zk_proofs += 1;
        }
        proof
    }

    /// Положить в Cold vault
    pub fn store_cold(&mut self, key_id: &str, owner_id: &str,
                      payload: &[u8], rep_required: f64) -> ZkProof {
        let encrypted = self.encrypt(payload);
        let proof = ZkProof::generate(owner_id, payload, &mut self.rng);
        let now = Self::now();
        let entry = VaultEntry {
            key_id: key_id.to_string(),
            owner_id: owner_id.to_string(),
            tier: VaultTier::Cold,
            encrypted_payload: encrypted,
            proof: proof.clone(),
            created_at: now, accessed_at: now,
            access_count: 0, reputation_required: rep_required,
        };
        if self.cold.len() < COLD_VAULT_LIMIT {
            self.cold.insert(key_id.to_string(), entry);
            self.total_entries += 1;
            self.total_zk_proofs += 1;
        }
        proof
    }

    /// Осколочное хранение — Veteran ключ дробится по Ghost-узлам
    pub fn shard_to_ghosts(&mut self, key_id: &str, owner_id: &str,
                            payload: &[u8], ghost_ids: &[&str],
                            n: usize, k: usize) -> ShardingResult {
        let shards = ShamirScheme::split(payload, n, k, &mut self.rng);

        self.rng ^= self.rng << 13;
        let commitment = format!("commit_{:016x}", self.rng ^ payload.len() as u64);
        let owner_commit = format!("owner_{:016x}", self.rng
            ^ owner_id.bytes().fold(0u64, |a, b| a.wrapping_add(b as u64)));

        let mut shard_map = vec![];

        for (i, shard_data) in shards.iter().enumerate() {
            let ghost_id = ghost_ids[i % ghost_ids.len()];

            // Реальный осколок
            let shard = KeyShard {
                shard_id: (i + 1) as u8,
                total_shards: n as u8,
                threshold: k as u8,
                ghost_node_id: ghost_id.to_string(),
                shard_data: shard_data.clone(),
                key_commitment: commitment.clone(),
                owner_commitment: owner_commit.clone(),
                is_decoy: false,
            };
            self.ghost_network.store_shard(ghost_id, shard);
            shard_map.push((i as u8 + 1, ghost_id.to_string()));

            // Ложные осколки для маскировки
            for _ in 0..2 {
                self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7;
                let decoy_data: Vec<u8> = (0..shard_data.len())
                    .map(|_| { self.rng ^= self.rng << 17; (self.rng & 0xff) as u8 }).collect();
                let decoy = KeyShard {
                    shard_id: (i + 1) as u8,
                    total_shards: n as u8,
                    threshold: k as u8,
                    ghost_node_id: ghost_id.to_string(),
                    shard_data: decoy_data,
                    key_commitment: format!("decoy_{:08x}", self.rng),
                    owner_commitment: format!("decoy_owner_{:08x}", self.rng),
                    is_decoy: true,
                };
                self.ghost_network.store_shard(ghost_id, decoy);
            }
        }

        self.shard_index.insert(key_id.to_string(), shard_map.clone());
        self.total_entries += 1;

        ShardingResult {
            key_id: key_id.to_string(),
            commitment: commitment.clone(),
            total_shards: n,
            threshold: k,
            ghost_nodes: ghost_ids.iter().take(n).map(|s| s.to_string()).collect(),
            decoy_shards: n * 2,
        }
    }

    /// Получить из Hot vault с ZK проверкой
    pub fn retrieve_hot(&mut self, key_id: &str,
                         proof: &ZkProof, owner_rep: f64) -> VaultResult {
        if !proof.verify() {
            return VaultResult::denied("ZK proof истёк");
        }
        match self.hot.get_mut(key_id) {
            None => VaultResult::denied("Ключ не найден в Hot vault"),
            Some(entry) => {
                if owner_rep < entry.reputation_required {
                    return VaultResult::denied(&format!(
                        "Недостаточная репутация: {:.1} < {:.1}",
                        owner_rep, entry.reputation_required));
                }
                entry.access_count += 1;
                entry.accessed_at = Self::now();
                VaultResult::success(
                    entry.encrypted_payload.clone(),
                    VaultTier::Hot, entry.access_count)
            }
        }
    }

    pub fn vault_stats(&self) -> VaultStats {
        VaultStats {
            hot_entries: self.hot.len(),
            cold_entries: self.cold.len(),
            ghost_nodes: self.ghost_network.ghost_count(),
            total_shards: self.ghost_network.total_shards,
            total_decoys: self.ghost_network.total_decoys,
            ghost_memory_kb: self.ghost_network.total_memory_kb(),
            total_zk_proofs: self.total_zk_proofs,
        }
    }
}

impl Default for CryptoVault { fn default() -> Self { Self::new() } }

// -----------------------------------------------------------------------------
// VaultResult / ShardingResult / VaultStats
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultResult {
    pub success: bool,
    pub payload: Option<Vec<u8>>,
    pub tier: Option<VaultTier>,
    pub access_count: u32,
    pub reason: String,
}

impl VaultResult {
    pub fn success(payload: Vec<u8>, tier: VaultTier, count: u32) -> Self {
        VaultResult { success: true, payload: Some(payload),
            tier: Some(tier), access_count: count, reason: "OK".into() }
    }
    pub fn denied(reason: &str) -> Self {
        VaultResult { success: false, payload: None,
            tier: None, access_count: 0, reason: reason.to_string() }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShardingResult {
    pub key_id: String,
    pub commitment: String,
    pub total_shards: usize,
    pub threshold: usize,
    pub ghost_nodes: Vec<String>,
    pub decoy_shards: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStats {
    pub hot_entries: usize,
    pub cold_entries: usize,
    pub ghost_nodes: usize,
    pub total_shards: u64,
    pub total_decoys: u64,
    pub ghost_memory_kb: f64,
    pub total_zk_proofs: u64,
}

impl std::fmt::Display for VaultStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  CRYPTO VAULT — STATS                        ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Hot:    {:>4}  Cold:   {:>6}               ║\n\
             ║  Ghost:  {:>4}  Shards: {:>6}  Decoys: {:>4} ║\n\
             ║  Memory: {:>8.2} KB  ZK proofs: {:>6}     ║\n\
             ╚══════════════════════════════════════════════╝",
            self.hot_entries, self.cold_entries,
            self.ghost_nodes, self.total_shards, self.total_decoys,
            self.ghost_memory_kb, self.total_zk_proofs,
        )
    }
}
