# ccpulse

[English](README.md) · [中文](README.zh.md)

写给 Claude Code 的 statusline。我自己用的时候关心几件事:当前哪个模型在
回答、上下文吃了多少、这个会话总共烧了多少 token、过去 5 小时 / 7 天的
配额还剩多少。这个工具就是把这些信息打到状态栏里。

实现是单个 Rust 二进制。读 Claude Code 通过 stdin 给的 JSON,顺手扫一下
当前会话的 transcript,然后打三行字。

## 长这样

```
 user   ~/path/to/project   main *   Opus 4.7 (1M context)  effort:xhigh  ccpulse: v0.1.2
ctx 207.2K/1.00M ██░░░░░░░░  21.0% | in 139  out 119.3K  cache 16.66M  | $8.83
5h █░░░░░░░░░   6.0% reset 04-24 15:20 (3h16m)  |  7d ░░░░░░░░░░   1.0% reset 04-26 02:00 (1d13h)
```

逐行说:

- **第一行**:用户名、当前目录、git 分支(脏工作区会带 `*`)、模型显示
  名。会话设了 effort 或 fast 模式的话会接在后面。行末是当前二进制版本
  号(详见下面的更新检测)。
- **第二行**:上下文窗口占用(带小进度条和百分比),后面是会话累积的
  输入 / 输出 / 缓存 token,以及 Claude Code 报的当前会话花费(美元)。
- **第三行**:当前 5 小时 / 7 天配额已用百分比,以及窗口重置时间。两组
  数都是 Claude Code 自己算好交给我们的,无需任何配置。

百分比按阈值上色:小于 60% 绿、60% 到 80% 黄、80% 及以上红。

第一行末尾的版本号是当前运行的二进制版本。每 6 小时 ccpulse 会异步起一个
`curl` 查 GitHub 上最新 release,如果有更新就在版本号后面加黄色箭头和新
版本号(`ccpulse: v0.1.0 → v0.1.2`)。检查全程不阻塞渲染。

## 安装

### 二进制 release

到 [Releases](https://github.com/asm2apex/ccpulse/releases) 下载对应平台
的包,解压后扔到顺手的位置(比如 `~/.claude/`):

```bash
# macOS Apple Silicon
curl -L https://github.com/asm2apex/ccpulse/releases/latest/download/ccpulse-macos-arm64.tar.gz | tar -xz
mkdir -p ~/.claude/ccpulse && mv ccpulse ~/.claude/ccpulse/
```

文件名换成对应平台即可。每个 release 都附 Linux x64/arm64、macOS
x64/arm64、Windows x64 的构建。

### 从源码

```bash
cargo install --git https://github.com/asm2apex/ccpulse
```

或者 clone 下来 `cargo build --release`,产物在 `target/release/ccpulse`。

## 配置

编辑 `~/.claude/settings.json`,加上:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/ccpulse/ccpulse",
    "padding": 0
  }
}
```

二进制放别处就改路径。`~` 由 Claude Code 自己展开。

就这一步。没有任何必填环境变量。

## 可选环境变量

| 变量名 | 作用 |
| --- | --- |
| `CCPULSE_ASCII` | 设成 `1` 关闭 powerline 字符,适合没装 Nerd Font 的终端。 |
| `CCPULSE_NO_TRANSCRIPT` | 设成 `1` 跳过 transcript 扫描。第二行的 cache token 数会随之隐藏,其它一切照常。 |

## 工作原理

每次渲染:

1. 读 Claude Code 从 stdin 发的 JSON。新版本(2.1.x)已经把 rate-limit
   窗口、上下文窗口大小和百分比、本轮 token 拆分、累计花费、effort /
   fast 模式标志都直接送过来了,我们就直接拿来用。
2. 把当前会话的 transcript JSONL 流式扫一遍,补上累计 cache_creation /
   cache_read 总数 — 这部分 stdin 里没有,但显示在状态栏里挺有用。
   `CCPULSE_NO_TRANSCRIPT=1` 可以跳过。
3. shell 出去调 `git` 拿分支和脏状态。
4. 输出三行带 ANSI 色码的字符串。

整个渲染大概 25 ms。没有缓存文件,没有跨项目扫描,要扫的就只有一个文件。

## 兼容性

- **建议 Claude Code 2.1+。** 从这个版本开始 stdin 才带
  `rate_limits` / `context_window` / `effort` / `cost`。老版本第三行会
  退化成"rate_limits not in stdin"提示,第二行的百分比可能显示 0。
- **Anthropic OAuth 登录才有配额数据。** 用 API key 直连的不走同一套
  配额模型,stdin 里不会有 `rate_limits`。

## 构建

```bash
cargo build --release
```

需要 Rust 1.85 及以上(使用 2024 edition)。

## 许可

MIT.
