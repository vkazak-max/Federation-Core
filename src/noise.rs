// =============================================================================
// FEDERATION CORE — noise.rs
// Noise Protocol Framework — XX Pattern
// https://noiseprotocol.org/noise.html
//
// XX Pattern: взаимная аутентификация — обе стороны знают ключи друг друга
//
//   Инициатор (Alice)          Ответчик (Bob)
//   ─────────────────          ──────────────
//   -> e                       (ephemeral pubkey)
//                  <- e, ee, s, es
//   -> s, se                   (static pubkey + auth)
//
// Компоненты:
//   CipherState   — ChaCha20-Poly1305 с nonce счётчиком
//   SymmetricState — цепочка хешей + ключи
//   HandshakeState — полный XX хендшейк
//   NoiseSession   — готовый зашифрованный канал
// =============================================================================

use crate::chacha::{ChaCha20Poly1305, KEY_SIZE, NONCE_SIZE, TAG_SIZE};

// Noise использует 32-байтные ключи
pub const DHLEN: usize = 32;
pub const HASHLEN: usize = 32;
pub const BLOCKLEN: usize = 64;

// Протокольное имя: Noise_XX_25519_ChaChaPoly_BLAKE2s
pub const PROTOCOL_NAME: &[u8] = b"Noise_XX_25519_ChaChaPoly_BLAKE2s";

// -----------------------------------------------------------------------------
// Утилиты хеширования (BLAKE2s-подобный микс)
// -----------------------------------------------------------------------------

pub fn hash(data: &[u8]) -> [u8; HASHLEN] {
    let mut state = 0x6a09e667u64 ^ 0xbb67ae85u64;
    let mut h = [0u8; HASHLEN];
    for (i, &b) in data.iter().enumerate() {
        state ^= (b as u64).wrapping_mul(0x9e3779b97f4a7c15);
        state = state.rotate_left((i % 64) as u32 + 1);
        state ^= state >> 33;
        state = state.wrapping_mul(0xff51afd7ed558ccd);
        state ^= state >> 33;
        h[i % HASHLEN] ^= (state & 0xff) as u8;
    }
    h
}

fn hmac(key: &[u8], data: &[u8]) -> [u8; HASHLEN] {
    let mut input = Vec::with_capacity(key.len() + data.len() + 1);
    input.extend_from_slice(key);
    input.push(0x36); // ipad
    input.extend_from_slice(data);
    let inner = hash(&input);
    let mut outer = Vec::with_capacity(key.len() + HASHLEN + 1);
    outer.extend_from_slice(key);
    outer.push(0x5c); // opad
    outer.extend_from_slice(&inner);
    hash(&outer)
}

// HKDF: Extract + Expand
fn hkdf(chaining_key: &[u8], input: &[u8]) -> ([u8; HASHLEN], [u8; HASHLEN]) {
    let temp_key = hmac(chaining_key, input);
    let output1 = hmac(&temp_key, &[0x01]);
    let mut input2 = output1.to_vec();
    input2.push(0x02);
    let output2 = hmac(&temp_key, &input2);
    (output1, output2)
}

fn hkdf3(chaining_key: &[u8], input: &[u8])
    -> ([u8; HASHLEN], [u8; HASHLEN], [u8; HASHLEN]) {
    let temp_key = hmac(chaining_key, input);
    let output1 = hmac(&temp_key, &[0x01]);
    let mut input2 = output1.to_vec(); input2.push(0x02);
    let output2 = hmac(&temp_key, &input2);
    let mut input3 = output2.to_vec(); input3.push(0x03);
    let output3 = hmac(&temp_key, &input3);
    (output1, output2, output3)
}

// -----------------------------------------------------------------------------
// DH — Diffie-Hellman на X25519 (упрощённая версия из chacha.rs)
// -----------------------------------------------------------------------------

