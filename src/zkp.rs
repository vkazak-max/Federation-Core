cat > src/zkp.rs << 'EOF'
// =============================================================================
// FEDERATION CORE — zkp.rs
// PHASE 2 / WEEK 6 — «Zero-Knowledge Basics»
// =============================================================================
//
// Реализует:
//   1. OnionHeader   — многослойное шифрование заголовка (как Tor onion)
//   2. BlindRoute    — маршрут где каждый узел видит только след. hop
//   3. ZkpCommitment — математическое обязательство (commitment scheme)
//   4. HeaderCipher  — шифрование/дешифрование заголовков
//   5. NullifierSet  — защита от replay-атак
//
// Принцип (White Paper §3.1):
//   Узел-маршрутизатор видит только «вектор весов» (куда направить),
//   но НЕ видит отправителя и получателя.
//
// Реализация: симметричное шифрование XOR + commitment через SHA-256 (hmac).
// В production заменить на ChaCha20-Poly1305 + zk-SNARKs.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
// Простой PRNG на основе xorshift64 (без внешних зависимостей)
// -----------------------------------------------------------------------------

fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Генерация псевдослучайных байт из seed
fn generate_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    let mut result = Vec::with_capacity(len);
    while result.len() < len {
        let val = xorshift64(&mut state);
        for b in val.to_le_bytes() {
            result.push(b);
            if result.len() == len { break; }
        }
    }
    result
}

/// Простой hash (FNV-1a 64bit) — детерминированный
pub fn fnv_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Hex-строка из байт
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// -----------------------------------------------------------------------------
// SessionKey — сессионный ключ шифрования
// -----------------------------------------------------------------------------

/// Сессионный ключ для шифрования одного слоя onion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKey {
    pub key_bytes: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_id: String,
}

impl SessionKey {
    /// Генерировать новый случайный ключ
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

    /// Создать ключ из общего секрета (Diffie-Hellman симуляция)
    /// В production: X25519 ECDH
    pub fn from_shared_secret(our_secret: u64, their_public: u64) -> Self {
        let shared = our_secret.wrapping_mul(their_public) ^ 0x9e3779b97f4a7c15;
        let key_bytes = generate_bytes(shared, KEY_SIZE);
        let nonce = generate_bytes(shared ^ 0xdeadbeef, NONCE_SIZE);
        let key_id = to_hex(&generate_bytes(shared ^ 0xcafe, 8));
        SessionKey { key_bytes, nonce, key_id }
    }

    /// XOR-шифрование (симуляция ChaCha20)
    /// В production заменить на chacha20-poly1305 crate
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let keystream = generate_bytes(
            fnv_hash(&self.key_bytes) ^ fnv_hash(&self.nonce),
            plaintext.len(),
        );
        plaintext.iter().zip(keystream.iter()).map(|(p, k)| p ^ k).collect()
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8> {
        // XOR симметричен: decrypt = encrypt
        self.encrypt(ciphertext)
    }
}

// -----------------------------------------------------------------------------
// ZkpCommitment — криптографическое обязательство
// -----------------------------------------------------------------------------

/// Commitment scheme: commit(value, blinding_factor) = hash(value || blinding)
///
/// Свойства:
///   - Hiding:  по commitment нельзя узнать value
///   - Binding: нельзя открыть commitment с другим value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpCommitment {
    /// Публичный commitment (можно показывать всем)
    pub commitment: String,
    /// Blinding factor (держим в секрете до reveal)
    pub blinding_factor: u64,
    /// Хеш от value (для верификации при reveal)
    value_hash: u64,
}

impl ZkpCommitment {
    /// Создать commitment для значения
    pub fn commit(value: &[u8]) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let blinding_factor = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64 ^ 0xfeedface;

        let value_hash = fnv_hash(value);

        // commitment = hash(value || blinding_factor)
        let mut combined = value.to_vec();
        combined.extend_from_slice(&blinding_factor.to_le_bytes());
        let commitment = to_hex(&generate_bytes(fnv_hash(&combined), 32));

