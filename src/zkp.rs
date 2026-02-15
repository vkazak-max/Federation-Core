// =============================================================================
// FEDERATION CORE — zkp.rs
// PHASE 2 / WEEK 6 — «Zero-Knowledge Basics» (MVP)
// =============================================================================
//
// MVP-реализация:
//
//   1) Onion layers (nested payload): слой шифрует не просто байты,
//      а сериализованный OnionPayload { Layer(next_layer) | Data(final_bytes) }.
//
//   2) key_id в каждом слое, чтобы узел мог выбрать правильный SessionKey.
//
//   3) Integrity tag (MVP): простая проверка целостности ciphertext.
//
// ⚠️ ВАЖНО: XOR + FNV = НЕ криптография production.
// В production: X25519 + ChaCha20-Poly1305 + настоящие commitments/zk.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// -----------------------------------------------------------------------------
// Константы
// -----------------------------------------------------------------------------

/// Размер ключа шифрования (байт)
pub const KEY_SIZE: usize = 32;
/// Размер nonce (байт)
pub const NONCE_SIZE: usize = 16;
/// Максимальное число слоёв onion
pub const MAX_ONION_LAYERS: usize = 8;

// -----------------------------------------------------------------------------
// Утилиты (PRNG + hash) — детерминированные, без внешних зависимостей
// -----------------------------------------------------------------------------

fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn generate_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    let mut result = Vec::with_capacity(len);
    while result.len() < len {
        let val = xorshift64(&mut state);
        for b in val.to_le_bytes() {
            result.push(b);
            if result.len() == len {
                break;
            }
        }
    }
    result
}

/// FNV-1a 64bit
pub fn fnv_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

// -----------------------------------------------------------------------------
// SessionKey — сессионный ключ шифрования
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKey {
    pub key_bytes: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_id: String,
}

impl SessionKey {
    pub fn generate() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let key_bytes = generate_bytes(seed, KEY_SIZE);
        let nonce = generate_bytes(seed ^ 0xdeadbeef, NONCE_SIZE);
        let key_id = to_hex(&generate_bytes(seed ^ 0xcafe, 8));

        SessionKey { key_bytes, nonce, key_id }
    }

    /// Симуляция shared secret. В production: X25519 ECDH.
    pub fn from_shared_secret(our_secret: u64, their_public: u64) -> Self {
        let shared = our_secret.wrapping_mul(their_public) ^ 0x9e3779b97f4a7c15;
        let key_bytes = generate_bytes(shared, KEY_SIZE);
        let nonce = generate_bytes(shared ^ 0xdeadbeef, NONCE_SIZE);
        let key_id = to_hex(&generate_bytes(shared ^ 0xcafe, 8));
        SessionKey { key_bytes, nonce, key_id }
    }

    /// XOR-шифрование (MVP).
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let ks_seed = fnv_hash(&self.key_bytes) ^ fnv_hash(&self.nonce);
        let keystream = generate_bytes(ks_seed, plaintext.len());
        plaintext
            .iter()
            .zip(keystream.iter())
            .map(|(p, k)| p ^ k)
            .collect()
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8> {
        self.encrypt(ciphertext) // XOR симметричен
    }
}

// -----------------------------------------------------------------------------
// KeyRegistry — хранение ключей по key_id (нужно для peel/forward)
// -----------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct KeyRegistry {
    keys: HashMap<String, SessionKey>,
}

impl KeyRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, key: SessionKey) {
        self.keys.insert(key.key_id.clone(), key);
    }

    pub fn get(&self, key_id: &str) -> Option<&SessionKey> {
        self.keys.get(key_id)
    }

    pub fn len(&self) -> usize { self.keys.len() }
}

// -----------------------------------------------------------------------------
// ZkpCommitment — MVP commitment (НЕ production)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpCommitment {
    pub commitment: String,
    pub blinding_factor: u64,
    value_hash: u64,
}

impl ZkpCommitment {
    pub fn commit(value: &[u8]) -> Self {
        let blind = (now_ms() as u64) ^ 0xfeedface;
        let value_hash = fnv_hash(value);

        let mut combined = value.to_vec();
        combined.extend_from_slice(&blind.to_le_bytes());

        let commitment = to_hex(&generate_bytes(fnv_hash(&combined), 32));
        ZkpCommitment { commitment, blinding_factor: blind, value_hash }
    }

    pub fn verify(&self, value: &[u8]) -> bool {
        if fnv_hash(value) != self.value_hash {
            return false;
        }
        let mut combined = value.to_vec();
        combined.extend_from_slice(&self.blinding_factor.to_le_bytes());
        let expected = to_hex(&generate_bytes(fnv_hash(&combined), 32));
        expected == self.commitment
    }
}

// -----------------------------------------------------------------------------
// Onion structures
// -----------------------------------------------------------------------------

/// То, что реально лежит внутри encrypted_payload после decrypt.
/// Либо следующий слой, либо финальные данные.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum OnionPayload {
    Layer(OnionLayer),
    Data(Vec<u8>),
}

