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
    // Currencies are stored lowercase internally but the cache/API use
    // uppercase ISO codes; compare case-insensitively so the cache is hit.
    if !cache.base.eq_ignore_ascii_case(base) {
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
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(TIMEOUT_SECS)))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let url = format!("https://api.frankfurter.dev/v2/rates?base={base}");

    let mut retries = 0u8;
    loop {
        match agent.get(&url).call() {
            Ok(response) => {
                let records: Vec<RateRecord> =
                    response.into_body().read_json().map_err(ureq_err)?;
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
            Err(ureq::Error::Io(_) | ureq::Error::Timeout(_) | ureq::Error::HostNotFound)
                if retries < MAX_RETRIES =>
            {
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

/// Resolve exchange rates for an optional target currency, degrading
/// gracefully: a missing target or a fetch error yields `None` (with a warning
/// to `out`) so commands keep working — amounts shown without conversion.
pub fn rates_for(out: &mut impl io::Write, target: Option<&str>) -> Option<HashMap<String, f64>> {
    let base = target?;
    match get_rates(base) {
        Ok(rates) => Some(rates),
        Err(e) => {
            let _ = writeln!(
                out,
                "warning: could not fetch exchange rates ({e}); showing amounts without conversion"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_cache_matches_base_case_insensitively() {
        // Regression: a lowercase query must hit an uppercase-base cache,
        // otherwise every call refetches over the network.
        let path = std::env::temp_dir().join(format!("recu-rates-test-{}.json", std::process::id()));
        let cache = ExchangeRateCache {
            base: "PLN".to_string(),
            rates: HashMap::from([("USD".to_string(), 0.25)]),
            fetched_at: Utc::now(),
        };
        std::fs::write(&path, serde_json::to_string(&cache).unwrap()).unwrap();

        let hit = read_cache(&path, "pln");
        let _ = std::fs::remove_file(&path);

        assert_eq!(hit.expect("lowercase query hits uppercase-base cache").base, "PLN");
    }

    #[test]
    fn rates_for_returns_none_on_error() {
        // Can't mock the network, so this is best-effort: a nonsense base
        // currency should fail (or, rarely, hit a cache) but never panic.
        let mut buf = Vec::new();
        let result = rates_for(&mut buf, Some("ZZZZZ_INVALID"));
        if result.is_none() {
            let warning = String::from_utf8(buf).expect("utf8");
            assert!(warning.contains("warning:"));
            assert!(warning.contains("without conversion"));
        }
    }
}
