[![CI](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml/badge.svg)](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Crates.io Downloads](https://img.shields.io/crates/d/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Homebrew](https://img.shields.io/github/v/release/Y-Square-T3/oh-my-gocd?label=homebrew&logo=homebrew)](https://github.com/Y-Square-T3/homebrew-oh-my-gocd)

# oh-my-gocd

[English](README.md) | [简体中文](README.zh-CN.md)

## Installation

### Linux (x86-64)

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download/install.sh | sh
```

No Rust toolchain is required. The installer verifies the release checksum, installs `omg` to `~/.local/bin`, and adds that directory to your shell `PATH` when needed. Restart the terminal if it updates your shell profile. The prebuilt binary requires a compatible glibc-based distribution; use the Cargo method below on ARM64, Alpine/musl, or older glibc systems.

### Windows (x64)

Run in PowerShell:

```powershell
irm https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download/install.ps1 | iex
```

No Rust toolchain or Visual Studio C++ Build Tools are required. The installer verifies the release checksum, installs `omg` under `%LOCALAPPDATA%\Programs\omg`, and adds it to your user `PATH`. Restart terminals and MCP clients that were already open. For WSL, use the Linux command inside your WSL terminal.

### Homebrew (macOS)

```sh
brew install Y-Square-T3/homebrew-oh-my-gocd/omg
```

### Cargo (all platforms)

Install the latest stable Rust toolchain using [rustup](https://rust-lang.org/tools/install/) first. On Windows, install the Visual Studio C++ Build Tools when prompted. On Linux, a C compiler and linker are required (for example, `build-essential` on Debian/Ubuntu).

```sh
cargo install oh-my-gocd
omg --version
```

If `omg` is not found, ensure Cargo's binary directory is on your `PATH`: `~/.cargo/bin` on Linux/macOS or `%USERPROFILE%\.cargo\bin` on Windows, then restart your terminal. You can also download archives and inspect the installer scripts on the [latest release](https://github.com/Y-Square-T3/oh-my-gocd/releases/latest) page before running them. Re-run your chosen installation command to upgrade.

## Configuration

Settings live in `~/.config/omg/omg.jsonc` (`$XDG_CONFIG_HOME/omg/omg.jsonc` if set):

```sh
omg config --token 'gocd-bearer-token' --endpoint 'https://gocd.example.com/go'

# Keep secrets out of shell history — '-' reads one line from stdin
security find-generic-password -w gocd-token | omg config -T -

# What the MCP server may do with the token (see "Security modes" below)
omg config --mode operate

omg config --list
```

## MCP server

omg can serve the [Model Context Protocol](https://modelcontextprotocol.io) over stdio:

```sh
omg server --mcp
```

It authenticates with the saved `server.endpoint` and `server.token`, and fails at startup if either is missing. It speaks JSON-RPC on stdin/stdout, so it is meant to be spawned by an MCP client rather than run by hand.

### Security modes

Every tool belongs to one of three tiers and the saved `mcp.mode` decides how far up the ladder an MCP session reaches. Each mode includes the tiers below it:

| Mode | What it exposes |
| --- | --- |
| `view` (default) | Reads only — the pure GET tools, including the admin reads. |
| `operate` | Plus routine writes: scheduling and pausing pipelines, running and canceling stages and jobs, comments, material notifications, backup scheduling, artifact-store and config updates, agent updates. |
| `full` | Plus the danger tier: every delete and bulk delete, token revokes, user and authorization-config writes, maintenance mode, killing agent tasks. |

```sh
omg config --mode view      # agent may read, nothing else (this is also the default)
omg config --mode operate   # day-to-day pipeline driving
omg config --mode full      # nothing is withheld — deletes included
```

- Unset means `view` — omg is safe out of the box. **Upgraders beware**: earlier releases exposed all tools, so a workflow that writes to GoCD needs an explicit `omg config --mode operate` (or `full`).
- The mode is read at server startup — restart the MCP client session to apply a change. `omg config --list` shows the saved value.

How tools are tiered, how hidden tools are enforced, and why: [ADR 0001](docs/adr/0001-mcp-security-mode-ladder.md). The full list of tools is in [docs/mcp-tools.md](docs/mcp-tools.md).

### opencode

Add to your project's `opencode.json` (or `~/.config/opencode/opencode.json` to make it global):

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "omg": {
      "type": "local",
      "command": ["omg", "server", "--mcp"],
      "enabled": true
    }
  }
}
```

Restart opencode to load it.
