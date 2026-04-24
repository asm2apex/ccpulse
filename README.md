# ccpulse

[English](README.md) · [中文](README.zh.md)

A statusline for Claude Code that shows the things I actually want to know
while I'm working: which model is answering, how much context I've burned,
how many tokens this session has cost, and where I sit in the rolling 5h /
7d quota windows.

It is a single Rust binary. Reads the JSON Claude Code pipes in on stdin,
optionally cross-checks it against the active session's transcript, and
prints three lines.

## What it shows

```
 user   ~/path/to/project   main *   Opus 4.7 (1M context)  effort:xhigh  thinking
ctx 207.2K/1.00M ██░░░░░░░░  21.0% | in 139  out 119.3K  cache 16.66M  | $8.83 (+1831 -447)
5h █░░░░░░░░░   6.0% reset 04-24 15:20 (3h16m)  |  7d ░░░░░░░░░░   1.0% reset 04-26 02:00 (1d13h)
```

Line by line:

- **Line 1** — user, current directory, git branch (`*` when dirty), model
  display name. If the session has an effort level, thinking, or fast mode
  set, those follow.
- **Line 2** — context window usage with a small bar and percentage, then
  the session's cumulative input / output / cache tokens, the running cost
  in USD, and lines added/removed (when the session has touched files).
- **Line 3** — current 5-hour and 7-day usage as a percentage of your
  Anthropic quota, and the timestamp each window resets at. Both values
  come from Claude Code itself — there's nothing to configure.

The version printed at the end of line 1 is the running binary's version.
Once every 6 hours ccpulse spawns a detached `curl` to check the latest
release on GitHub; if a newer tag exists, an arrow and the new version
appear next to it (`v0.1.0 → v0.2.0`). The check never blocks the render.

## Install

### Binary release

Download from [Releases](https://github.com/asm2apex/ccpulse/releases),
extract, drop the binary somewhere convenient (e.g. `~/.claude/`):

```bash
# macOS Apple Silicon
curl -L https://github.com/asm2apex/ccpulse/releases/latest/download/ccpulse-macos-arm64.tar.gz | tar -xz
mkdir -p ~/.claude/ccpulse && mv ccpulse ~/.claude/ccpulse/
```

Replace the asset name to match your platform — Linux x64/arm64, macOS
x64/arm64, and Windows x64 are all attached to each release.

### From source

```bash
cargo install --git https://github.com/asm2apex/ccpulse
```

or clone and `cargo build --release`. The binary lands at
`target/release/ccpulse`.

## Configure

Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/ccpulse/ccpulse",
    "padding": 0
  }
}
```

Adjust the path if you put the binary somewhere else. Claude Code expands
`~` itself.

That's it. There are no required env vars.

## Optional environment variables

| Variable | Effect |
| --- | --- |
| `CCPULSE_ASCII` | Set to `1` to skip the powerline glyphs for terminals without a Nerd Font. |
| `CCPULSE_NO_TRANSCRIPT` | Set to `1` to skip the transcript scan. The cache token counter on line 2 will be hidden as a result; everything else still renders. |

## How it works

Every render, ccpulse:

1. Reads the JSON Claude Code sends on stdin. Recent versions (2.1.x)
   already include the rate-limit windows, the context window size and
   percentage, the per-turn token breakdown, the running cost, and the
   effort / thinking / fast-mode flags. ccpulse uses these directly.
2. Streams the active session's transcript JSONL once to recover the
   cumulative cache_creation / cache_read totals — these aren't in the
   stdin payload but are useful to see at a glance. Skip the scan with
   `CCPULSE_NO_TRANSCRIPT=1`.
3. Shells out to `git` for branch and dirty state.
4. Prints three ANSI-coded lines.

Total render is around 25 ms. There's no cache file and no cross-project
scan; everything that needs scanning is one file.

## Compatibility

- **Claude Code 2.1+ recommended.** That's the version where stdin
  started carrying `rate_limits`, `context_window`, `effort`, and `cost`.
  On older versions, line 3 falls back to a "rate_limits not in stdin"
  message and the line 2 percentages may show 0.
- **Anthropic OAuth login required for rate limits.** API-key users
  don't have the same quota model, so `rate_limits` won't be present.

## Build

```bash
cargo build --release
```

Requires Rust 1.74 or newer.

## License

MIT.
