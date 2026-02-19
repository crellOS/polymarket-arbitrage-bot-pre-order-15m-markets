use crate::api::PolymarketApi;
use anyhow::Result;
use chrono::{Datelike, TimeZone, Timelike};
use chrono_tz::America::New_York;
use std::sync::Arc;

pub const ASSET_TO_SLUG: &[(&str, &str)] = &[
    ("BTC", "bitcoin"),
    ("ETH", "ethereum"),
    ("SOL", "solana"),
    ("XRP", "xrp"),
];

pub struct MarketDiscovery {
    api: Arc<PolymarketApi>,
}

impl MarketDiscovery {
    pub fn new(api: Arc<PolymarketApi>) -> Self {
        Self { api }
    }

    pub fn build_1h_slug(asset_slug: &str, period_start_et: i64) -> String {
        let dt_et = New_York.timestamp_opt(period_start_et, 0).single().unwrap();
        let month_str = match dt_et.month() {
            1 => "january",
            2 => "february",
            3 => "march",
            4 => "april",
            5 => "may",
            6 => "june",
            7 => "july",
            8 => "august",
            9 => "september",
            10 => "october",
            11 => "november",
            12 => "december",
            _ => "january",
        };
        let day = dt_et.day();
        let hour24 = dt_et.hour();
        let (hour12, am_pm) = match hour24 {
            0 => (12, "am"),
            1..=11 => (hour24, "am"),
            12 => (12, "pm"),
            _ => (hour24 - 12, "pm"),
        };
        format!(
            "{}-up-or-down-{}-{}-{}{}-et",
            asset_slug, month_str, day, hour12, am_pm
        )
    }


    pub fn current_1h_period_start_et() -> i64 {
        let now_utc = chrono::Utc::now();
        let now_et = now_utc.with_timezone(&New_York);

        let hour_start_et = New_York
            .with_ymd_and_hms(
                now_et.year(),
                now_et.month(),
                now_et.day(),
                now_et.hour(),
                0,
                0,
            )
            .single()
            .unwrap();

        hour_start_et.timestamp()
    }

    /// 15m market slug format: btc-updown-15m-{period_start_timestamp}
    pub fn build_15m_slug(asset_ticker: &str, period_start_et: i64) -> String {
        let asset = asset_ticker.to_lowercase();
        format!("{}-updown-15m-{}", asset, period_start_et)
    }

    /// Current 15-minute period start (ET), rounded down to :00, :15, :30, :45.
    pub fn current_15m_period_start_et() -> i64 {
        let now_utc = chrono::Utc::now();
        let now_et = now_utc.with_timezone(&New_York);
        let minute = now_et.minute();
        let minute_floor = (minute / 15) * 15;

        let period_start_et = New_York
            .with_ymd_and_hms(
                now_et.year(),
                now_et.month(),
                now_et.day(),
                now_et.hour(),
                minute_floor,
                0,
            )
            .single()
            .unwrap();

        period_start_et.timestamp()
    }

    pub async fn get_market_tokens(&self, condition_id: &str) -> Result<(String, String)> {
        let details = self.api.get_market(condition_id).await?;
        let mut up_token = None;
        let mut down_token = None;

        for token in details.tokens {
            let outcome = token.outcome.to_uppercase();
            if outcome.contains("UP") || outcome == "1" {
                up_token = Some(token.token_id);
            } else if outcome.contains("DOWN") || outcome == "0" {
                down_token = Some(token.token_id);
            }
        }

        let up = up_token.ok_or_else(|| anyhow::anyhow!("Up token not found"))?;
        let down = down_token.ok_or_else(|| anyhow::anyhow!("Down token not found"))?;

        Ok((up, down))
    }
}