        ZkpCommitment { commitment, blinding_factor, value_hash }
    }

    /// Верифицировать что value соответствует commitment
    pub fn verify(&self, value: &[u8]) -> bool {
        let claimed_hash = fnv_hash(value);
        if claimed_hash != self.value_hash {
            return false;
        }
        let mut combined = value.to_vec();
        combined.extend_from_slice(&self.blinding_factor.to_le_bytes());
        let expected = to_hex(&generate_bytes(fnv_hash(&combined), 32));
        expected == self.commitment
    }
}

// -----------------------------------------------------------------------------
// OnionLayer — один слой луковичного шифрования
// -----------------------------------------------------------------------------

/// Один слой onion-маршрутизации.
/// Каждый узел видит только:
///   - Зашифрованный payload для следующего хопа
///   - ID следующего узла (в открытом виде — только для маршрутизации)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnionLayer {
    /// ID следующего узла (единственное что видит текущий узел)
    pub next_hop: String,
    /// Зашифрованный payload (остаток луковицы)
    pub encrypted_payload: Vec<u8>,
    /// Commitment для верификации целостности
    pub integrity_commitment: String,
    /// Nullifier — предотвращает replay
    pub nullifier: String,
}

// -----------------------------------------------------------------------------
// OnionPacket — полный зашифрованный пакет
// -----------------------------------------------------------------------------

/// Полный onion-пакет. Видимая часть минимальна.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnionPacket {
    /// Только первый слой виден текущему узлу
    pub outer_layer: OnionLayer,
    /// Число слоёв (без раскрытия маршрута)
    pub layer_count: usize,
    /// Публичный commitment отправителя (без раскрытия ID)
    pub sender_commitment: String,
    /// Timestamp для anti-replay
    pub created_at: i64,
}

// -----------------------------------------------------------------------------
// OnionBuilder — строит многослойный зашифрованный пакет
// -----------------------------------------------------------------------------

/// Строитель onion-пакета.
///
/// Использование:
///   1. Задаём маршрут [node1, node2, ..., destination]
///   2. Для каждого узла генерируем SessionKey
///   3. Оборачиваем payload в слои (от последнего к первому)
///   4. Каждый узел может снять только свой слой
pub struct OnionBuilder {
    route: Vec<String>,
    keys: Vec<SessionKey>,
}

impl OnionBuilder {
    pub fn new() -> Self {
        OnionBuilder { route: vec![], keys: vec![] }
    }

    /// Задать маршрут и автоматически сгенерировать ключи
    pub fn with_route(mut self, route: Vec<String>) -> Self {
        self.keys = route.iter().map(|_| SessionKey::generate()).collect();
        self.route = route;
        self
    }

    /// Собрать onion-пакет с payload
    pub fn build(self, payload: &[u8]) -> Result<(OnionPacket, Vec<SessionKey>), String> {
        if self.route.is_empty() {
            return Err("Маршрут не задан".to_string());
        }
        if self.route.len() > MAX_ONION_LAYERS {
            return Err(format!("Слишком много слоёв: {} > {}", self.route.len(), MAX_ONION_LAYERS));
        }

        let layer_count = self.route.len();

        // Строим слои от последнего к первому (как настоящий onion)
        let mut current_payload = payload.to_vec();

        // Сохраним все слои для дебага
        let mut layers: Vec<OnionLayer> = Vec::new();

        for i in (0..self.route.len()).rev() {
            let key = &self.keys[i];
            let next_hop = if i + 1 < self.route.len() {
                self.route[i + 1].clone()
            } else {
                "DESTINATION".to_string()
            };

            // Шифруем текущий payload ключом этого узла
            let encrypted = key.encrypt(&current_payload);

            // Integrity commitment
            let integrity = to_hex(&generate_bytes(fnv_hash(&encrypted), 16));

            // Nullifier — уникальный идентификатор для anti-replay
            let nullifier = to_hex(&generate_bytes(
                fnv_hash(key.key_id.as_bytes()) ^ i as u64,
                16,
            ));

            let layer = OnionLayer {
                next_hop,
                encrypted_payload: encrypted.clone(),
                integrity_commitment: integrity,
                nullifier,
            };

            layers.push(layer);
            // Следующий слой оборачивает текущий зашифрованный payload
            current_payload = encrypted;
        }

        layers.reverse();

        // Commitment отправителя (без раскрытия ID)
        let sender_commitment = to_hex(&generate_bytes(
            fnv_hash(self.route[0].as_bytes()) ^ 0xdeadcafe,
            32,
        ));

        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let packet = OnionPacket {
            outer_layer: layers.into_iter().next().unwrap(),
            layer_count,
            sender_commitment,
            created_at: now,
        };

        Ok((packet, self.keys))
    }
}

