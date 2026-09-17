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
- `update_current_user` — update the token's own user from a JSON body (`email`/`email_me`/`checkin_aliases`; no `etag` needed — GoCD requires no If-Match here).

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

**Backups**

- `schedule_backup` — trigger an asynchronous backup of all configuration and the database.
- `get_backup` — read one backup's status by id, or with the `backup_id` keyword `running`.

**Dashboard**

- `get_dashboard` — the personalized dashboard: pipelines with their latest instances and stage status, pipeline groups and environments.

**Encryption**

- `encrypt_value` — get the cipher text for a plain text value from a JSON body (`{"value": ...}`); GoCD rate-limits this to 30 requests per minute.

**Jobs**

- `get_job_instance` — read one job instance (state, result, agent, state transitions) by pipeline/counter/stage/counter/job.
- `get_job_history` — list a job's past instances, with optional `page_size`/`after`/`before` cursor pagination.

**Pipeline Instances**

- `get_pipeline_instance` — read one pipeline instance (build cause, stages and jobs) by pipeline name and counter.
- `get_pipeline_history` — list a pipeline's past instances, with optional `page_size`/`after`/`before` cursor pagination.
- `comment_pipeline_instance` — attach a comment (e.g. a failure reason) to a pipeline instance from a JSON body (`{"comment": ...}`).

**Pipelines**

- `get_pipeline_status` — read whether a pipeline is paused, locked and schedulable.
- `pause_pipeline` — pause a pipeline from a JSON body (`{"pause_cause": ...}`; sent body-less with GoCD's documented `X-GoCD-Confirm` header when no body is given).
- `unpause_pipeline` — unpause a pipeline (body-less POST, confirmed per the docs).
- `unlock_pipeline` — release a pipeline lock (body-less POST, confirmed per the docs; only while locked with no running instance).
- `schedule_pipeline` — trigger a new instance from an optional JSON body (`environment_variables`/`materials`/`update_materials_before_scheduling`).
- `compare_pipeline_instances` — the material changes between two pipeline instances.

**Server Health**

- `check_server_health` — whether the GoCD server is up and running (the health object, e.g. `{"health": "OK"}`).

**Server Health Messages**

- `get_server_health_messages` — the current server errors and warnings (message, detail, level, time), the same set the web UI shows in its errors-and-warnings modal.

**Stage Instances**

- `get_stage_instance` — read one stage instance (result, approval and its jobs with state transitions) by pipeline/counter/stage/counter.
- `get_stage_history` — list a stage's past instances, with optional `page_size`/`after`/`before` cursor pagination.
- `cancel_stage_instance` — cancel an active stage instance (body-less POST, confirmed per the docs).
- `run_failed_stage_jobs` — rerun the failed jobs of a completed stage instance (body-less POST, confirmed per the docs).
- `run_selected_stage_jobs` — rerun the named jobs of a completed stage instance from a JSON body (`{"jobs": [...]}`).

**Stages**

- `run_stage` — trigger a stage against an existing pipeline instance (body-less POST, confirmed per the docs; GoCD answers 202 with an acceptance message).

**Users**

- `get_users` — list all users.
- `get_user` — read one user by login name (returns `_etag` when GoCD sends one).
- `create_user` — create a user from a JSON body (`login_name` required; no If-Match needed — GoCD requires none here).
- `update_user` — update one user's attributes (`enabled`/`email`/`email_me`/`checkin_aliases`) from a JSON body (no `etag` needed — GoCD requires no If-Match here).
- `delete_user` — delete one user by login name (disable the user first; no `etag` needed — GoCD requires no If-Match here).
- `bulk_delete_users` — bulk delete users from a JSON body (`users`).
- `bulk_enable_disable_users` — enable or disable users from a JSON body (`users` + `operations.enable`).

**Version**

- `get_version` — the GoCD server version details (version, build number, git SHA, full version, commit URL).

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