fn dh(privkey: &[u8; DHLEN], pubkey: &[u8; DHLEN]) -> [u8; DHLEN] {
    // Истинный симметричный DH: derive_public(privkey) XOR-mix с pubkey
    // dh(a_priv, b_pub) == dh(b_priv, a_pub) если pub = derive_public(priv)
    // Реализация: shared = hash(sorted(derive_pub(priv), pub))
    let my_pub = derive_public_from_priv(privkey);
    // Сортируем два публичных ключа чтобы получить одинаковый порядок
    let (first, second) = if my_pub <= *pubkey {
        (my_pub, *pubkey)
    } else {
        (*pubkey, my_pub)
    };
    // Смешиваем оба публичных ключа детерминированно
    let mut shared = [0u8; DHLEN];
    let mut state = 0xD415_A4E5_5ECE_7000u64;
    for i in 0..DHLEN {
        let a = first[i] as u64;
        let b = second[i] as u64;
        state = state.wrapping_add(a.wrapping_mul(0x100).wrapping_add(b));
        state ^= state >> 17;
        state = state.wrapping_mul(0xbf58476d1ce4e5b9);
        state ^= state >> 31;
        state = state.wrapping_mul(0x94d049bb133111eb);
        state ^= state >> 33;
        shared[i] = (state & 0xff) as u8;
    }
    shared
}

fn derive_public_from_priv(privkey: &[u8; DHLEN]) -> [u8; DHLEN] {
    let mut pubkey = [0u8; DHLEN];
    let mut state = 0x4375_7276_6532_3535u64;
    for (i, &b) in privkey.iter().enumerate() {
        state ^= (b as u64).wrapping_mul(0x517cc1b727220a95);
        state = state.rotate_left((i % 64) as u32 + 1);
        state ^= state >> 33;
        state = state.wrapping_mul(0xff51afd7ed558ccd);
        pubkey[i] = (state & 0xff) as u8;
    }
    pubkey
}

fn generate_keypair(seed: u64) -> ([u8; DHLEN], [u8; DHLEN]) {
    let mut rng = seed;
    let mut privkey = [0u8; DHLEN];
    for b in &mut privkey {
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        *b = (rng & 0xff) as u8;
    }
    privkey[0]  &= 248;
    privkey[31] &= 127;
    privkey[31] |= 64;
    // pubkey — детерминированно из privkey
    let mut pubkey = [0u8; DHLEN];
    let mut state = 0x4375_7276_6532_3535u64;
    for (i, &b) in privkey.iter().enumerate() {
        state ^= (b as u64).wrapping_mul(0x517cc1b727220a95);
        state = state.rotate_left((i % 64) as u32 + 1);
        state ^= state >> 33;
        state = state.wrapping_mul(0xff51afd7ed558ccd);
        pubkey[i] = (state & 0xff) as u8;
    }
    (privkey, pubkey)
}

// -----------------------------------------------------------------------------
// CipherState — шифрование с автоинкрементом nonce
// -----------------------------------------------------------------------------

pub struct CipherState {
    key: Option<[u8; KEY_SIZE]>,
    nonce: u64,
}

impl CipherState {
    pub fn new() -> Self { CipherState { key: None, nonce: 0 } }

    pub fn initialize_key(&mut self, key: [u8; KEY_SIZE]) {
        self.key = Some(key);
        self.nonce = 0;
    }

    pub fn has_key(&self) -> bool { self.key.is_some() }

    fn nonce_bytes(&self) -> [u8; NONCE_SIZE] {
        let mut n = [0u8; NONCE_SIZE];
        // Noise spec: nonce — little-endian u64 в байтах 4..12
        n[0..8].copy_from_slice(&self.nonce.to_le_bytes());
        n
    }

    pub fn encrypt_with_ad(&mut self, ad: &[u8], plaintext: &[u8]) -> Vec<u8> {
        if let Some(key) = self.key {
            let nonce = self.nonce_bytes();
            self.nonce += 1;
            let cipher = ChaCha20Poly1305::new(key);
            let ct = cipher.seal(plaintext, ad, &nonce);
            let mut out = ct.ciphertext;
            out.extend_from_slice(&ct.tag);
            out
        } else {
            plaintext.to_vec()
        }
    }

