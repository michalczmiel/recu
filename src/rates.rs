use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const TIMEOUT_SECS: u64 = 5;
const MAX_RETRIES: u8 = 2;

#[derive(Debug, Serialize, Deserialize)]
struct ExchangeRateCache {
    base: String,
    rates: HashMap<String, f64>,
    fetched_at: DateTime<Utc>,
}

// The v2 API returns a JSON array of rate records.
#[derive(Deserialize)]
struct RateRecord {
    base: String,
    quote: String,
    rate: f64,
}

fn cache_path() -> io::Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot determine home directory"))
        .map(|home| home.join(".cache").join("recu").join("rates.json"))
}

fn read_cache(path: &Path, base: &str) -> Option<ExchangeRateCache> {
    let content = std::fs::read_to_string(path).ok()?;
    let cache: ExchangeRateCache = serde_json::from_str(&content).ok()?;
    if cache.base != base {
        return None;
    }
    let age = Utc::now().signed_duration_since(cache.fetched_at);
    if age.num_hours() >= 24 {
        return None;
    }
    Some(cache)
}

fn ureq_err(e: ureq::Error) -> io::Error {
    io::Error::other(e)
}

fn fetch_rates(base: &str) -> io::Result<ExchangeRateCache> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build();
    let url = format!("https://api.frankfurter.dev/v2/rates?base={base}");

    let mut retries = 0u8;
    loop {
        match agent.get(&url).call() {
            Ok(response) => {
                let records: Vec<RateRecord> = response.into_json()?;
                let rates = records.iter().map(|r| (r.quote.clone(), r.rate)).collect();
                let base = records
                    .first()
                    .map_or_else(|| base.to_uppercase(), |r| r.base.clone());
                return Ok(ExchangeRateCache {
                    base,
                    rates,
                    fetched_at: Utc::now(),
                });
            }
            Err(ureq::Error::Transport(_)) if retries < MAX_RETRIES => {
                retries += 1;
            }
            Err(e) => return Err(ureq_err(e)),
        }
    }
}

fn write_cache(path: &Path, cache: &ExchangeRateCache) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string(cache).map_err(io::Error::other)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &content)?;
    std::fs::rename(tmp, path)
}

pub fn get_rates(base_currency: &str) -> io::Result<HashMap<String, f64>> {
    let path = cache_path()?;
    if let Some(cache) = read_cache(&path, base_currency) {
        return Ok(cache.rates);
    }
    let cache = fetch_rates(base_currency)?;
    let _ = write_cache(&path, &cache);
    Ok(cache.rates)
}
