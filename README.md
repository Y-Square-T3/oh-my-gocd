[![CI](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml/badge.svg)](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Crates.io Downloads](https://img.shields.io/crates/d/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Homebrew](https://img.shields.io/github/v/release/Y-Square-T3/oh-my-gocd?label=homebrew&logo=homebrew)](https://github.com/Y-Square-T3/homebrew-oh-my-gocd)

# oh-my-gocd

## Installation

### Homebrew (macOS)

```sh
brew install Y-Square-T3/homebrew-oh-my-gocd/omg
```

### Cargo (all platforms)

```sh
cargo install oh-my-gocd
```

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