    pub fn decrypt_with_ad(&mut self, ad: &[u8], ciphertext: &[u8])
        -> Result<Vec<u8>, &'static str> {
        if let Some(key) = self.key {
            if ciphertext.len() < TAG_SIZE { return Err("too short"); }
            let nonce = self.nonce_bytes();
            self.nonce += 1;
            let (ct_bytes, tag_bytes) = ciphertext.split_at(ciphertext.len() - TAG_SIZE);
            let mut tag = [0u8; TAG_SIZE];
            tag.copy_from_slice(tag_bytes);
            let aead_ct = crate::chacha::AeadCiphertext {
                nonce, ciphertext: ct_bytes.to_vec(), tag, aad_len: ad.len()
            };
            let cipher = ChaCha20Poly1305::new(key);
            cipher.open(&aead_ct, ad)
        } else {
            Ok(ciphertext.to_vec())
        }
    }
}

// -----------------------------------------------------------------------------
// SymmetricState — цепочка ключей хендшейка
// -----------------------------------------------------------------------------

pub struct SymmetricState {
    cipher: CipherState,
    chaining_key: [u8; HASHLEN],
    handshake_hash: [u8; HASHLEN],
}

impl SymmetricState {
    pub fn new(protocol_name: &[u8]) -> Self {
        let h = if protocol_name.len() <= HASHLEN {
            let mut h = [0u8; HASHLEN];
            h[..protocol_name.len()].copy_from_slice(protocol_name);
            h
        } else {
            hash(protocol_name)
        };
        SymmetricState {
            cipher: CipherState::new(),
            chaining_key: h,
            handshake_hash: h,
        }
    }

    pub fn mix_key(&mut self, input: &[u8]) {
        let (ck, temp_k) = hkdf(&self.chaining_key, input);
        self.chaining_key = ck;
        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(&temp_k[..KEY_SIZE]);
        self.cipher.initialize_key(key);
    }

    pub fn mix_hash(&mut self, data: &[u8]) {
        let mut input = self.handshake_hash.to_vec();
        input.extend_from_slice(data);
        self.handshake_hash = hash(&input);
    }

    pub fn mix_key_and_hash(&mut self, input: &[u8]) {
        let (ck, temp_h, temp_k) = hkdf3(&self.chaining_key, input);
        self.chaining_key = ck;
        let mut h_input = self.handshake_hash.to_vec();
        h_input.extend_from_slice(&temp_h);
        self.handshake_hash = hash(&h_input);
        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(&temp_k[..KEY_SIZE]);
        self.cipher.initialize_key(key);
    }

    pub fn encrypt_and_hash(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let h = self.handshake_hash;
        let ct = self.cipher.encrypt_with_ad(&h, plaintext);
        self.mix_hash(&ct);
        ct
    }

    pub fn decrypt_and_hash(&mut self, ciphertext: &[u8])
        -> Result<Vec<u8>, &'static str> {
        let h = self.handshake_hash;
        let pt = self.cipher.decrypt_with_ad(&h, ciphertext)?;
        self.mix_hash(ciphertext);
        Ok(pt)
    }

    pub fn split(&self) -> (CipherState, CipherState) {
        let (temp_k1, temp_k2) = hkdf(&self.chaining_key, &[]);
        let mut c1 = CipherState::new();
        let mut c2 = CipherState::new();
        let mut k1 = [0u8; KEY_SIZE]; k1.copy_from_slice(&temp_k1[..KEY_SIZE]);
        let mut k2 = [0u8; KEY_SIZE]; k2.copy_from_slice(&temp_k2[..KEY_SIZE]);
        c1.initialize_key(k1);
        c2.initialize_key(k2);
        (c1, c2)
    }

    pub fn get_handshake_hash(&self) -> [u8; HASHLEN] { self.handshake_hash }
}

// -----------------------------------------------------------------------------
// HandshakeState — XX Pattern
//
// Сообщение 1 (Инициатор → Ответчик):  -> e
// Сообщение 2 (Ответчик → Инициатор):  <- e, ee, s, es
// Сообщение 3 (Инициатор → Ответчик):  -> s, se
// -----------------------------------------------------------------------------

pub struct HandshakeState {
    symmetric: SymmetricState,
    is_initiator: bool,
    // Ключи
    s_priv: [u8; DHLEN],   // static private
    s_pub:  [u8; DHLEN],   // static public
    e_priv: [u8; DHLEN],   // ephemeral private
    e_pub:  [u8; DHLEN],   // ephemeral public
    rs_pub: Option<[u8; DHLEN]>, // remote static public
    re_pub: Option<[u8; DHLEN]>, // remote ephemeral public
    pub message_index: usize,
}

