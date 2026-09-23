# 0002 — Derived convenience tools

Date: 2026-09-23
Status: accepted

## Context

The section tools mirror GoCD's documented endpoints one-to-one, so answering
a routine question — "what did this job print?", "is this pipeline doing
anything right now?" — makes an agent walk several calls: list artifacts,
pick a file out of the tree, fetch it, and separately ask the pipeline's
operation state. The walk is where agents waste turns and go wrong. We
agreed (design interview) to add a small family of **derived tools** that
compose the same calls internally, and to make agents reach for those first.

## Decision

- Three derived tools in v1, in their own section module `src/server/mcp/derived.rs`:
  `get_job_console_log` (fetches the fixed `cruise-output/console.log`),
  `get_job_code_review` (fetches the fixed
  `code-review/code_review/reports/index.html`, the path this server's
  review pipelines publish), and `get_pipeline_latest_status` (pipeline
  history's first entry merged flat with the pipeline status object).
- Derivation composes over the `GocdApi` seam with the existing `crate::gocd`
  request builders — never by calling a section tool and re-parsing its
  formatted answer. The recording fake asserts the exact call chain, so a
  derived tool is testable like any other.
- Job-level tools take **all five coordinates explicitly**; nothing is
  auto-resolved. `get_pipeline_latest_status` exists precisely to hand the
  agent those coordinates (its stages[]/jobs[] carry names and counters) in
  one cheap call.
- The file tools answer the fetched bytes as **pure text** — no `[artifact]`
  metadata line — and a GoCD 404 for the fixed path is a meaningful answer,
  not a failure: a one-line `[no <thing>] ...` notice with `is_error` unset.
  Every other transport failure stays the shared actionable tool error.
- `get_pipeline_latest_status` strips `_links` like every tool, carries **no
  `_etag`** (two sources whose etags would collide), and answers a tool
  error for a pipeline that has never run.
- All derived tools are pure reads: `view` tier.
- **Discovery is by description, both directions**: each derived tool
  advertises what it derives from, and the low-level tools it competes with
  (`get_job_artifacts`, `get_job_artifact_file`, `get_pipeline_history`)
  gained "prefer <derived tool>" hints. No tool is hidden to force the
  preference — the raw escape hatch to arbitrary artifacts stays open.
- No new dependencies: the family fetches whole files verbatim, so the
  tempting-but-unbuilt `get_job_test_results` XML summarizer (and its XML
  parser) was dropped from v1 — raw test XML stays reachable through
  `get_job_artifact_file`.

## Consequences

- Agents can answer the three common questions in one call; the raw section
  tools remain for everything the fixed paths do not cover.
- The code-review path is an operational convention of this GoCD server, not
  a GoCD concept: if the pipelines move the report, one constant in
  derived.rs moves with them (and the notice makes a wrong guess loud).
- Derived tools duplicate nothing at the HTTP layer, so GoCD API changes
  surface in the same `crate::gocd` builders as before.
- Future derived tools land in derived.rs and must: compose over GocdApi,
  classify in the tier table, carry the two-way description hints, and add
  their entry to `docs/mcp-tools.md`'s Derived Conveniences section — the
  policy this ADR records replaces relitigating each of those per tool.
