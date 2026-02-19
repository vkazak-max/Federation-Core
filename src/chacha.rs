// =============================================================================
// FEDERATION CORE — chacha.rs
// PHASE 9 — Crypto Core
// ChaCha20-Poly1305 AEAD + X25519 Key Exchange
//
// Без внешних крипто-крейтов — pure Rust реализация.
// Используется для шифрования всех Pulse и меш-пакетов.
//
// Компоненты:
//   ChaCha20      — потоковый шифр (RFC 8439)
//   Poly1305      — MAC аутентификатор (RFC 8439)
//   ChaCha20Poly1305 — AEAD (Authenticated Encryption with Associated Data)
//   X25519        — ECDH обмен ключами (упрощённая версия)
//   FederationCipher — высокоуровневый API для Федерации
// =============================================================================

use std::collections::HashMap;

pub const KEY_SIZE: usize    = 32; // 256 бит
pub const NONCE_SIZE: usize  = 12; // 96 бит
pub const TAG_SIZE: usize    = 16; // 128 бит MAC
pub const BLOCK_SIZE: usize  = 64; // ChaCha20 блок

// -----------------------------------------------------------------------------
// Утилиты
// -----------------------------------------------------------------------------

fn u32_from_le(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i+1], b[i+2], b[i+3]])
}

fn quarter_round(a: &mut u32, b: &mut u32, c: &mut u32, d: &mut u32) {
    *a = a.wrapping_add(*b); *d ^= *a; *d = d.rotate_left(16);
    *c = c.wrapping_add(*d); *b ^= *c; *b = b.rotate_left(12);
    *a = a.wrapping_add(*b); *d ^= *a; *d = d.rotate_left(8);
    *c = c.wrapping_add(*d); *b ^= *c; *b = b.rotate_left(7);
}

// -----------------------------------------------------------------------------
// ChaCha20 — потоковый шифр RFC 8439
// -----------------------------------------------------------------------------

pub struct ChaCha20 {
    state: [u32; 16],
}

impl ChaCha20 {
    pub fn new(key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE], counter: u32) -> Self {
        // Константа "expand 32-byte k"
        let mut state = [0u32; 16];
        state[0]  = 0x61707865;
        state[1]  = 0x3320646e;
        state[2]  = 0x79622d32;
        state[3]  = 0x6b206574;
        // Ключ (256 бит = 8 u32)
        for i in 0..8 {
            state[4+i] = u32_from_le(key, i*4);
        }
        // Счётчик
        state[12] = counter;
        // Nonce (96 бит = 3 u32)
        state[13] = u32_from_le(nonce, 0);
        state[14] = u32_from_le(nonce, 4);
        state[15] = u32_from_le(nonce, 8);
        ChaCha20 { state }
    }

    fn block(&self) -> [u8; BLOCK_SIZE] {
        let mut x = self.state;
        // 20 раундов (10 double rounds)
        for _ in 0..10 {
            // Колонки — извлекаем во временные переменные
            let (mut a,mut b,mut c,mut d) = (x[0],x[4],x[8],x[12]);
            quarter_round(&mut a,&mut b,&mut c,&mut d);
            x[0]=a;x[4]=b;x[8]=c;x[12]=d;

            let (mut a,mut b,mut c,mut d) = (x[1],x[5],x[9],x[13]);
            quarter_round(&mut a,&mut b,&mut c,&mut d);
            x[1]=a;x[5]=b;x[9]=c;x[13]=d;

            let (mut a,mut b,mut c,mut d) = (x[2],x[6],x[10],x[14]);
            quarter_round(&mut a,&mut b,&mut c,&mut d);
            x[2]=a;x[6]=b;x[10]=c;x[14]=d;

            let (mut a,mut b,mut c,mut d) = (x[3],x[7],x[11],x[15]);
            quarter_round(&mut a,&mut b,&mut c,&mut d);
            x[3]=a;x[7]=b;x[11]=c;x[15]=d;

            // Диагонали
            let (mut a,mut b,mut c,mut d) = (x[0],x[5],x[10],x[15]);
            quarter_round(&mut a,&mut b,&mut c,&mut d);
            x[0]=a;x[5]=b;x[10]=c;x[15]=d;

            let (mut a,mut b,mut c,mut d) = (x[1],x[6],x[11],x[12]);
            quarter_round(&mut a,&mut b,&mut c,&mut d);
            x[1]=a;x[6]=b;x[11]=c;x[12]=d;

            let (mut a,mut b,mut c,mut d) = (x[2],x[7],x[8],x[13]);
            quarter_round(&mut a,&mut b,&mut c,&mut d);
            x[2]=a;x[7]=b;x[8]=c;x[13]=d;

            let (mut a,mut b,mut c,mut d) = (x[3],x[4],x[9],x[14]);
            quarter_round(&mut a,&mut b,&mut c,&mut d);
            x[3]=a;x[4]=b;x[9]=c;x[14]=d;
        }
        // Добавляем исходное состояние
        let mut out = [0u8; BLOCK_SIZE];
        for i in 0..16 {
            let word = x[i].wrapping_add(self.state[i]);
            let bytes = word.to_le_bytes();
            out[i*4..i*4+4].copy_from_slice(&bytes);
        }
        out
    }

    pub fn keystream(&mut self, len: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(len);
        let mut remaining = len;
        while remaining > 0 {
            let block = self.block();
            let take = remaining.min(BLOCK_SIZE);
            result.extend_from_slice(&block[..take]);
            remaining -= take;
            self.state[12] = self.state[12].wrapping_add(1);
        }
        result
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let ks = self.keystream(plaintext.len());
        plaintext.iter().zip(ks.iter()).map(|(p,k)| p^k).collect()
    }

    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Vec<u8> {
        self.encrypt(ciphertext) // XOR симметричен
    }
}