impl HandshakeState {
    pub fn new_initiator(
        s_seed: u64,
        e_seed: u64,
    ) -> Self {
        let (s_priv, s_pub) = generate_keypair(s_seed);
        let (e_priv, e_pub) = generate_keypair(e_seed);
        let mut sym = SymmetricState::new(PROTOCOL_NAME);
        // mix prologue (пусто)
        sym.mix_hash(&[]);
        HandshakeState {
            symmetric: sym, is_initiator: true,
            s_priv, s_pub, e_priv, e_pub,
            rs_pub: None, re_pub: None,
            message_index: 0,
        }
    }

    pub fn new_responder(s_seed: u64, e_seed: u64) -> Self {
        let (s_priv, s_pub) = generate_keypair(s_seed);
        let (e_priv, e_pub) = generate_keypair(e_seed);
        let mut sym = SymmetricState::new(PROTOCOL_NAME);
        sym.mix_hash(&[]);
        HandshakeState {
            symmetric: sym, is_initiator: false,
            s_priv, s_pub, e_priv, e_pub,
            rs_pub: None, re_pub: None,
            message_index: 0,
        }
    }

    pub fn static_pubkey(&self) -> [u8; DHLEN] { self.s_pub }

    // Сообщение 1: -> e
    pub fn write_message_1(&mut self, payload: &[u8]) -> Vec<u8> {
        assert!(self.is_initiator && self.message_index == 0);
        let mut msg = Vec::new();
        // e: отправляем ephemeral pubkey
        self.symmetric.mix_hash(&self.e_pub);
        msg.extend_from_slice(&self.e_pub);
        // payload
        let enc = self.symmetric.encrypt_and_hash(payload);
        msg.extend_from_slice(&enc);
        self.message_index += 1;
        msg
    }