/// Один слой onion-маршрутизации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnionLayer {
    /// Каким ключом этот слой должен быть расшифрован
    pub key_id: String,
    /// Следующий узел (единственное, что должен узнать текущий узел ПОСЛЕ peel)
    pub next_hop: String,
    /// Зашифрованный OnionPayload
    pub encrypted_payload: Vec<u8>,
    /// Integrity tag (MVP): хеш ciphertext
    pub integrity_tag: String,
    /// Nullifier — предотвращает replay
    pub nullifier: String,
}

/// Полный onion-пакет: внешняя оболочка + мета
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnionPacket {
    pub outer_layer: OnionLayer,
    pub layer_count: usize,
    pub sender_commitment: String,
    pub created_at: i64,
}

/// Результат peel
#[derive(Debug)]
pub struct PeeledResult {
    pub next_hop: String,
    pub is_final: bool,
    /// Если is_final=true -> final_data
    pub final_data: Option<Vec<u8>>,
    /// Если is_final=false -> следующий слой (для форварда)
    pub next_layer: Option<OnionLayer>,
}

// -----------------------------------------------------------------------------
// OnionBuilder
// -----------------------------------------------------------------------------

pub struct OnionBuilder {
    route: Vec<String>,
    keys: Vec<SessionKey>,
}

impl OnionBuilder {
    pub fn new() -> Self {
        OnionBuilder { route: vec![], keys: vec![] }
    }

    /// Задать маршрут и сгенерировать ключи по количеству hop
    pub fn with_route(mut self, route: Vec<String>) -> Self {
        self.keys = route.iter().map(|_| SessionKey::generate()).collect();
        self.route = route;
        self
    }

    /// build возвращает:
    /// - OnionPacket (для отправки)
    /// - Vec<SessionKey> (ключи по каждому hop: route[i] -> keys[i])
    pub fn build(self, payload: &[u8]) -> Result<(OnionPacket, Vec<SessionKey>), String> {
        if self.route.is_empty() {
            return Err("Маршрут не задан".to_string());
        }
        if self.route.len() > MAX_ONION_LAYERS {
            return Err(format!(
                "Слишком много слоёв: {} > {}",
                self.route.len(),
                MAX_ONION_LAYERS
            ));
        }

        let layer_count = self.route.len();

        // Начинаем с финального payload
        let mut inner = OnionPayload::Data(payload.to_vec());

        // Строим от последнего hop к первому
        let mut built_layers: Vec<OnionLayer> = Vec::with_capacity(layer_count);

        for i in (0..layer_count).rev() {
            let key = &self.keys[i];

            let next_hop = if i + 1 < layer_count {
                self.route[i + 1].clone()
            } else {
                "DESTINATION".to_string()
            };

            let inner_bytes = serde_json::to_vec(&inner)
                .map_err(|e| format!("Onion serialize error: {}", e))?;

            let ciphertext = key.encrypt(&inner_bytes);

            // Integrity tag (MVP) = hash(ciphertext)
            let integrity_tag = to_hex(&generate_bytes(fnv_hash(&ciphertext), 16));

            // Nullifier: зависит от key_id и индекса слоя
            let nullifier = to_hex(&generate_bytes(
                fnv_hash(key.key_id.as_bytes()) ^ (i as u64) ^ 0xabcddcba,
                16,
            ));

            let layer = OnionLayer {
                key_id: key.key_id.clone(),
                next_hop,
                encrypted_payload: ciphertext.clone(),
                integrity_tag,
                nullifier,
            };

            // Следующий inner = Layer(layer)
            inner = OnionPayload::Layer(layer.clone());
            built_layers.push(layer);
        }

        // Последний добавленный (на i=0) — это внешний слой, но он сейчас в built_layers.last()
        built_layers.reverse();
        let outer_layer = built_layers
            .into_iter()
            .next()
            .ok_or_else(|| "Onion build internal error".to_string())?;

        // Commitment отправителя
        let sender_commitment = to_hex(&generate_bytes(
            fnv_hash(self.route[0].as_bytes()) ^ 0xdeadcafe,
            32,
        ));

        let packet = OnionPacket {
            outer_layer,
            layer_count,
            sender_commitment,
            created_at: now_ms(),
        };

        Ok((packet, self.keys))
    }
}

impl Default for OnionBuilder {
    fn default() -> Self { Self::new() }
}

// -----------------------------------------------------------------------------
// Peel / Verify
// -----------------------------------------------------------------------------

fn check_integrity(ciphertext: &[u8], integrity_tag: &str) -> bool {
    let expected = to_hex(&generate_bytes(fnv_hash(ciphertext), 16));
    expected == integrity_tag
}