// -----------------------------------------------------------------------------
// Poly1305 — MAC аутентификатор RFC 8439
// -----------------------------------------------------------------------------

pub struct Poly1305 {
    r: [u64; 5],
    s: [u64; 4],
    h: [u64; 5],
}

impl Poly1305 {
    pub fn new(key: &[u8; 32]) -> Self {
        // r: первые 16 байт ключа (clamp)
        let mut r = [0u64; 5];
        let r128 = u128::from_le_bytes(key[..16].try_into().unwrap());
        // clamp: обнуляем определённые биты
        let r_clamped = r128 & 0x0ffffffc0ffffffc0ffffffc0fffffff;
        r[0] = (r_clamped & 0x3ffffff) as u64;
        r[1] = ((r_clamped >> 26) & 0x3ffffff) as u64;
        r[2] = ((r_clamped >> 52) & 0x3ffffff) as u64;
        r[3] = ((r_clamped >> 78) & 0x3ffffff) as u64;
        r[4] = ((r_clamped >> 104) & 0x3ffffff) as u64;

        // s: последние 16 байт ключа
        let mut s = [0u64; 4];
        for i in 0..4 {
            s[i] = u32::from_le_bytes(key[16+i*4..20+i*4].try_into().unwrap()) as u64;
        }

        Poly1305 { r, s, h: [0u64; 5] }
    }

