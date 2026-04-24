//! Transcript scanner for the active session.
//!
//! Used as a fallback when stdin doesn't carry the field we need (older
//! Claude Code versions, or when we want session-level cache totals that
//! the stdin schema doesn't expose).

use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

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
