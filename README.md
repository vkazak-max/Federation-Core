# Federation Core  
### AI-Driven Cognitive Overlay Network

**Federation Core** — экспериментальная децентрализованная сеть нового поколения.  
Проект исследует замену статических протоколов маршрутизации (BGP/OSPF) на адаптивную ИИ-маршрутизацию с системой структурной осведомлённости (SSAU) и лёгким DAG-консенсусом.

> ⚠️ Status: Research / Experimental (MVP Phase 2)

---

## 🚀 Core Concepts

### 🧠 SSAU (Structural Self-Awareness Units)

Тензоры состояния сети, описывающие:

- latency (распределение задержки)
- jitter
- bandwidth
- reliability
- energy cost

Каждый узел формирует SSAU-тензоры и публикует их в сеть.

---

### 🤖 AI Routing (Softmax + Entropy)

Маршруты оцениваются по:

- латентности
- стабильности (Shannon entropy)
- стоимости
- уровню доверия узлов

Используется softmax-выбор + автоматическое переключение маршрута при росте энтропии.

---

### 🔗 DAG Consensus (Proof-of-Awareness)

Лёгкий in-memory DAG:

- Каждая запись — факт маршрутизации
- Triangle Check проверяет честность заявленных задержек
- Узлы получают PoA-награду за честные данные
- TrustRegistry динамически обновляет доверие

---

### 🧅 ZKP-Inspired Onion Routing

- Многослойное шифрование заголовков
- Nullifier-защита от replay-атак
- Blinded SSAU (частичное скрытие источника)

> В текущей версии используется криптографическая симуляция.  
> Production-замена: ChaCha20-Poly1305 + X25519 + zk-proofs.

---

### 🪞 Active Mimicry (Mirage Module)

При обнаружении сканирования:

- Генерируются ложные SSAU-тензоры
- Создаётся виртуальная топология-ловушка
- Атакующий зацикливается в псевдомаршрутах

---

## 🏗 Architecture (Phase 1–2)

```
tensor.rs    → SSAU + Trust + Triangle Check
routing.rs   → AI Router + Softmax + Entropy
dag.rs       → DAG Ledger + PoA
zkp.rs       → Onion + Commitment + Nullifiers
mirage.rs    → Active Mimicry Defense
network.rs   → JSON Protocol
p2p.rs       → TCP Overlay + Handshake
overlay.rs   → Full MVP Integration
main.rs      → Node bootstrap
```

---

## 🛠 Technology Stack

- Language: **Rust (async Tokio)**
- Networking: TCP overlay (MVP)
- Serialization: Serde / JSON
- Consensus: Custom DAG (in-memory)
- AI Routing: Softmax + Entropy heuristics
- Privacy Layer: Onion simulation

---

## ▶️ Run Node

```bash
cargo run --release
```

По умолчанию узел:

- слушает порт 7777
- запускает heartbeat
- активирует DAG + Router + Mirage
- выводит статус каждые 10 секунд

---

## 📊 Project Status

| Phase | Component | Status |
|-------|----------|--------|
| 1 | SSAU Tensor Engine | ✅ |
| 1 | AI Router | ✅ |
| 1 | TCP P2P | ✅ |
| 2 | DAG Consensus | ✅ |
| 2 | ZKP Onion | ✅ |
| 2 | Active Mimicry | ✅ |
| 2 | Overlay MVP | ✅ |
| 3 | Persistent Storage | ⏳ |
| 3 | Real Cryptography | ⏳ |

---

## 📖 Documentation

- `WHITE_PAPER.md`
- `MANIFESTO.md`

---

## 🤝 Contributing

Ищем:

- Rust developers
- Network engineers
- Cryptography researchers
- Distributed systems enthusiasts

Fork → Feature branch → Pull Request.

---

## ⚖️ License

See `LICENSE.txt`