    fn process_block(&mut self, block: &[u8], last: bool) {
        let shift = block.len() * 8;
        let pad = if last || shift >= 128 { 0u128 } else { 1u128 << shift };
        let mut n = [0u8; 17];
        n[..block.len()].copy_from_slice(block);
        if !last { n[block.len()] = 1; }
        let n = u128::from_le_bytes(n[..16].try_into().unwrap()) | pad;

        // Добавляем к аккумулятору
        let n0 = (n & 0x3ffffff) as u64;
        let n1 = ((n >> 26) & 0x3ffffff) as u64;
        let n2 = ((n >> 52) & 0x3ffffff) as u64;
        let n3 = ((n >> 78) & 0x3ffffff) as u64;
        let n4 = ((n >> 104) & 0x3ffffff) as u64;

        let mut h = [0u64; 5];
        h[0] = self.h[0].wrapping_add(n0);
        h[1] = self.h[1].wrapping_add(n1);
        h[2] = self.h[2].wrapping_add(n2);
        h[3] = self.h[3].wrapping_add(n3);
        h[4] = self.h[4].wrapping_add(n4);

        // Умножение h * r (упрощённое для pure Rust)
        let r = &self.r;
        let mut t = [0u128; 5];
        t[0] = h[0] as u128 * r[0] as u128
             + h[1] as u128 * (r[4]*5) as u128
             + h[2] as u128 * (r[3]*5) as u128
             + h[3] as u128 * (r[2]*5) as u128
             + h[4] as u128 * (r[1]*5) as u128;
        t[1] = h[0] as u128 * r[1] as u128
             + h[1] as u128 * r[0] as u128
             + h[2] as u128 * (r[4]*5) as u128
             + h[3] as u128 * (r[3]*5) as u128
             + h[4] as u128 * (r[2]*5) as u128;
        t[2] = h[0] as u128 * r[2] as u128
             + h[1] as u128 * r[1] as u128
             + h[2] as u128 * r[0] as u128
             + h[3] as u128 * (r[4]*5) as u128
             + h[4] as u128 * (r[3]*5) as u128;
        t[3] = h[0] as u128 * r[3] as u128
             + h[1] as u128 * r[2] as u128
             + h[2] as u128 * r[1] as u128
             + h[3] as u128 * r[0] as u128
             + h[4] as u128 * (r[4]*5) as u128;
        t[4] = h[0] as u128 * r[4] as u128
             + h[1] as u128 * r[3] as u128
             + h[2] as u128 * r[2] as u128
             + h[3] as u128 * r[1] as u128
             + h[4] as u128 * r[0] as u128;

        // Редукция по 2^130-5
        let mut h2 = [0u64; 5];
        let mut c = 0u128;
        for i in 0..5 {
            let v = t[i] + c;
            h2[i] = (v & 0x3ffffff) as u64;
            c = v >> 26;
        }
        h2[0] = h2[0].wrapping_add((c * 5) as u64);
        c = (h2[0] >> 26) as u128;
        h2[0] &= 0x3ffffff;
        h2[1] = h2[1].wrapping_add(c as u64);

        self.h = h2;
    }

    pub fn mac(&mut self, msg: &[u8]) -> [u8; TAG_SIZE] {
        // Обрабатываем блоки по 16 байт
        let mut i = 0;
        while i + 16 <= msg.len() {
            let block = &msg[i..i+16];
            self.process_block(block, false);
            i += 16;
        }
        if i < msg.len() {
            self.process_block(&msg[i..], true);
        }

        // Финализация: h + s
        let mut h = self.h;
        // Полная редукция
        let mut c = h[1] >> 26; h[1] &= 0x3ffffff; h[2] = h[2].wrapping_add(c);
        c = h[2] >> 26; h[2] &= 0x3ffffff; h[3] = h[3].wrapping_add(c);
        c = h[3] >> 26; h[3] &= 0x3ffffff; h[4] = h[4].wrapping_add(c);
        c = h[4] >> 26; h[4] &= 0x3ffffff; h[0] = h[0].wrapping_add(c * 5);
        c = h[0] >> 26; h[0] &= 0x3ffffff; h[1] = h[1].wrapping_add(c);

        // Сборка в 128 бит
        let tag_val = (h[0] as u128)
            | ((h[1] as u128) << 26)
            | ((h[2] as u128) << 52)
            | ((h[3] as u128) << 78)
            | ((h[4] as u128) << 104);

        let s = (self.s[0] as u128)
            | ((self.s[1] as u128) << 32)
            | ((self.s[2] as u128) << 64)
            | ((self.s[3] as u128) << 96);

        let tag = tag_val.wrapping_add(s);
        tag.to_le_bytes()
    }
}

// -----------------------------------------------------------------------------
// ChaCha20Poly1305 — AEAD RFC 8439
// -----------------------------------------------------------------------------

pub struct ChaCha20Poly1305 {
    key: [u8; KEY_SIZE],
}

#[derive(Debug, Clone)]
pub struct AeadCiphertext {
    pub nonce: [u8; NONCE_SIZE],
    pub ciphertext: Vec<u8>,
    pub tag: [u8; TAG_SIZE],
    pub aad_len: usize,
}

impl AeadCiphertext {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(NONCE_SIZE + self.ciphertext.len() + TAG_SIZE);
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.ciphertext);
        out.extend_from_slice(&self.tag);
        out
    }
    pub fn len(&self) -> usize {
        NONCE_SIZE + self.ciphertext.len() + TAG_SIZE
    }
}

impl ChaCha20Poly1305 {
    pub fn new(key: [u8; KEY_SIZE]) -> Self {
        ChaCha20Poly1305 { key }
    }

    // Генерация one-time ключа Poly1305
    fn poly1305_key(&self, nonce: &[u8; NONCE_SIZE]) -> [u8; 32] {
        let mut cipher = ChaCha20::new(&self.key, nonce, 0);
        let ks = cipher.keystream(64);
        ks[..32].try_into().unwrap()
    }

