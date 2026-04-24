//! Version display and update check.
//!
//! - Current version comes from `CARGO_PKG_VERSION` at compile time.
//! - Latest release tag is fetched in a detached background process so the
//!   statusline never blocks on the network. The result is cached to
//!   `~/.claude/cache/ccpulse-version.json` and reused for 6 hours.
//! - The check uses `curl`, which ships with macOS and modern Linux/Windows.

use serde::{Deserialize, Serialize};
use std::fs;
use std::process::{Command, Stdio};

use crate::util::{home_dir, now_secs};

pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
const CACHE_REL_PATH: &str = ".claude/cache/ccpulse-version.json";
const CHECK_TTL_SEC: i64 = 6 * 3600;
const REPO_API: &str = "https://api.github.com/repos/asm2apex/ccpulse/releases/latest";

#[derive(Serialize, Deserialize, Default)]
struct Cache {
    latest: String,
    checked_at: i64,
}

pub struct Status {
    pub current: &'static str,
    pub latest: Option<String>,
    pub update_available: bool,
}

pub fn status() -> Status {
    let cache_path = home_dir().join(CACHE_REL_PATH);
    let now = now_secs();
    let mut latest = None;
    let mut needs_check = true;

    if let Ok(s) = fs::read_to_string(&cache_path) {
        if let Ok(c) = serde_json::from_str::<Cache>(&s) {
            if !c.latest.is_empty() {
                latest = Some(c.latest.clone());
            }
            if c.checked_at + CHECK_TTL_SEC > now {
                needs_check = false;
            }
        }
    }

    if needs_check {
        spawn_check();
    }

    let update_available = latest
        .as_deref()
        .map(|l| version_newer(l, CURRENT))
        .unwrap_or(false);

    Status {
        current: CURRENT,
        latest,
        update_available,
    }
}

fn version_newer(remote: &str, local: &str) -> bool {
    let r = parse_semver(remote);
    let l = parse_semver(local);
    r > l
}

fn parse_semver(s: &str) -> Vec<u32> {
    s.trim_start_matches('v')
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse().ok())
        .collect()
}

#[cfg(unix)]
fn spawn_check() {
    let cache = home_dir().join(CACHE_REL_PATH);
    let cache_dir = cache.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let cache_str = cache.to_string_lossy().to_string();
    let cache_dir_str = cache_dir.to_string_lossy().to_string();
    let script = format!(
        r#"(
            mkdir -p "{cache_dir}" 2>/dev/null
            resp=$(curl -fsS --max-time 5 \
                -H 'Accept: application/vnd.github+json' \
                -H 'User-Agent: ccpulse' \
                '{api}' 2>/dev/null) || exit 0
            tag=$(printf '%s' "$resp" | sed -n 's/.*"tag_name":[ ]*"\([^"]*\)".*/\1/p' | head -n1)
            [ -z "$tag" ] && exit 0
            printf '{{"latest":"%s","checked_at":%d}}' "$tag" "$(date +%s)" > "{cache}.tmp" \
                && mv "{cache}.tmp" "{cache}"
        ) </dev/null >/dev/null 2>&1 &"#,
        cache_dir = cache_dir_str,
        cache = cache_str,
        api = REPO_API,
    );
    let _ = Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(not(unix))]
fn spawn_check() {
    // Background check is Unix-only for now. Windows users can run
    // `ccpulse --check-version` manually if a CLI hook is added later.
}