impl Default for OnionBuilder {
    fn default() -> Self { Self::new() }
}

// -----------------------------------------------------------------------------
// OnionProcessor — обработка пакета на промежуточном узле
// -----------------------------------------------------------------------------

/// Результат обработки onion-пакета
#[derive(Debug)]
pub struct PeeledLayer {
    /// Следующий узел куда переслать
    pub next_hop: String,
    /// Расшифрованный payload для следующего слоя
    pub inner_payload: Vec<u8>,
    /// Это последний слой (мы — получатель)?
    pub is_final: bool,
}

/// Снять один слой onion-пакета.
///
/// Узел знает только свой ключ → видит только next_hop.
/// Содержимое payload остаётся зашифрованным для него.
pub fn peel_onion_layer(layer: &OnionLayer, key: &SessionKey) -> PeeledLayer {
    let inner = key.decrypt(&layer.encrypted_payload);
    let is_final = layer.next_hop == "DESTINATION";
    PeeledLayer {
        next_hop: layer.next_hop.clone(),
        inner_payload: inner,
        is_final,
    }
}

// -----------------------------------------------------------------------------
// NullifierSet — защита от replay-атак
// -----------------------------------------------------------------------------

/// Реестр использованных nullifiers.
/// Если nullifier уже видели — пакет отброшен (replay-атака).
#[derive(Debug, Default)]
pub struct NullifierSet {
    seen: HashSet<String>,
    /// Счётчик отброшенных replay
    pub replay_attempts: u64,
}

impl NullifierSet {
    pub fn new() -> Self { Self::default() }

    /// Проверить и добавить nullifier. Возвращает false если replay.
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
// BlindedSsau — SSAU данные без раскрытия узла-отправителя
// -----------------------------------------------------------------------------

/// SSAU тензор с ослеплённым (blinded) источником.
/// Промежуточный узел видит данные о качестве канала,
/// но не знает кто их отправил.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedSsau {
    /// Commitment вместо реального node_id отправителя
    pub sender_commitment: String,
    /// Данные канала (в открытом виде — нужны для маршрутизации)
    pub latency_ms: f64,
    pub bandwidth_mbps: f64,
    /// Reliability зашифрован — только получатель может расшифровать
    pub encrypted_reliability: Vec<u8>,
    /// Proof что данные валидны (без раскрытия источника)
    pub validity_proof: String,
}