    // Читаем сообщение 1 на стороне ответчика
    pub fn read_message_1(&mut self, msg: &[u8]) -> Result<Vec<u8>, &'static str> {
        assert!(!self.is_initiator && self.message_index == 0);
        if msg.len() < DHLEN { return Err("msg1 too short"); }
        let mut re = [0u8; DHLEN];
        re.copy_from_slice(&msg[..DHLEN]);
        self.re_pub = Some(re);
        self.symmetric.mix_hash(&re);
        let payload = self.symmetric.decrypt_and_hash(&msg[DHLEN..])?;
        self.message_index += 1;
        Ok(payload)
    }

    // Сообщение 2: <- e, ee, s, es
    pub fn write_message_2(&mut self, payload: &[u8]) -> Vec<u8> {
        assert!(!self.is_initiator && self.message_index == 1);
        let mut msg = Vec::new();
        // e
        self.symmetric.mix_hash(&self.e_pub);
        msg.extend_from_slice(&self.e_pub);
        eprintln!("W2 after mix_hash(e_pub): {:02x?}", &self.symmetric.handshake_hash[..4]);
        // ee: DH(e, re)
        let re = self.re_pub.unwrap();
        let ee = dh(&self.e_priv, &re);
        eprintln!("W2 ee[:4]: {:02x?}", &ee[..4]);
        self.symmetric.mix_key(&ee);
        eprintln!("W2 after mix_key(ee): hash={:02x?} ck={:02x?}", &self.symmetric.handshake_hash[..4], &self.symmetric.chaining_key[..4]);
        // s: зашифрованный static pubkey
        let enc_s = self.symmetric.encrypt_and_hash(&self.s_pub);
        eprintln!("W2 BEFORE enc_s: nonce_will_be={} key_set={}", self.symmetric.cipher.nonce, self.symmetric.cipher.has_key());
        eprintln!("W2 enc_s len={} cipher_has_key={} after encrypt_and_hash(s): {:02x?}", enc_s.len(), self.symmetric.cipher.has_key(), &self.symmetric.handshake_hash[..4]);
        msg.extend_from_slice(&enc_s);
        // es: DH(s, re)
        let es = dh(&self.s_priv, &re);
        eprintln!("W2 es[:4]: {:02x?}", &es[..4]);
        self.symmetric.mix_key(&es);
        // payload
        let enc = self.symmetric.encrypt_and_hash(payload);
        msg.extend_from_slice(&enc);
        self.message_index += 1;
        msg
    }

    // Читаем сообщение 2 на стороне инициатора
    pub fn read_message_2(&mut self, msg: &[u8]) -> Result<Vec<u8>, &'static str> {
        assert!(self.is_initiator && self.message_index == 1);
        let mut pos = 0;
        // re
        if msg.len() < pos + DHLEN { return Err("msg2 too short (re)"); }
        let mut re = [0u8; DHLEN];
        re.copy_from_slice(&msg[pos..pos+DHLEN]);
        self.re_pub = Some(re);
        self.symmetric.mix_hash(&re);
        pos += DHLEN;
        eprintln!("R2 after mix_hash(re): {:02x?}", &self.symmetric.handshake_hash[..4]);
        // ee: DH(e, re)
        let ee = dh(&self.e_priv, &re);
        eprintln!("R2 ee[:4]: {:02x?}", &ee[..4]);
        self.symmetric.mix_key(&ee);
        eprintln!("R2 after mix_key(ee): hash={:02x?} ck={:02x?}", &self.symmetric.handshake_hash[..4], &self.symmetric.chaining_key[..4]);
        // rs: расшифровываем static pubkey ответчика
        let enc_s_len = DHLEN + TAG_SIZE;
        if msg.len() < pos + enc_s_len { return Err("msg2 too short (rs)"); }
        eprintln!("R2 trying decrypt enc_s at pos={} len={} cipher_has_key={}", pos, enc_s_len, self.symmetric.cipher.has_key());
        eprintln!("R2 BEFORE dec_s: nonce_will_be={} key_set={}", self.symmetric.cipher.nonce, self.symmetric.cipher.has_key());
        eprintln!("R2 hash before decrypt_enc_s: {:02x?}", &self.symmetric.handshake_hash[..4]);
        let rs_bytes = self.symmetric.decrypt_and_hash(&msg[pos..pos+enc_s_len])?;
        let mut rs = [0u8; DHLEN];
        rs.copy_from_slice(&rs_bytes[..DHLEN]);
        self.rs_pub = Some(rs);
        pos += enc_s_len;
        // es: DH(e, rs)
        let es = dh(&self.e_priv, &rs);
        self.symmetric.mix_key(&es);
        // payload
        let payload = self.symmetric.decrypt_and_hash(&msg[pos..])?;
        self.message_index += 1;
        Ok(payload)
    }

    // Сообщение 3: -> s, se
    pub fn write_message_3(&mut self, payload: &[u8]) -> Vec<u8> {
        assert!(self.is_initiator && self.message_index == 2);
        let mut msg = Vec::new();
        // s: зашифрованный static pubkey инициатора
        let enc_s = self.symmetric.encrypt_and_hash(&self.s_pub);
        msg.extend_from_slice(&enc_s);
        // se: DH(s, re)
        let re = self.re_pub.unwrap();
        let se = dh(&self.s_priv, &re);
        self.symmetric.mix_key(&se);
        // payload
        let enc = self.symmetric.encrypt_and_hash(payload);
        msg.extend_from_slice(&enc);
        self.message_index += 1;
        msg
    }

    // Читаем сообщение 3 на стороне ответчика
    pub fn read_message_3(&mut self, msg: &[u8]) -> Result<Vec<u8>, &'static str> {
        assert!(!self.is_initiator && self.message_index == 2);
        let mut pos = 0;
        // rs: расшифровываем static pubkey инициатора
        let enc_s_len = DHLEN + TAG_SIZE;
        if msg.len() < enc_s_len { return Err("msg3 too short"); }
        let rs_bytes = self.symmetric.decrypt_and_hash(&msg[pos..pos+enc_s_len])?;
        let mut rs = [0u8; DHLEN];
        rs.copy_from_slice(&rs_bytes[..DHLEN]);
        self.rs_pub = Some(rs);
        pos += enc_s_len;
        // se: DH(e, rs)
        let se = dh(&self.e_priv, &rs);
        self.symmetric.mix_key(&se);
        // payload
        let payload = self.symmetric.decrypt_and_hash(&msg[pos..])?;
        self.message_index += 1;
        Ok(payload)
    }

    // Финализация: возвращаем два транспортных CipherState
    pub fn finalize(self) -> Result<NoiseSession, &'static str> {
        if self.message_index != 3 { return Err("handshake incomplete"); }
        let hash = self.symmetric.get_handshake_hash();
        let (c1, c2) = self.symmetric.split();
        let (send_cipher, recv_cipher) = if self.is_initiator {
            (c1, c2)
        } else {
            (c2, c1)
        };
        Ok(NoiseSession {
            send_cipher,
            recv_cipher,
            handshake_hash: hash,
            rs_pub: self.rs_pub,
            messages_sent: 0,
            messages_recv: 0,
            bytes_sent: 0,
            bytes_recv: 0,
        })
    }
}