    // Паддинг до кратного 16
    fn pad16(len: usize) -> Vec<u8> {
        let rem = len % 16;
        if rem == 0 { vec![] } else { vec![0u8; 16 - rem] }
    }

    pub fn seal(&self, plaintext: &[u8], aad: &[u8],
                nonce: &[u8; NONCE_SIZE]) -> AeadCiphertext {
        // Шифруем с counter=1
        let mut cipher = ChaCha20::new(&self.key, nonce, 1);
        let ciphertext = cipher.encrypt(plaintext);

        // MAC: poly1305(aad || pad || ct || pad || len_aad || len_ct)
        let poly_key = self.poly1305_key(nonce);
        let mut mac_input = Vec::new();
        mac_input.extend_from_slice(aad);
        mac_input.extend_from_slice(&Self::pad16(aad.len()));
        mac_input.extend_from_slice(&ciphertext);
        mac_input.extend_from_slice(&Self::pad16(ciphertext.len()));
        mac_input.extend_from_slice(&(aad.len() as u64).to_le_bytes());
        mac_input.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());

        let tag = Poly1305::new(&poly_key).mac(&mac_input);

        AeadCiphertext { nonce: *nonce, ciphertext, tag, aad_len: aad.len() }
    }

    pub fn open(&self, ct: &AeadCiphertext, aad: &[u8]) -> Result<Vec<u8>, &'static str> {
        // Верифицируем MAC
        let poly_key = self.poly1305_key(&ct.nonce);
        let mut mac_input = Vec::new();
        mac_input.extend_from_slice(aad);
        mac_input.extend_from_slice(&Self::pad16(aad.len()));
        mac_input.extend_from_slice(&ct.ciphertext);
        mac_input.extend_from_slice(&Self::pad16(ct.ciphertext.len()));
        mac_input.extend_from_slice(&(aad.len() as u64).to_le_bytes());
        mac_input.extend_from_slice(&(ct.ciphertext.len() as u64).to_le_bytes());

        let expected_tag = Poly1305::new(&poly_key).mac(&mac_input);

        // Constant-time сравнение
        let tag_ok = ct.tag.iter().zip(expected_tag.iter())
            .fold(0u8, |acc, (a,b)| acc | (a^b)) == 0;
        if !tag_ok { return Err("authentication failed"); }

        // Расшифровываем
        let mut cipher = ChaCha20::new(&self.key, &ct.nonce, 1);
        Ok(cipher.decrypt(&ct.ciphertext))
    }
}

// -----------------------------------------------------------------------------
// X25519 — ECDH обмен ключами (упрощённая Curve25519)
// -----------------------------------------------------------------------------

pub struct X25519 {
    rng: u64,
}

impl X25519 {
    pub fn new(seed: u64) -> Self { X25519 { rng: seed } }

    fn rand_bytes(&mut self, n: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            self.rng ^= self.rng << 17;
            out.extend_from_slice(&self.rng.to_le_bytes());
        }
        out.truncate(n);
        out
    }

    pub fn generate_keypair(&mut self) -> (Vec<u8>, Vec<u8>) {
        let mut privkey = self.rand_bytes(KEY_SIZE);
        // Clamp приватного ключа (Curve25519 требование)
        privkey[0]  &= 248;
        privkey[31] &= 127;
        privkey[31] |= 64;
        // Публичный ключ = scalar_mult(privkey, basepoint)
        // Упрощённая версия: используем BLAKE2b-подобный микс
        let pubkey = self.derive_public(&privkey);
        (privkey, pubkey)
    }

    fn derive_public(&self, privkey: &[u8]) -> Vec<u8> {
        // Детерминированное получение публичного ключа из приватного
        // (в реальном X25519 — умножение на базовую точку Curve25519)
        let mut state = 0x43757276_65323535u64; // "Curve255"
        let mut pub_key = Vec::with_capacity(KEY_SIZE);
        for (i, &b) in privkey.iter().enumerate() {
            state ^= (b as u64).wrapping_mul(0x517cc1b7_27220a95);
            state = state.rotate_left((i % 64) as u32 + 1);
            state ^= state >> 33;
            state = state.wrapping_mul(0xff51afd7ed558ccd);
            state ^= state >> 33;
            pub_key.push((state & 0xff) as u8);
        }
        pub_key
    }

    pub fn diffie_hellman(privkey: &[u8], pubkey: &[u8]) -> Vec<u8> {
        // ECDH: shared_secret = scalar_mult(privkey, pubkey)
        // Упрощённая версия: XOR + mix для демонстрации протокола
        let mut shared = Vec::with_capacity(KEY_SIZE);
        let mut state = 0xD415_A4E5_5ECE_7000u64;
        for i in 0..KEY_SIZE {
            let a = privkey[i % privkey.len()] as u64;
            let b = pubkey[i % pubkey.len()] as u64;
            state = state.wrapping_add(a.wrapping_mul(b));
            state ^= state >> 17;
            state = state.wrapping_mul(0xbf58476d1ce4e5b9);
            state ^= state >> 31;
            shared.push((state & 0xff) as u8);
        }
        shared
    }
}

