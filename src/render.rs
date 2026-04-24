use chrono::{Local, TimeZone};
use std::fmt::Write;

use crate::git;
use crate::input::Input;
use crate::transcript::scan_session;
use crate::util::{env_bool, fmt_duration, fmt_tokens, home_dir, now_secs};

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const FG_BLACK: &str = "\x1b[38;2;38;43;68m";
const FG_WHITE: &str = "\x1b[38;2;224;222;246m";
const FG_YELLOW: &str = "\x1b[38;2;243;174;53m";
const FG_ORANGE: &str = "\x1b[38;2;240;118;35m";
const FG_GREEN: &str = "\x1b[38;2;89;201;165m";
const FG_RED: &str = "\x1b[38;2;216;30;91m";
const FG_BLUE: &str = "\x1b[38;2;75;149;233m";
const BG_YELLOW: &str = "\x1b[48;2;243;174;53m";
const BG_ORANGE: &str = "\x1b[48;2;240;118;35m";
const BG_GREEN: &str = "\x1b[48;2;89;201;165m";

const PL_LD: &str = "\u{e0b6}";
const PL_R: &str = "\u{e0b0}";

pub fn render(input: &Input) -> String {
    let ascii = env_bool("CCPULSE_ASCII");

    let cwd = input
        .workspace
        .as_ref()
        .and_then(|w| w.current_dir.as_deref())
        .or(input.cwd.as_deref())
        .map(String::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".into())
        });

    let model_name = input
        .model
        .as_ref()
        .and_then(|m| m.display_name.as_deref())
        .or_else(|| input.model.as_ref().and_then(|m| m.id.as_deref()))
        .unwrap_or("Claude");
    let style = input
        .output_style
        .as_ref()
        .and_then(|o| o.name.as_deref())
        .unwrap_or("");
    let effort_level = input.effort.as_ref().and_then(|e| e.level.as_deref());
    let fast = input.fast_mode.unwrap_or(false);

    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let home = home_dir();
    let home_str = home.to_string_lossy().to_string();
    let short_cwd = if cwd.starts_with(&home_str) {
        format!("~{}", &cwd[home_str.len()..])
    } else {
        cwd.clone()
    };
    let (branch, dirty) = git::branch_status(&cwd);

    let mut out = String::with_capacity(1024);

    // Line 1: identity / location / model / effort / thinking / fast
    if ascii {
        let _ = write!(out, "[{}] {}", user, short_cwd);
        if let Some(b) = branch.as_ref() {
            let _ = write!(out, " ({}{})", b, if dirty { " *" } else { "" });
        }
    } else {
        let _ = write!(
            out,
            "{}{}{}{} {} {}{}{}{}{}",
            FG_YELLOW, PL_LD, BG_YELLOW, FG_BLACK, user, RESET, FG_YELLOW, BG_ORANGE, PL_R, RESET
        );
        let _ = write!(out, "{}{} {} {}", BG_ORANGE, FG_WHITE, short_cwd, RESET);
        if let Some(b) = branch.as_ref() {
            let mark = if dirty { " *" } else { "" };
            let _ = write!(
                out,
                "{}{}{}{}{}{} {}{} {}{}{}{}",
                FG_ORANGE,
                BG_GREEN,
                PL_R,
                RESET,
                BG_GREEN,
                FG_BLACK,
                b,
                mark,
                RESET,
                FG_GREEN,
                PL_R,
                RESET
            );
        } else {
            let _ = write!(out, "{}{}{}", FG_ORANGE, PL_R, RESET);
        }
    }
    let _ = write!(out, "  {}{}{}{}", FG_BLUE, BOLD, model_name, RESET);
    if let Some(e) = effort_level {
        let _ = write!(out, " {}effort:{}{}{}{}", DIM, RESET, FG_YELLOW, e, RESET);
    }
    if fast {
        let _ = write!(out, " {}fast{}", DIM, RESET);
    }
    if !style.is_empty() && style != "default" {
        let _ = write!(out, " {}[{}]{}", DIM, style, RESET);
    }
    let v = crate::version::status();
    let _ = write!(out, " {}ccpulse: v{}{}", DIM, v.current, RESET);
    if v.update_available {
        if let Some(latest) = v.latest.as_deref() {
            let tag = latest.trim_start_matches('v');
            let _ = write!(out, " {}\u{2192} v{}{}", FG_YELLOW, tag, RESET);
        }
    }
    out.push('\n');

    // Line 2: context window + session totals + cost
    let cw = input.context_window.as_ref();
    let ctx_size = cw.and_then(|c| c.context_window_size).unwrap_or(0);
    let ctx_used = cw
        .and_then(|c| c.current_usage.as_ref())
        .map(|u| {
            u.input_tokens.unwrap_or(0)
                + u.cache_creation_input_tokens.unwrap_or(0)
                + u.cache_read_input_tokens.unwrap_or(0)
        })
        .unwrap_or(0);
    let ctx_pct = cw.and_then(|c| c.used_percentage).unwrap_or_else(|| {
        if ctx_size > 0 {
            ctx_used as f64 / ctx_size as f64 * 100.0
        } else {
            0.0
        }
    });
    let cc = color_pct(ctx_pct);

    let _ = write!(
        out,
        "{}ctx{} {}{}{}{}/{}{} {}{}{} {}{:5.1}%{}",
        DIM,
        RESET,
        cc,
        fmt_tokens(ctx_used),
        RESET,
        DIM,
        fmt_tokens(ctx_size),
        RESET,
        cc,
        bar(ctx_pct, 10),
        RESET,
        cc,
        ctx_pct,
        RESET,
    );

    // Session in/out: prefer stdin's context_window.total_*, fall back to transcript scan.
    let stdin_total_in = cw.and_then(|c| c.total_input_tokens);
    let stdin_total_out = cw.and_then(|c| c.total_output_tokens);
    let need_cache_totals = !env_bool("CCPULSE_NO_TRANSCRIPT");
    let scan = if need_cache_totals {
        scan_session(input.transcript_path.as_deref())
    } else {
        Default::default()
    };
    let sess_in = stdin_total_in.unwrap_or(scan.input);
    let sess_out = stdin_total_out.unwrap_or(scan.output);
    let sess_cache = scan.cache_create + scan.cache_read;

    let _ = write!(
        out,
        " {}|{} {}in{} {}{}{} {}out{} {}{}{}",
        DIM,
        RESET,
        DIM,
        RESET,
        FG_GREEN,
        fmt_tokens(sess_in),
        RESET,
        DIM,
        RESET,
        FG_ORANGE,
        fmt_tokens(sess_out),
        RESET,
    );
    if sess_cache > 0 {
        let _ = write!(
            out,
            " {}cache{} {}{}{}",
            DIM,
            RESET,
            FG_BLUE,
            fmt_tokens(sess_cache),
            RESET,
        );
    }
    if let Some(cost) = input.cost.as_ref().and_then(|c| c.total_cost_usd) {
        let _ = write!(
            out,
            " {}|{} {}\u{0024}{:.2}{}",
            DIM, RESET, BOLD, cost, RESET,
        );
    }
    if let Some(c) = input.cost.as_ref() {
        let added = c.total_lines_added.unwrap_or(0);
        let removed = c.total_lines_removed.unwrap_or(0);
        if added + removed > 0 {
            let _ = write!(
                out,
                " {}({}+{}{}{}-{}{}){}",
                DIM, FG_GREEN, added, RESET, FG_RED, removed, DIM, RESET,
            );
        }
    }
    out.push('\n');

    // Line 3: 5h + 7d (only if stdin gave us rate_limits)
    let now = now_secs();
    let rl = input.rate_limits.as_ref();
    let five = rl.and_then(|r| r.five_hour.as_ref());
    let seven = rl.and_then(|r| r.seven_day.as_ref());
    if five.is_some() || seven.is_some() {
        let mut parts: Vec<String> = Vec::new();
        if let Some(w) = five {
            parts.push(fmt_window("5h", w, now));
        }
        if let Some(w) = seven {
            parts.push(fmt_window("7d", w, now));
        }
        let sep = format!("  {}|{}  ", DIM, RESET);
        out.push_str(&parts.join(&sep));
    } else {
        let _ = write!(
            out,
            "{}rate_limits not in stdin (need Claude Code 2.1+){}",
            DIM, RESET
        );
    }

    out
}