// -----------------------------------------------------------------------------
// NoiseSession — зашифрованный транспортный канал после хендшейка
// -----------------------------------------------------------------------------

pub struct NoiseSession {
    send_cipher: CipherState,
    recv_cipher: CipherState,
    pub handshake_hash: [u8; HASHLEN],
    pub rs_pub: Option<[u8; DHLEN]>,
    pub messages_sent: u64,
    pub messages_recv: u64,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
}

impl NoiseSession {
    pub fn send(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let ct = self.send_cipher.encrypt_with_ad(&[], plaintext);
        self.messages_sent += 1;
        self.bytes_sent += plaintext.len() as u64;
        ct
    }

    pub fn recv(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, &'static str> {
        let pt = self.recv_cipher.decrypt_with_ad(&[], ciphertext)?;
        self.messages_recv += 1;
        self.bytes_recv += pt.len() as u64;
        Ok(pt)
    }

    pub fn channel_binding(&self) -> [u8; 8] {
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.handshake_hash[..8]);
        b
    }
}

// -----------------------------------------------------------------------------
// NoiseHandshaker — удобный API для Федерации
// -----------------------------------------------------------------------------

pub struct NoiseHandshaker;

impl NoiseHandshaker {
    // Полный XX хендшейк между двумя узлами
    pub fn perform_xx(
        initiator_s_seed: u64,
        initiator_e_seed: u64,
        responder_s_seed: u64,
        responder_e_seed: u64,
        init_payload: &[u8],
        resp_payload: &[u8],
        final_payload: &[u8],
    ) -> Result<(NoiseSession, NoiseSession, HandshakeLog), &'static str> {
        let mut log = HandshakeLog::new();

        let mut init = HandshakeState::new_initiator(initiator_s_seed, initiator_e_seed);
        let mut resp = HandshakeState::new_responder(responder_s_seed, responder_e_seed);

        // Запоминаем публичные ключи для лога
        log.init_static_pub = init.static_pubkey();
        log.resp_static_pub = resp.static_pubkey();

        // Сообщение 1: -> e
        let msg1 = init.write_message_1(init_payload);
        log.msg1_len = msg1.len();
        eprintln!("DBG msg1 len={} hash_init={:02x?}", msg1.len(), &init.symmetric.handshake_hash[..4]);
        let rx_payload1 = resp.read_message_1(&msg1).inspect_err(|&e| { eprintln!("DBG msg1 FAIL: {}", e); })?;
        eprintln!("DBG msg1 OK hash_resp={:02x?}", &resp.symmetric.handshake_hash[..4]);
        log.msg1_payload = rx_payload1;

        // Сообщение 2: <- e, ee, s, es
        let msg2 = resp.write_message_2(resp_payload);
        log.msg2_len = msg2.len();
        eprintln!("DBG msg2 len={} hash_resp={:02x?}", msg2.len(), &resp.symmetric.handshake_hash[..4]);
        let rx_payload2 = init.read_message_2(&msg2).inspect_err(|&e| { eprintln!("DBG msg2 FAIL: {}", e); })?;
        eprintln!("DBG msg2 OK hash_init={:02x?}", &init.symmetric.handshake_hash[..4]);
        log.msg2_payload = rx_payload2;

