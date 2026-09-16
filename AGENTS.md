## Agent skills

### Issue tracker

Issues tracked as GitHub issues via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Five canonical labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout with `CONTEXT.md` at root and `docs/adr/` for decisions. See `docs/agents/domain.md`.

### Test strategy

**Discipline**: TDD. Write the failing test first, then make it pass.

**Organization**: Inline `#[cfg(test)] mod tests` in each module file.

**Naming**: Descriptive snake_case — e.g. `agent_stops_after_max_turns()`.

**Async**: `#[tokio::test]` for async code.

**Mocking**: Trait-based DI with manual fakes. Define traits at I/O boundaries (LLM provider, tool executor), inject into domain logic, swap fakes in tests. No `mockall`.

**Test data**: Inline builders. Helper functions to construct mock responses, tool calls, streams. JSON fixtures only if inline becomes unwieldy.