fn fmt_window(label: &str, w: &crate::input::RateLimitWindow, now: i64) -> String {
    let pct = w.used_percentage.unwrap_or(0.0);
    let c = color_pct(pct);
    let mut s = format!(
        "{}{}{} {}{}{} {}{:5.1}%{}",
        DIM,
        label,
        RESET,
        c,
        bar(pct, 10),
        RESET,
        c,
        pct,
        RESET
    );
    if let Some(reset_ts) = w.resets_at {
        let remaining = reset_ts - now;
        if remaining > 0 {
            if let Some(d) = Local.timestamp_opt(reset_ts, 0).single() {
                let _ = write!(
                    s,
                    " {}reset{} {} {}({}){}",
                    DIM,
                    RESET,
                    d.format("%m-%d %H:%M"),
                    DIM,
                    fmt_duration(remaining),
                    RESET,
                );
            }
        }
    }
    s
}

fn color_pct(pct: f64) -> &'static str {
    if pct >= 90.0 {
        FG_RED
    } else if pct >= 70.0 {
        FG_YELLOW
    } else {
        FG_GREEN
    }
}

fn bar(pct: f64, width: usize) -> String {
    let pct = pct.clamp(0.0, 100.0);
    let filled = (pct * width as f64 / 100.0).round() as usize;
    let mut s = String::with_capacity(width * 4);
    for _ in 0..filled {
        s.push('\u{2588}');
    }
    for _ in filled..width {
        s.push('\u{2591}');
    }
    s
}
