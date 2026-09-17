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

The MCP server is a guardrail for whoever — usually an AI agent — holds the session, so each tool belongs to one of three tiers and the saved `mcp.mode` decides how far up the ladder a session reaches. Every mode includes the tiers below it:

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

- Unset `mcp.mode` means `view`: omg is safe out of the box. **Upgraders beware** — every release before this one exposed all tools; if an existing agent workflow writes to GoCD, it needs an explicit `omg config --mode operate` (or `full`) after upgrading.
- A mode change is read at server startup — restart the MCP client session to apply it. `omg config --list` shows the saved mode, or `mode: <unset> (default: view)`.
- Hidden tools do not appear in the session's tool list at all, and a call that sneaks through (e.g. a client with a stale cached list) is rejected with an explanation naming the mode and the way out.
- Only the canonical values `view`, `operate`, `full` are accepted, from the CLI and in the file; anything else fails loudly rather than guessing.
- The per-tool tier table lives in `src/server/security.rs`; a new tool without a tier is treated as danger, and the test suite refuses to ship one.

The full list of tools is in [docs/mcp-tools.md](docs/mcp-tools.md).

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
