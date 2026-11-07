//! Exchange rates: fetched on demand in a background thread, cached on disk for offline use.

use crate::num::{self, R};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Free, key-less API (fiat + crypto + metals, base USD). `{date}` is today's UTC date.
/// jsDelivr's cached `@latest` can lag a day, so dated and Cloudflare URLs come first.
const URLS: [&str; 4] = [
    "https://latest.currency-api.pages.dev/v1/currencies/usd.min.json",
    "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@{date}/v1/currencies/usd.min.json",
    "https://{date}.currency-api.pages.dev/v1/currencies/usd.min.json",
    "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/usd.min.json",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rates {
    /// Date the provider published these rates (YYYY-MM-DD).
    pub date: String,
    /// Unix seconds when we fetched them.
    pub fetched_at: u64,
    /// Units of each currency per 1 USD, lowercase codes.
    usd: HashMap<String, f64>,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Today's UTC date as YYYY-MM-DD (civil-from-days, no date library needed).
pub fn today_utc() -> String {
    let z = (now() / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    format!("{y:04}-{m:02}-{d:02}")
}

impl Rates {
    /// Exact rate as published (the API's decimal, not a float approximation).
    pub fn get(&self, code: &str) -> Option<R> {
        let v = *self.usd.get(&code.to_lowercase())?;
        (v.is_finite() && v > 0.0).then(|| num::from_f64(v)).flatten()
    }

    pub fn age_secs(&self) -> u64 {
        now().saturating_sub(self.fetched_at)
    }

    #[cfg(test)]
    pub fn from_pairs(pairs: &[(&str, f64)]) -> Self {
        Self { date: "test".into(), fetched_at: now(), usd: pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect() }
    }

    fn cache_path() -> Option<std::path::PathBuf> {
        Some(dirs::cache_dir()?.join("consertion").join("rates.json"))
    }

    pub fn load_cache() -> Option<Rates> {
        serde_json::from_slice(&std::fs::read(Self::cache_path()?).ok()?).ok()
    }

    fn save_cache(&self) {
        if let Some(p) = Self::cache_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            // Write-then-rename so a crash never leaves a half-written cache.
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, serde_json::to_vec(self).unwrap_or_default()).is_ok() {
                let _ = std::fs::rename(tmp, p);
            }
        }
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    date: String,
    usd: HashMap<String, f64>,
}

/// Tries the sources in order and keeps the newest answer; stops early once one is from today.
pub fn fetch() -> Result<Rates, String> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(8)).build();
    let today = today_utc();
    let mut best: Option<ApiResponse> = None;
    let mut last_err = String::new();
    for url in URLS {
        let url = url.replace("{date}", &today);
        let resp = agent.get(&url).call().map_err(|e| e.to_string()).and_then(|r| r.into_string().map_err(|e| e.to_string()));
        match resp.map(|body| serde_json::from_str::<ApiResponse>(&body)) {
            Ok(Ok(api)) if !api.usd.is_empty() => {
                if best.as_ref().is_none_or(|b| api.date > b.date) {
                    best = Some(api);
                }
                if best.as_ref().is_some_and(|b| b.date >= today) {
                    break;
                }
            }
            Ok(_) => last_err = "Unexpected response from rates server".into(),
            Err(e) => last_err = e,
        }
    }
    let api = best.ok_or(last_err)?;
    let rates = Rates { date: api.date, fetched_at: now(), usd: api.usd };
    rates.save_cache();
    Ok(rates)
}

/// Starts a fetch in the background; poll the receiver from the UI loop.
pub fn fetch_async() -> Receiver<Result<Rates, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(fetch());
    });
    rx
}

#[cfg(test)]
mod tests {
    #[test]
    fn today_is_a_date() {
        let d = super::today_utc();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20") && d.as_bytes()[4] == b'-' && d.as_bytes()[7] == b'-');
    }
}