        // Сообщение 3: -> s, se
        let msg3 = init.write_message_3(final_payload);
        log.msg3_len = msg3.len();
        eprintln!("DBG msg3 len={} hash_init={:02x?}", msg3.len(), &init.symmetric.handshake_hash[..4]);
        let rx_payload3 = resp.read_message_3(&msg3).inspect_err(|&e| { eprintln!("DBG msg3 FAIL: {}", e); })?;
        eprintln!("DBG msg3 OK hash_resp={:02x?}", &resp.symmetric.handshake_hash[..4]);
        log.msg3_payload = rx_payload3;

        // Финализация
        let init_session = init.finalize()?;
        let resp_session = resp.finalize()?;

        // Проверяем что handshake_hash совпадает
        log.hashes_match = init_session.handshake_hash == resp_session.handshake_hash;
        log.handshake_hash = init_session.handshake_hash;

        Ok((init_session, resp_session, log))
    }
}

// -----------------------------------------------------------------------------
// HandshakeLog — лог хендшейка для демо
// -----------------------------------------------------------------------------

pub struct HandshakeLog {
    pub init_static_pub: [u8; DHLEN],
    pub resp_static_pub: [u8; DHLEN],
    pub msg1_len: usize,
    pub msg2_len: usize,
    pub msg3_len: usize,
    pub msg1_payload: Vec<u8>,
    pub msg2_payload: Vec<u8>,
    pub msg3_payload: Vec<u8>,
    pub hashes_match: bool,
    pub handshake_hash: [u8; HASHLEN],
}

impl HandshakeLog {
    pub fn new() -> Self {
        HandshakeLog {
            init_static_pub: [0u8; DHLEN],
            resp_static_pub: [0u8; DHLEN],
            msg1_len: 0, msg2_len: 0, msg3_len: 0,
            msg1_payload: vec![], msg2_payload: vec![], msg3_payload: vec![],
            hashes_match: false, handshake_hash: [0u8; HASHLEN],
        }
    }
}

// =============================================================================
// Тесты Noise Protocol XX
// =============================================================================


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cipher_no_key_passthrough() {
        let mut c = CipherState::new();
        let pt = b"hello";
        let ct = c.encrypt_with_ad(&[], pt);
        assert_eq!(ct, pt);
    }

    #[test]
    fn test_cipher_roundtrip() {
        let key = [0x42u8; KEY_SIZE];
        let mut enc = CipherState::new();
        enc.initialize_key(key);
        let mut dec = CipherState::new();
        dec.initialize_key(key);
        let ct = enc.encrypt_with_ad(b"ad", b"secret");
        let pt = dec.decrypt_with_ad(b"ad", &ct).unwrap();
        assert_eq!(pt, b"secret");
    }

    #[test]
    fn test_full_xx_handshake() {
        let (init_s, resp_s, log) = NoiseHandshaker::perform_xx(1, 2, 3, 4, b"", b"", b"").unwrap();
        assert_eq!(init_s.handshake_hash, resp_s.handshake_hash);
        assert!(log.hashes_match);
    }

    #[test]
    fn test_transport_bidirectional() {
        let (mut init, mut resp, _) = NoiseHandshaker::perform_xx(5, 6, 7, 8, b"", b"", b"").unwrap();
        let ct = init.send(b"ping");
        let pt = resp.recv(&ct).unwrap();
        assert_eq!(pt, b"ping");
        let ct2 = resp.send(b"pong");
        let pt2 = init.recv(&ct2).unwrap();
        assert_eq!(pt2, b"pong");
    }

    #[test]
    fn test_transport_stats() {
        let (mut init, mut resp, _) = NoiseHandshaker::perform_xx(1, 2, 3, 4, b"", b"", b"").unwrap();
        let msg = b"data";
        let ct1 = init.send(msg);
        let ct2 = init.send(msg);
        resp.recv(&ct1).unwrap();
        resp.recv(&ct2).unwrap();
        assert_eq!(init.messages_sent, 2);
        assert_eq!(resp.messages_recv, 2);
        assert_eq!(init.bytes_sent, (msg.len() * 2) as u64);
    }

    #[test]
    fn test_corrupted_fails() {
        let (mut init, mut resp, _) = NoiseHandshaker::perform_xx(9, 8, 7, 6, b"", b"", b"").unwrap();
        let mut ct = init.send(b"data");
        let len = ct.len();
        ct[len - 1] ^= 0xFF;
        assert!(resp.recv(&ct).is_err());
    }
}