/// Снять один слой:
/// - проверяем integrity_tag
/// - decrypt -> OnionPayload
/// - если Layer(next_layer) -> вернуть next_hop + next_layer
/// - если Data(bytes) -> финал
pub fn peel_onion_layer(layer: &OnionLayer, key: &SessionKey) -> Result<PeeledResult, String> {
    if layer.key_id != key.key_id {
        return Err("key_id mismatch for onion layer".to_string());
    }

    if !check_integrity(&layer.encrypted_payload, &layer.integrity_tag) {
        return Err("integrity check failed".to_string());
    }

    let decrypted = key.decrypt(&layer.encrypted_payload);

    let inner: OnionPayload = serde_json::from_slice(&decrypted)
        .map_err(|e| format!("Onion deserialize error: {}", e))?;

    match inner {
        OnionPayload::Layer(next_layer) => {
            let is_final = next_layer.next_hop == "DESTINATION";
            Ok(PeeledResult {
                next_hop: next_layer.next_hop.clone(),
                is_final,
                final_data: None,
                next_layer: Some(next_layer),
            })
        }
        OnionPayload::Data(data) => {
            Ok(PeeledResult {
                next_hop: "DESTINATION".to_string(),
                is_final: true,
                final_data: Some(data),
                next_layer: None,
            })
        }
    }
}

// -----------------------------------------------------------------------------
// NullifierSet — защита от replay-атак
// -----------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct NullifierSet {
    seen: HashSet<String>,
    pub replay_attempts: u64,
}

impl NullifierSet {
    pub fn new() -> Self { Self::default() }

    pub fn check_and_add(&mut self, nullifier: &str) -> bool {
        if self.seen.contains(nullifier) {
            self.replay_attempts += 1;
            false
        } else {
            self.seen.insert(nullifier.to_string());
            true
        }
    }

    pub fn size(&self) -> usize { self.seen.len() }
}

// -----------------------------------------------------------------------------
// BlindedSsau — SSAU данные без раскрытия узла-отправителя (MVP)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedSsau {
    pub sender_commitment: String,
    pub latency_ms: f64,
    pub bandwidth_mbps: f64,
    pub encrypted_reliability: Vec<u8>,
    pub validity_proof: String,
}

impl BlindedSsau {
    pub fn from_tensor(
        sender_id: &str,
        latency_ms: f64,
        bandwidth_mbps: f64,
        reliability: f64,
        session_key: &SessionKey,
    ) -> Self {
        let commitment = ZkpCommitment::commit(sender_id.as_bytes());

        let reliability_bytes = reliability.to_le_bytes();
        let encrypted_reliability = session_key.encrypt(&reliability_bytes);

        let validity_proof = to_hex(&generate_bytes(
            fnv_hash(sender_id.as_bytes()) ^ (latency_ms as u64) ^ 0x12345678,
            32,
        ));

        BlindedSsau {
            sender_commitment: commitment.commitment,
            latency_ms,
            bandwidth_mbps,
            encrypted_reliability,
            validity_proof,
        }
    }

    pub fn decrypt_reliability(&self, session_key: &SessionKey) -> f64 {
        let decrypted = session_key.decrypt(&self.encrypted_reliability);
        if decrypted.len() >= 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&decrypted[..8]);
            f64::from_le_bytes(bytes)
        } else {
            0.0
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onion_build_and_full_peel() {
        let route = vec!["node_A".to_string(), "node_B".to_string(), "node_C".to_string()];
        let payload = b"SECRET PAYLOAD";

        let (packet, keys) = OnionBuilder::new()
            .with_route(route.clone())
            .build(payload)
            .expect("build ok");

        // peel A
        let r1 = peel_onion_layer(&packet.outer_layer, &keys[0]).expect("peel A ok");
        assert_eq!(r1.next_hop, "node_B");
        assert!(r1.next_layer.is_some());

        // peel B (берём следующий слой)
        let layer_b = r1.next_layer.unwrap();
        let r2 = peel_onion_layer(&layer_b, &keys[1]).expect("peel B ok");
        assert_eq!(r2.next_hop, "node_C");
        assert!(r2.next_layer.is_some());

        // peel C
        let layer_c = r2.next_layer.unwrap();
        let r3 = peel_onion_layer(&layer_c, &keys[2]).expect("peel C ok");

        // финал
        assert!(r3.is_final);
        assert_eq!(r3.final_data.unwrap(), payload.to_vec());
    }

    #[test]
    fn test_key_registry() {
        let k = SessionKey::generate();
        let id = k.key_id.clone();
        let mut kr = KeyRegistry::new();
        kr.insert(k);
        assert!(kr.get(&id).is_some());
    }

    #[test]
    fn test_nullifier_replay() {
        let mut n = NullifierSet::new();
        assert!(n.check_and_add("abc"));
        assert!(!n.check_and_add("abc"));
        assert_eq!(n.replay_attempts, 1);
    }

    #[test]
    fn test_commitment() {
        let v = b"node_X";
        let c = ZkpCommitment::commit(v);
        assert!(c.verify(v));
        assert!(!c.verify(b"node_Y"));
    }
}
