# Polymarket 15-Minute Up/Down Arbitrage Bot

**Automated market-making and arbitrage bot for Polymarket 15-minute crypto Up/Down prediction markets (BTC, ETH, SOL, XRP).** Place pre-limit orders on both sides, manage risk with signal-based logic, and redeem winning positions at resolution—all in Rust.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Polymarket](https://img.shields.io/badge/Polymarket-15m%20markets-blue)](https://polymarket.com)

---

## Table of Contents

- [What Is This Bot?](#what-is-this-bot)
- [How Does the Bot Work?](#how-does-the-bot-work)
- [Supported Markets](#supported-markets)
- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Strategy Logic in Detail](#strategy-logic-in-detail)
- [Risk Disclaimer](#risk-disclaimer)
- [Contact & Support](#contact--support)

---

## What Is This Bot?

This bot trades **Polymarket 15-minute Up/Down prediction markets** for **Bitcoin (BTC), Ethereum (ETH), Solana (SOL), and XRP**. It uses a **pre-order arbitrage strategy**: it places limit buy orders on **both** the Up and Down tokens before or at the start of each 15-minute period. If both orders fill at or below your limit price, you are guaranteed a small profit at resolution (one side pays $1 per share). The bot also includes **signal-based risk management** (when to place orders, when to skip, when to sell early) and **automatic redemption** when markets resolve.

**Use cases:** automated market-making on Polymarket 15m crypto markets, hedging both outcomes, and capturing edge when Up and Down can both be bought below 0.50 combined.

---

## How Does the Bot Work?

### High-Level Flow

1. **Period alignment**  
   The bot uses **15-minute periods** in **Eastern Time (ET)**, aligned to :00, :15, :30, and :45 (e.g. 2:00, 2:15, 2:30, 2:45).

2. **Market discovery**  
   For each asset (BTC, ETH, SOL, XRP), markets are found by **slug** in the form:
   - `btc-updown-15m-{timestamp}`
   - `eth-updown-15m-{timestamp}`
   - `sol-updown-15m-{timestamp}`
   - `xrp-updown-15m-{timestamp}`  
   The `timestamp` is the **period start** (Unix seconds). The bot discovers the **current** and **next** period markets via the Polymarket Gamma API.

3. **Pre-orders for the next period**  
   When the time until the **next** 15m period is less than or equal to `place_order_before_mins` (e.g. 2–3 minutes), the bot:
   - Optionally evaluates a **signal** on the **current** market (see [Strategy Logic](#strategy-logic-in-detail)).
   - If the signal is **Good**, it looks up the **next** period market by slug and places **limit buy** orders on both **Up** and **Down** at your configured `price_limit` (e.g. 0.45).
   - If the signal is **Bad**, it **skips** placing pre-orders for the next period.

4. **After orders are placed**  
   - The bot periodically checks whether orders have **filled** (via CLOB API in production, or price vs limit in simulation).
   - It maintains **per-asset state**: which orders are filled, expiry time, and whether it has already sold one side or redeemed.

5. **When both sides are filled**  
   - If one side’s **sell price** rises above `sell_opposite_above` (e.g. 0.84) **and** the time remaining in the period is ≤ `sell_opposite_time_remaining` minutes, the bot **sells the losing side** (market sell) and **holds the winning side** to resolution.
   - At resolution, the winning token pays **$1 per share**; the bot **redeems** that position automatically (or you can use the redeem CLI).

6. **One side filled only (risk management)**  
   If only **Up** or only **Down** is filled, the bot can **sell that side** and **cancel** the other order to limit loss:
   - **Price-based:** sell when the matched token’s price falls to or below `danger_price`.
   - **Time-based:** sell after `danger_time_passed` minutes with only one side filled.

7. **Mid-market orders (optional)**  
   If `mid_market_enabled` is true and there is enough time left in the **current** period, the bot may place **limit orders on the current** market (not just the next one), using dynamic prices derived from current Up/Down prices.

8. **Resolution and redemption**  
   A background task runs every `market_closure_check_interval_seconds`. When a market is **closed** and the bot holds a winning position, it **redeems** the winning tokens and updates PnL.

### Summary Diagram

```
Current 15m period (e.g. 2:00–2:15 ET)
    │
    ├── Optional: mid-market orders on current market (if signal Good, time left)
    │
    └── When ≤ place_order_before_mins until NEXT period (e.g. 2:13)
            │
            ├── Evaluate signal on CURRENT market
            ├── If Good → place limit buys (Up + Down) on NEXT period market
            └── If Bad  → skip next period
                    │
                    ▼
Next period starts (e.g. 2:15)
    │
    ├── Orders can fill (both / one / none)
    ├── If both filled → optionally sell loser when winner price high + time low
    ├── If one filled  → danger logic (price or time) → sell + cancel other
    └── At period end → redeem winner, record PnL
```

---

## Supported Markets

| Asset | Slug pattern        | Example                    |
|-------|---------------------|----------------------------|
| BTC   | `btc-updown-15m-{ts}`  | `btc-updown-15m-1771007400` |
| ETH   | `eth-updown-15m-{ts}`  | `eth-updown-15m-1771007400` |
| SOL   | `sol-updown-15m-{ts}`  | `sol-updown-15m-1771007400` |
| XRP   | `xrp-updown-15m-{ts}`  | `xrp-updown-15m-1771007400` |

The bot runs the same logic **in parallel** for all four assets. Each asset has its own state (orders, fills, expiry).

---

## Features

- **Multi-asset 15m support:** BTC, ETH, SOL, XRP in one process.
- **Pre-order strategy:** Limit buys on both Up and Down before/at period start.
- **Signal-based placement:** Good/Bad/Unknown signal to decide whether to place pre-orders for the next period.
- **Sell-opposite logic:** When both filled, sell the losing side if the winner’s price is high and time is short.
- **One-side risk management:** Price-based or time-based early exit when only one side fills.
- **Mid-market orders:** Optional limit orders on the **current** period market.
- **Simulation mode:** Run without placing real orders; match logic based on price vs limit.
- **Automatic redemption:** Redeem winning positions when markets resolve.
- **Redeem CLI:** Manual redeem by condition ID or fetch all redeemable positions for your proxy wallet.

---

## Requirements

- **Rust** 1.70+ (`rustup` recommended).
- **Polymarket API credentials:** API key, secret, passphrase (for CLOB), and optionally a **private key** + **proxy wallet** for signing and redemption.
- **Network:** Access to Polymarket Gamma API and CLOB (default endpoints in config).

---

## Installation

```bash
# Clone the repository (replace with your fork or repo URL)
git clone https://github.com/crellos/polymarket-arbitrage-bot-pre-order-15m-markets.git
cd polymarket-arbitrage-bot-pre-order-15m-markets

# Build release binary
cargo build --release
```

The binary will be at `target/release/polymarket-arbitrage-bot`.

---

## Configuration

Configuration is read from **`config.json`** by default (override with `-c` / `--config`). Copy the example file and fill in your credentials:

```bash
cp config.json.example config.json
# Edit config.json with your Polymarket API keys and wallet details
```

### Example structure

```json
{
  "polymarket": {
    "gamma_api_url": "https://gamma-api.polymarket.com",
    "clob_api_url": "https://clob.polymarket.com",
    "api_key": "YOUR_API_KEY",
    "api_secret": "YOUR_API_SECRET",
    "api_passphrase": "YOUR_PASSPHRASE",
    "private_key": "YOUR_PRIVATE_KEY_HEX",
    "proxy_wallet_address": "0xYourProxyWallet",
    "signature_type": 2
  },
  "strategy": {
    "price_limit": 0.45,
    "shares": 5,
    "place_order_before_mins": 2,
    "check_interval_ms": 500,
    "simulation_mode": false,
    "sell_opposite_above": 0.84,
    "sell_opposite_time_remaining": 15,
    "market_closure_check_interval_seconds": 60,
    "signal": {
      "enabled": true,
      "stable_min": 0.35,
      "stable_max": 0.65,
      "clear_threshold": 0.9,
      "clear_remaining_mins": 3,
      "danger_price": 0.28,
      "danger_time_passed": 15,
      "one_side_buy_risk_management": "time",
      "mid_market_enabled": true
    }
  }
}
```

### Polymarket API

| Field                  | Description |
|------------------------|-------------|
| `gamma_api_url`        | Gamma API base URL (market/event data). |
| `clob_api_url`         | CLOB API base URL (order book, orders). |
| `api_key` / `api_secret` / `api_passphrase` | CLOB API credentials. |
| `private_key`          | Wallet private key (hex) for signing; optional for monitoring only. |
| `proxy_wallet_address` | Proxy wallet used for trading and redemption. |
| `signature_type`       | Signature type for CLOB (e.g. 2). |

### Strategy

| Field                             | Description |
|-----------------------------------|-------------|
| `price_limit`                     | Limit price for pre-orders (e.g. 0.45 = 45¢). |
| `shares`                          | Size per order (same for Up and Down). |
| `place_order_before_mins`         | Place pre-orders when this many minutes before the **next** 15m period. |
| `check_interval_ms`               | Main loop interval (ms). |
| `simulation_mode`                 | If `true`, no real orders; fills inferred from price vs limit. |
| `sell_opposite_above`             | When **both** filled, sell the loser only if the winner’s price ≥ this (e.g. 0.84). |
| `sell_opposite_time_remaining`    | And only if minutes left in period ≤ this (e.g. 15; for 15m you may use 3–5). |
| `market_closure_check_interval_seconds` | How often to check for resolved markets and run redemption. |

### Signal (risk / placement)

| Field                           | Description |
|---------------------------------|-------------|
| `enabled`                       | Use signal to allow/skip pre-orders and mid-market orders. |
| `stable_min` / `stable_max`     | “Good” signal: current Up/Down prices in this range (e.g. 0.35–0.65). |
| `clear_threshold`               | “Bad” if either side ≥ this (e.g. 0.9). |
| `clear_remaining_mins`         | Time-remaining condition for clear signal. |
| `danger_price`                  | For one-side risk: sell matched side if its price ≤ this. |
| `danger_time_passed`            | For one-side risk: sell after this many minutes with only one side filled. |
| `one_side_buy_risk_management`  | `"price"` or `"time"` (or `"none"`). |
| `mid_market_enabled`            | Allow placing orders on the **current** period market when signal is Good. |

If `config.json` does not exist, the bot can create a default one (see code: `Config::load`).

---

## Usage

### Run the bot (live or simulation)

```bash
# Use default config.json
./target/release/polymarket-arbitrage-bot

# Custom config path
./target/release/polymarket-arbitrage-bot --config /path/to/config.json
```

Set `strategy.simulation_mode` to `true` in config to run without placing real orders.

### Redeem winning positions

```bash
# Redeem a specific condition (condition_id in hex, with or without 0x prefix)
./target/release/polymarket-arbitrage-bot --redeem --condition-id 0x...

# Fetch all redeemable positions for proxy_wallet_address and redeem them
./target/release/polymarket-arbitrage-bot --redeem
```

`--redeem` requires `proxy_wallet_address` in config.

### Logging

Log level is controlled by the `RUST_LOG` environment variable (e.g. `info`, `debug`).

```bash
RUST_LOG=info ./target/release/polymarket-arbitrage-bot
RUST_LOG=debug ./target/release/polymarket-arbitrage-bot
```

---

## Strategy Logic in Detail

- **Good signal:** Up and Down prices in the current market are within `stable_min`–`stable_max`, and no “clear” condition (e.g. one side ≥ `clear_threshold` with little time left). → Bot may place pre-orders for next period and (if enabled) mid-market orders on current period.
- **Bad signal:** Clear condition met (e.g. one side very high near period end). → Bot skips pre-orders for the **next** 15m period.
- **Both filled:** If winner’s sell price ≥ `sell_opposite_above` and minutes remaining ≤ `sell_opposite_time_remaining`, sell the loser, hold winner to resolution, then redeem.
- **One side filled:** Depending on `one_side_buy_risk_management`, sell the matched side when price ≤ `danger_price` or after `danger_time_passed` minutes, and cancel the other order.

All times are based on the **current 15-minute period** in ET; market slugs follow the `{asset}-updown-15m-{timestamp}` convention used by Polymarket 15m markets.

---

## Risk Disclaimer

This bot interacts with real funds and Polymarket’s live APIs. Use at your own risk.

- **No guarantee of profit:** Past behavior and simulation are not guarantees of future results.
- **API and connectivity:** Failures or rate limits can prevent orders or redemption.
- **Market risk:** Slippage, illiquidity, and resolution rules apply.
- **Credentials:** Keep API keys and private keys secure; never commit them to version control.

The authors and contributors are not responsible for any financial loss. Prefer testing in **simulation mode** and small size before live use.

---

## Contact & Support

- **GitHub:** [crellos](https://github.com/crellos)
- **Telegram:** [crellos_0x](https://t.me/crellos_0x)

For questions, bug reports, or feature ideas related to this Polymarket 15m arbitrage bot, reach out via Telegram or open an issue on GitHub.

---

*Polymarket 15-Minute Up/Down Arbitrage Bot — automated pre-order strategy for BTC, ETH, SOL, and XRP 15m prediction markets.*
