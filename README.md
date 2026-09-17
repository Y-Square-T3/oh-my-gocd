[![CI](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml/badge.svg)](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Crates.io Downloads](https://img.shields.io/crates/d/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Homebrew](https://img.shields.io/github/v/release/Y-Square-T3/oh-my-gocd?label=homebrew&logo=homebrew)](https://github.com/Y-Square-T3/homebrew-oh-my-gocd)

# oh-my-gocd

[English](README.md) | [简体中文](README.zh-CN.md)

## Installation

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

If `omg` is not found, ensure Cargo's binary directory is on your `PATH`: `~/.cargo/bin` on Linux/macOS or `%USERPROFILE%\.cargo\bin` on Windows, then restart your terminal.

### Linux (x86-64)

Download the prebuilt binary from the [latest release](https://github.com/Y-Square-T3/oh-my-gocd/releases/latest). Rust is not required. The Linux archive targets `x86_64-unknown-linux-gnu` and requires a compatible glibc-based distribution; it is not a native ARM64 or Alpine/musl build. Use Cargo to build for other supported targets or if your system's glibc is too old.

Run in a POSIX-compatible shell with `curl` and `tar` installed:

```sh
omg_tmp_dir=$(mktemp -d)
curl -fL https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download/omg-linux-amd64.tar.gz \
  -o "$omg_tmp_dir/omg-linux-amd64.tar.gz" &&
tar -xzf "$omg_tmp_dir/omg-linux-amd64.tar.gz" -C "$omg_tmp_dir" &&
mkdir -p "$HOME/.local/bin" &&
install -m 755 "$omg_tmp_dir/omg" "$HOME/.local/bin/omg"

export PATH="$HOME/.local/bin:$PATH"
omg --version
```

To keep `omg` available in new terminals, add `export PATH="$HOME/.local/bin:$PATH"` to your shell startup file (for example, `~/.bashrc` for Bash or `~/.zshrc` for Zsh). Repeat the download and install steps to upgrade.

### Windows (x64)

Download and extract the prebuilt binary in PowerShell. Rust and C++ Build Tools are not required for this installation method:

```powershell
$omgInstallDir = "$env:LOCALAPPDATA\Programs\omg"
$omgArchive = "$env:TEMP\omg-windows-amd64.zip"
Invoke-WebRequest -Uri "https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download/omg-windows-amd64.zip" -OutFile $omgArchive -ErrorAction Stop
Expand-Archive -Path $omgArchive -DestinationPath $omgInstallDir -Force -ErrorAction Stop

$env:Path = "$omgInstallDir;$env:Path"
omg --version
```

The `PATH` change above applies to the current PowerShell session. To make it permanent, open **Edit environment variables for your account**, edit **Path** under **User variables**, and add `%LOCALAPPDATA%\Programs\omg`. Restart your terminal and any MCP client so they pick up the new path.

To upgrade, stop running `omg` processes (including MCP servers), then repeat the download and extraction steps. For WSL, follow the Linux instructions inside your WSL terminal.

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
