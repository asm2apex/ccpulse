//! Transcript scanner.
//!
//! Two responsibilities:
//! 1. Per-session cumulative cache_create / cache_read totals (the stdin
//!    schema only exposes cumulative input/output, not the cache split).
//! 2. Cross-project rolling cost aggregation for the 5h / 7d windows. The
//!    rate-limit windows from stdin only carry token percentages; to show
//!    the equivalent USD spend, we compute it locally from each assistant
//!    message's usage and the model's published API price.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::util::{home_dir, now_secs};

const RECORDS_TTL_SEC: i64 = 25;
const RECORDS_LOOKBACK_SEC: i64 = 14 * 86400 + 3600;
const RECORDS_CACHE_REL: &str = ".claude/cache/ccpulse-records.json";

fn parse_iso_to_secs(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
}

#[derive(Debug, Default)]
pub struct SessionStats {
    pub input: u64,
    pub output: u64,
    pub cache_create: u64,
    pub cache_read: u64,
}

pub fn scan_session(path: Option<&str>) -> SessionStats {
    let mut stats = SessionStats::default();
    let path = match path {
        Some(p) => PathBuf::from(p),
        None => return stats,
    };
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return stats,
    };
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let msg = match v.get("message") {
            Some(m) => m,
            None => continue,
        };
        if msg.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            continue;
        }
        let usage = match msg.get("usage") {
            Some(u) => u,
            None => continue,
        };
        stats.input += usage
            .get("input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        stats.output += usage
            .get("output_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        stats.cache_create += usage
            .get("cache_creation_input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        stats.cache_read += usage
            .get("cache_read_input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
    }
    stats
}

/// Anthropic API list price (USD per 1M tokens) for input / output / cache_read /
/// cache_creation. Tracks the public price sheet as of release; numbers may
/// drift if Anthropic re-prices a tier.
fn price_per_million(model_id: &str) -> (f64, f64, f64, f64) {
    let m = model_id.to_lowercase();
    if m.contains("opus") {
        (15.0, 75.0, 1.50, 18.75)
    } else if m.contains("haiku") {
        (0.80, 4.0, 0.08, 1.00)
    } else {
        // Sonnet (default for unknown identifiers).
        (3.0, 15.0, 0.30, 3.75)
    }
}

fn estimate_cost_usd(model_id: &str, usage: &Value) -> f64 {
    let (in_p, out_p, cr_p, cc_p) = price_per_million(model_id);
    let f = |k: &str| usage.get(k).and_then(|x| x.as_u64()).unwrap_or(0) as f64;
    (f("input_tokens") * in_p
        + f("output_tokens") * out_p
        + f("cache_read_input_tokens") * cr_p
        + f("cache_creation_input_tokens") * cc_p)
        / 1_000_000.0
}

#[derive(Serialize, Deserialize, Default)]
struct RecordsCache {
    /// Tuples of (timestamp_secs, cost_usd) for assistant messages within
    /// `RECORDS_LOOKBACK_SEC`.
    records: Vec<(i64, f64)>,
    computed_at: i64,
}

fn records_cache_path() -> PathBuf {
    home_dir().join(RECORDS_CACHE_REL)
}

/// Returns (timestamp_secs, cost_usd) for every assistant message under
/// `~/.claude/projects/*/*.jsonl` within the lookback window. Cached for
/// `RECORDS_TTL_SEC` seconds so successive renders don't re-scan everything.
pub fn collect_records() -> Vec<(i64, f64)> {
    let now = now_secs();
    let cache_path = records_cache_path();

    if let Ok(s) = fs::read_to_string(&cache_path) {
        if let Ok(c) = serde_json::from_str::<RecordsCache>(&s) {
            if c.computed_at + RECORDS_TTL_SEC > now {
                return c.records;
            }
        }
    }

    let cutoff = now - RECORDS_LOOKBACK_SEC;
    let mut records: Vec<(i64, f64)> = Vec::with_capacity(2048);
    // Resumed sessions / sub-session transcripts duplicate the same
    // assistant message in multiple JSONL files. Dedup by message id.
    let mut seen: HashSet<String> = HashSet::with_capacity(2048);
    let projects = home_dir().join(".claude/projects");
    let dirs = match fs::read_dir(&projects) {
        Ok(d) => d,
        Err(_) => return records,
    };
    for proj in dirs.flatten() {
        let pp = proj.path();
        if !pp.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&pp) {
            Ok(d) => d,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let fp = entry.path();
            if fp.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }
            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if mtime < cutoff {
                continue;
            }
            let f = match File::open(&fp) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                let v: Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let ts_s = match v.get("timestamp").and_then(|t| t.as_str()) {
                    Some(s) => s,
                    None => continue,
                };
                let ts = match parse_iso_to_secs(ts_s) {
                    Some(t) => t,
                    None => continue,
                };
                if ts < cutoff {
                    continue;
                }
                let msg = match v.get("message") {
                    Some(m) => m,
                    None => continue,
                };
                if msg.get("role").and_then(|r| r.as_str()) != Some("assistant") {
                    continue;
                }
                if let Some(id) = msg.get("id").and_then(|x| x.as_str()) {
                    if !seen.insert(id.to_string()) {
                        continue;
                    }
                }
                let usage = match msg.get("usage") {
                    Some(u) => u,
                    None => continue,
                };
                let model_id = msg.get("model").and_then(|m| m.as_str()).unwrap_or("");
                let cost = estimate_cost_usd(model_id, usage);
                if cost > 0.0 {
                    records.push((ts, cost));
                }
            }
        }
    }
    records.sort_by_key(|r| r.0);

    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(&RecordsCache {
        records: records.clone(),
        computed_at: now,
    }) {
        let _ = fs::write(&cache_path, s);
    }
    records
}

pub fn window_cost(records: &[(i64, f64)], window_sec: i64, now: i64) -> f64 {
    let cutoff = now - window_sec;
    records.iter().filter(|r| r.0 >= cutoff).map(|r| r.1).sum()
}
