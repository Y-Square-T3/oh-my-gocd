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

omg config --list
```

## MCP server

omg can serve the [Model Context Protocol](https://modelcontextprotocol.io) over stdio:

```sh
omg server --mcp
```

It authenticates with the saved `server.endpoint` and `server.token`, and fails at startup if either is missing. It speaks JSON-RPC on stdin/stdout, so it is meant to be spawned by an MCP client rather than run by hand.

Tools currently offered:

**Current User**

- `get_current_user` — the GoCD user the token authenticates as, as JSON.

**Artifact Store**

- `get_artifact_stores` — list every configured pluggable artifact store.
- `get_artifact_store` — read one artifact store by id (returns its `_etag`).
- `create_artifact_store` — create an artifact store from a JSON body.
- `update_artifact_store` — replace an artifact store, guarded by a required `etag` (the `_etag` you read).
- `delete_artifact_store` — delete an artifact store by id.

**Artifacts Config**

- `get_artifacts_config` — read the server-wide artifacts dir and purge settings (returns its `_etag`).
- `update_artifacts_config` — update the artifacts config, guarded by a required `etag` (the `_etag` you read).

**Authorization Configuration**

- `get_auth_configs` — list every configured authorization configuration.
- `get_auth_config` — read one authorization configuration by id (returns its `_etag`).
- `create_auth_config` — create an authorization configuration from a JSON body.
- `update_auth_config` — replace an authorization configuration, guarded by a required `etag` (the `_etag` you read).
- `delete_auth_config` — delete an authorization configuration by id.

**Agents**

- `get_agents` — list all agents, registered and pending.
- `get_agent` — read one agent by uuid.
- `update_agent` — update one agent's attributes (hostname/resources/environments/config state) from a JSON body.
- `delete_agent` — delete one agent by uuid.
- `bulk_update_agents` — bulk update agents from a JSON body (`uuids` + `operations`).
- `bulk_delete_agents` — bulk delete agents from a JSON body (`uuids`).
- `get_agent_job_run_history` — list jobs that have run on an agent, with optional `offset`/`page_size`/`sort_column`/`sort_order`.
- `kill_agent_running_tasks` — kill all running tasks on an agent.

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
