# 0001 — MCP security mode ladder

Date: 2026-09-17
Status: accepted

## Context

omg's MCP server is spawned by AI-agent clients using a GoCD token that is routinely admin-level. Nothing in the protocol stops a hallucinating or prompt-injected agent from calling `delete_user` or revoking tokens. We decided (design interview, issues #spec) that omg itself must cap what a session can do, and that the cap must be explicit, visible to the user, and hard to misconfigure.

## Decision

- Three tool **tiers** — `view`, `operate`, `danger` — assigned per tool in one central table (`src/server/security.rs`), derived from the HTTP verb of the underlying GoCD call with curated overrides for the edge cases (maintenance mode, security writes and bulk user operations are danger even though they are not deletes; all reads, including sensitive admin reads, are view).
- Three **modes** — `view` ⊂ `operate` ⊂ `full` — a cumulative ladder stored in the config file as `mcp.mode` and set with `omg config --mode <view|operate|full>`. The config file is the only control point: no environment variable and no server flag. Values are strictly canonical; no aliases.
- **Unset means `view`.** Safety by default beats backward compatibility: the upgrade note in the README tells existing write-workflow users to opt in explicitly.
- Enforcement is twofold: tools above the mode are removed from the `ToolRouter` at construction (absent from `tools/list`), and `call_tool` rejects any hidden-by-mode name before dispatch with an explanatory error (tool, current mode, tier, `omg config --mode …`, restart). An unclassified tool is treated as `danger` whenever anything can be hidden, so a forgotten table entry fails closed.
- A restrictive mode announces itself in the MCP `initialize` server instructions, so the agent can tell the user why a write is unavailable; `full` sends no instructions.
- The mode is read at server startup only; changing it takes effect on the next client session. No file watching.

## Consequences

- A default omg session can no longer change anything in GoCD, so first-time agent write workflows stop until the user opts in — accepted trade-off, documented in the README upgrade note.
- The tier table is a single review point: any new tool must be classified there, and the exhaustiveness tests fail CI both ways (uncategorized registered tools and unknown table entries).
- Tiers are about side effects, not data sensitivity: an admin read of LDAP config is `view`. If information exposure ever becomes the concern, it needs its own mechanism, not a tier promotion.
- Hiding tools shrinks the agent's perceived world, which is also the point: fewer tools, fewer temptations, less context noise. The blocked-call message covers clients that cache stale tool lists.
