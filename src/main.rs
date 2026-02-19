mod api;
mod config;
mod models;
mod discovery;
mod signals;
mod strategy;


use anyhow::Result;
use clap::Parser;
use config::{Args, Config};
use std::io::Write;
use std::sync::Arc;
use api::PolymarketApi;
use strategy::PreLimitStrategy;
use log::warn;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format(|buf, record| {
            writeln!(buf, "{}", record.args())
        })
        .init();

    let args = Args::parse();
    let config = Config::load(&args.config)?;
    let shares = config.strategy.shares;
    let price = config.strategy.price_limit;
    let cost_per_side = shares * price;
    let payout_per_trade = cost_per_side * 2.0;
    const N_ASSETS: u32 = 4;
    let four_assets = (N_ASSETS as f64) * cost_per_side;

    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("📋 Confirming configuration");
    eprintln!("   shares per side        {:.0}", shares);
    eprintln!("   ave price per share   ${:.2}", price);
    eprintln!("   payout per trade      ${:.0} × 2 = ${:.0}", cost_per_side, payout_per_trade);
    eprintln!("   {} assets              ${:.0}", N_ASSETS, four_assets);
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    eprintln!("🚀 Starting Polymarket Pre-Limit Order Bot");
    if config.strategy.simulation_mode {
        eprintln!("🎮 SIMULATION MODE ENABLED - No real orders will be placed");
        eprintln!("   Orders will match when prices hit ${:.2} or below", config.strategy.price_limit);
    }
    eprintln!("📈 Strategy: Placing Up/Down limit orders at ${:.2} for 15m markets (BTC, ETH, SOL, XRP)", config.strategy.price_limit);
    if config.strategy.signal.enabled {
        eprintln!("   📡 Signal-based risk management: enabled (place on good signal, skip on bad, sell early on danger)");
    }

    let api = Arc::new(PolymarketApi::new(
        config.polymarket.gamma_api_url.clone(),
        config.polymarket.clob_api_url.clone(),
        config.polymarket.api_key.clone(),
        config.polymarket.api_secret.clone(),
        config.polymarket.api_passphrase.clone(),
        config.polymarket.private_key.clone(),
        config.polymarket.proxy_wallet_address.clone(),
        config.polymarket.signature_type,
    ));

    if args.redeem {
        run_redeem_only(api.as_ref(), &config, args.condition_id.as_deref()).await?;
        return Ok(());
    }

    if config.polymarket.private_key.is_some() {
        if let Err(e) = api.authenticate().await {
            log::error!("Authentication failed: {}", e);
            anyhow::bail!("Authentication failed. Please check your credentials.");
        }
    } else {
        log::warn!("⚠️ No private key provided. Bot will only be able to monitor markets.");
    }


    let market_closure_interval = config.strategy.market_closure_check_interval_seconds;
    let strategy = Arc::new(PreLimitStrategy::new(api, config));
    let strategy_for_closure = Arc::clone(&strategy);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(market_closure_interval));
        loop {
            interval.tick().await;
            if let Err(e) = strategy_for_closure.check_market_closure().await {
                warn!("Error checking market closure: {}", e);
            }
            let total_profit = strategy_for_closure.get_total_profit().await;
            let period_profit = strategy_for_closure.get_period_profit().await;
            if total_profit != 0.0 || period_profit != 0.0 {
                eprintln!("Current Profit - Period: ${:.2} | Total: ${:.2}", period_profit, total_profit);
            }
        }
    });

    strategy.run().await
}

    
async fn run_redeem_only(
    api: &PolymarketApi,
    config: &Config,
    condition_id: Option<&str>,
) -> Result<()> {
    let proxy = config
        .polymarket
        .proxy_wallet_address
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--redeem requires proxy_wallet_address in config.json"))?;

    eprintln!("Redeem-only mode (proxy: {})", proxy);
    let cids: Vec<String> = if let Some(cid) = condition_id {
        let cid = if cid.starts_with("0x") { cid.to_string() } else { format!("0x{}", cid) };
        eprintln!("Redeeming condition: {}", cid);
        vec![cid]
    } else {
        eprintln!("Fetching redeemable positions...");
        let list = api.get_redeemable_positions(proxy).await?;
        if list.is_empty() {
            eprintln!("No redeemable positions found.");
            return Ok(());
        }
        eprintln!("Found {} condition(s) to redeem.", list.len());
        list
    };

    let mut ok_count = 0u32;
    let mut fail_count = 0u32;
    for cid in &cids {
        eprintln!("\n--- Redeeming condition {} ---", &cid[..cid.len().min(18)]);
        match api.redeem_tokens(cid, "", "Up").await {
            Ok(_) => {
                eprintln!("Success: {}", cid);
                ok_count += 1;
            }
            Err(e) => {
                eprintln!("Failed to redeem {}: {} (skipping)", cid, e);
                fail_count += 1;
            }
        }
    }
    eprintln!("\nRedeem complete. Succeeded: {}, Failed: {}", ok_count, fail_count);
    Ok(())
}