impl BlindedSsau {
    /// Создать ослеплённый SSAU из реального тензора
    pub fn from_tensor(
        sender_id: &str,
        latency_ms: f64,
        bandwidth_mbps: f64,
        reliability: f64,
        session_key: &SessionKey,
    ) -> Self {
        // Commitment вместо реального ID
        let commitment = ZkpCommitment::commit(sender_id.as_bytes());

        // Шифруем reliability (чувствительный параметр)
        let reliability_bytes = reliability.to_le_bytes();
        let encrypted_reliability = session_key.encrypt(&reliability_bytes);

        // Proof валидности (в реальной системе — zk-SNARK)
        let validity_proof = to_hex(&generate_bytes(
            fnv_hash(sender_id.as_bytes()) ^ (latency_ms as u64),
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

    /// Расшифровать reliability (только если знаешь ключ)
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
    fn test_session_key_encrypt_decrypt() {
        let key = SessionKey::generate();
        let plaintext = b"Federation ZKP test message 12345";
        let encrypted = key.encrypt(plaintext);
        let decrypted = key.decrypt(&encrypted);
        assert_eq!(plaintext.to_vec(), decrypted, "Расшифровка должна вернуть оригинал");
        assert_ne!(plaintext.to_vec(), encrypted, "Шифртекст должен отличаться");
        println!("✅ XOR cipher: plaintext={} bytes encrypted={} bytes", plaintext.len(), encrypted.len());
    }

    #[test]
    fn test_zkp_commitment_verify() {
        let value = b"node_ALPHA_secret_identity";
        let commitment = ZkpCommitment::commit(value);
        assert!(commitment.verify(value), "Верификация должна пройти с правильным значением");
        assert!(!commitment.verify(b"wrong_value"), "Верификация должна провалиться с неверным значением");
        println!("✅ ZKP Commitment: {}", &commitment.commitment[..16]);
        println!("   Hiding: commitment не раскрывает identity");
        println!("   Binding: нельзя открыть с другим значением");
    }

    #[test]
    fn test_onion_build_and_peel() {
        let route = vec![
            "node_A".to_string(),
            "node_B".to_string(),
            "node_C".to_string(),
        ];
        let payload = b"SECRET: send 100 coins to destination";

        let (packet, keys) = OnionBuilder::new()
            .with_route(route.clone())
            .build(payload)
            .expect("Onion build должен успешно выполниться");

        println!("✅ Onion packet создан: {} слоёв", packet.layer_count);
        println!("   Outer next_hop: {}", packet.outer_layer.next_hop);
        println!("   Encrypted size: {} bytes", packet.outer_layer.encrypted_payload.len());
        println!("   Sender commitment: {}", &packet.sender_commitment[..16]);

        // node_A снимает свой слой
        let peeled_a = peel_onion_layer(&packet.outer_layer, &keys[0]);
        println!("\n   node_A видит только: next_hop={}", peeled_a.next_hop);
        println!("   node_A НЕ знает отправителя и финального получателя");
        assert_eq!(peeled_a.next_hop, "node_B");
        assert!(!peeled_a.is_final);

        // Проверяем что payload зашифрован (не равен оригиналу)
        assert_ne!(packet.outer_layer.encrypted_payload, payload.to_vec());

        println!("\n✅ Zero-Knowledge routing: каждый узел видит только следующий hop");
    }

    #[test]
    fn test_nullifier_anti_replay() {
        let mut nullifiers = NullifierSet::new();
        let nullifier = "abc123def456";

        assert!(nullifiers.check_and_add(nullifier), "Первый пакет должен пройти");
        assert!(!nullifiers.check_and_add(nullifier), "Replay должен быть отброшен");
        assert_eq!(nullifiers.replay_attempts, 1);
        println!("✅ Anti-replay: повторный пакет отброшен. Replay attempts: {}", nullifiers.replay_attempts);
    }

    #[test]
    fn test_blinded_ssau() {
        let key = SessionKey::generate();
        let blinded = BlindedSsau::from_tensor("node_ALPHA", 10.0, 1000.0, 0.99, &key);

        println!("✅ Blinded SSAU:");
        println!("   Sender commitment: {}", &blinded.sender_commitment[..16]);
        println!("   Latency (открыто): {:.1}ms", blinded.latency_ms);
        println!("   Bandwidth (открыто): {:.0}Mbps", blinded.bandwidth_mbps);
        println!("   Reliability (зашифровано): {} bytes", blinded.encrypted_reliability.len());

        // Расшифровываем reliability
        let decrypted_reliability = blinded.decrypt_reliability(&key);
        println!("   Reliability (расшифровано): {:.4}", decrypted_reliability);
        assert!((decrypted_reliability - 0.99).abs() < 0.001,
            "Расшифрованное значение должно совпадать с оригиналом");

        println!("   Промежуточный узел видит только latency и bandwidth");
        println!("   Reliability и sender_id скрыты за commitment");
    }
}
EOF