// -----------------------------------------------------------------------------
// FederationCipher — высокоуровневый API
// -----------------------------------------------------------------------------

pub struct FederationCipher {
    sessions: HashMap<String, ChaCha20Poly1305>,
    rng: u64,
    pub encrypt_count: u64,
    pub decrypt_count: u64,
    pub bytes_encrypted: u64,
    pub auth_failures: u64,
}

impl FederationCipher {
    pub fn new() -> Self {
        FederationCipher {
            sessions: HashMap::new(),
            rng: 0xFEDC_1A50_C0DE_0000,
            encrypt_count: 0, decrypt_count: 0,
            bytes_encrypted: 0, auth_failures: 0,
        }
    }

    fn next_rng(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    pub fn random_nonce(&mut self) -> [u8; NONCE_SIZE] {
        let mut n = [0u8; NONCE_SIZE];
        for i in 0..3 {
            let r = self.next_rng().to_le_bytes();
            n[i*4..i*4+4].copy_from_slice(&r[..4]);
        }
        n
    }

    pub fn establish_session(&mut self, peer_id: &str, shared_secret: &[u8]) {
        let mut key = [0u8; KEY_SIZE];
        for i in 0..KEY_SIZE {
            key[i] = shared_secret[i % shared_secret.len()];
        }
        // HKDF-подобный mix
        for i in 0..KEY_SIZE {
            let r = self.next_rng();
            key[i] ^= (r >> (i*3 % 57)) as u8;
        }
        // Восстанавливаем детерминированно из shared_secret
        let mut key2 = [0u8; KEY_SIZE];
        let mut state = 0x4845_4453_4554u64; // "HEDSET"
        for i in 0..KEY_SIZE {
            state ^= shared_secret[i % shared_secret.len()] as u64;
            state = state.wrapping_mul(6364136223846793005);
            state = state.wrapping_add(1442695040888963407);
            key2[i] = (state >> 33) as u8;
        }
        self.sessions.insert(peer_id.to_string(), ChaCha20Poly1305::new(key2));
    }

    pub fn encrypt_pulse(&mut self, peer_id: &str, pulse: &[u8],
                          aad: &[u8]) -> Option<AeadCiphertext> {
        let nonce = self.random_nonce();
        let cipher = self.sessions.get(peer_id)?;
        let ct = cipher.seal(pulse, aad, &nonce);
        self.encrypt_count += 1;
        self.bytes_encrypted += pulse.len() as u64;
        Some(ct)
    }

    pub fn decrypt_pulse(&mut self, peer_id: &str, ct: &AeadCiphertext,
                          aad: &[u8]) -> Result<Vec<u8>, &'static str> {
        let cipher = self.sessions.get(peer_id)
            .ok_or("no session")?;
        match cipher.open(ct, aad) {
            Ok(pt) => { self.decrypt_count += 1; Ok(pt) }
            Err(e) => { self.auth_failures += 1; Err(e) }
        }
    }

    pub fn stats(&self) -> CipherStats {
        CipherStats {
            sessions: self.sessions.len(),
            encrypt_count: self.encrypt_count,
            decrypt_count: self.decrypt_count,
            bytes_encrypted: self.bytes_encrypted,
            auth_failures: self.auth_failures,
        }
    }
}

impl Default for FederationCipher { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct CipherStats {
    pub sessions: usize,
    pub encrypt_count: u64,
    pub decrypt_count: u64,
    pub bytes_encrypted: u64,
    pub auth_failures: u64,
}
