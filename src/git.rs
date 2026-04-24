use std::process::{Command, Stdio};

pub fn branch_status(cwd: &str) -> (Option<String>, bool) {
    let branch = run_git(cwd, &["symbolic-ref", "--short", "HEAD"])
        .or_else(|| run_git(cwd, &["rev-parse", "--short", "HEAD"]));
    let dirty = branch.is_some()
        && run_git(cwd, &["status", "--porcelain"])
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
    (branch, dirty)
}

fn run_git(cwd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
