# agents-soul

`agents-soul` is the behavioral composition layer for the `agents-world` ecosystem.
It reads upstream truth from `agents-identify` and `agents-registry`, then turns that
state plus local soul configuration into a deterministic `BehavioralContext`.

## Boundaries

- `agents-soul` does not discover identity on its own
- `agents-soul` does not issue credentials, sessions, or registry truth
- all transports must delegate to one inner compose path
- revoked standing fails closed

## Current Status

This repository is bootstrapped with the crate and module layout needed by the plan.
Most modules currently provide compile-safe placeholders so follow-on beads can land
in stable files without inventing new structure.

Foundation validation has already frozen a few non-negotiable execution gates:

- `soul.toml` plus `.soul/patterns.sqlite` and `.soul/adaptation_log.jsonl` are the
  required workspace contract
- `.soul/context_cache.json` may exist, but it is disposable and never authority
- the inner compose path still needs the real `AppDeps` boundary tracked in
  `soul-1ip.8`
- transport work should not treat compose as production-ready until the silent
  default-config fallback bug in `soul-1ip.7` is removed

## Workspace Authoring Patterns

`soul.toml` schema in this repository is currently based on:

- required root fields: `agent_id`, `profile_name`
- optional defaulted sections: `trait_baseline`, `communication_style`,
  `decision_heuristics`, `limits`, `templates`, `adaptation`
- required source fields: `sources.identity_workspace`, `sources.registry_url`
- optional `sources.registry_agent_id` (defaults to `agent_id` if omitted)

Reference examples are tracked under `examples/workspaces/`:

- `examples/workspaces/minimal/soul.toml`
- `examples/workspaces/healthy/soul.toml`
- `examples/workspaces/degraded/soul.toml`

Workspace contract still requires:

- `soul.toml` in workspace root
- `.soul/patterns.sqlite`
- `.soul/adaptation_log.jsonl`

`.soul/context_cache.json` remains optional and disposable authority-wise.

## Operator Quickstart

Use the shipped workspace examples as starting points for real operator workflows:

```bash
mkdir -p ~/.souls/alpha/.soul
cp examples/workspaces/healthy/soul.toml ~/.souls/alpha/soul.toml
: > ~/.souls/alpha/.soul/patterns.sqlite
: > ~/.souls/alpha/.soul/adaptation_log.jsonl
```

After the workspace contract exists, the live CLI surface is:

```bash
agents-soul inspect --workspace ~/.souls/alpha --json
agents-soul compose --workspace ~/.souls/alpha --json
agents-soul compose --workspace ~/.souls/alpha --prefix-only
agents-soul explain --workspace ~/.souls/alpha --json
agents-soul configure --workspace ~/.souls/alpha --trait openness 0.80
agents-soul record --workspace ~/.souls/alpha --interaction-type review --outcome positive
agents-soul reset --workspace ~/.souls/alpha --scope trait --target openness
```

Operational notes:

- `configure` rewrites the full `soul.toml` in canonical TOML; comments and table layout are not preserved.
- `record` and `reset` will initialize `.soul/patterns.sqlite` if it is empty or missing.
- `.soul/context_cache.json` may be created by compose/read paths; it can be deleted at any time because it is not authoritative.
- The example workspaces cover config posture only. Restricted and fail-closed behavior are driven by upstream registry state, not by a local workspace toggle.

## Layered Workspace Config

The workspace loader reads `soul.toml` first, then merges `soul.d/*.toml` in
sorted filename order. Later overlays win for conflicting scalar fields, so
operators can keep one canonical workspace file and layer environment- or
machine-local policy on top without forking the whole config.

Use this split deliberately:

- keep stable workspace identity and broad defaults in `soul.toml`
- use `soul.d/*.toml` for manual overlays such as registry endpoints,
  communication style adjustments, or adaptation thresholds
- avoid putting the same field in multiple drop-ins unless filename ordering is
  an intentional precedence rule

Patch tooling (`configure`, `update_traits`, and the equivalent MCP tools)
rewrites only the canonical `soul.toml`. It does not normalize or delete
existing `soul.d/*.toml` files. Those overlays remain operator-managed and will
still be merged after the rewritten base file loads.

See `examples/workspaces/overlayed/` for a runnable overlayed workspace example.

## Adaptive Persistence Throttling

`adaptation.min_persist_interval_seconds` controls how often bounded adaptation
state is durably rewritten. The default is `300` seconds. When a new adaptive
candidate arrives inside that window:

- `record_interaction` still returns the new candidate state
- the response effect is `session-only`
- durable workspace state is left unchanged until the interval elapses
- `inspect` and `explain_report` continue to reflect the last durable state, but
  now include the effective `min_interactions_for_adapt` and
  `min_persist_interval_seconds` values so operators can see the active policy

This keeps rapid bursts of evidence observable without rewriting SQLite on every
interaction.

## Top-Level Layout

- `src/app`: application wiring and runtime bootstrap
- `src/domain`: shared contracts and behavioral types
- `src/sources`: upstream readers and normalization surface
- `src/services`: centralized compose pipeline and render helpers
- `src/adaptation`: bounded adaptation helpers
- `src/storage`: persistence and fixtures
- `src/cli`, `src/api`, `src/mcp`: thin transport layers


## Runtime Contract
The current runtime seam is:
- `main` -> `SoulRuntime::run`
- transport entrypoint -> `AppDeps::compose_context`
- `AppDeps` -> `ComposeService::compose`
- `ComposeService` -> normalized inputs -> `BehavioralContext`
This boundary exists so CLI, REST, and MCP stay thin and all behavior flows through one shared compose path.
`AppDeps` is the authoritative bootstrap boundary for:
- workspace config loading
- adaptation state loading
- identify / registry reads
- template rendering
- clock access and provenance hashing
- transport error mapping via `map_soul_error`
Failure semantics are centralized in `SoulError` plus `map_soul_error`, so CLI exit codes, HTTP responses, and MCP tool failures stay consistent across transports.

## Compose-Mode Runbook

Use `inspect --warnings --provenance --json`, `compose --json`, and `explain --json` together when a session is not behaving as expected.

- Normal:
  Upstream identity and registry inputs are available and verified. Full commitments, relationships, heuristics, and adaptive notes can flow into the rendered context.
- Restricted:
  Triggered by `registry=suspended`. Expect warning codes including `registry_suspended` and `compose_restricted`, plus a prompt prefix that starts with `RESTRICTED:`.
- Degraded:
  Triggered when upstream inputs are partial or degraded. Expect reduced initiative and warning codes such as `compose_degraded`, `identity_degraded`, and `reputation_unavailable`.
- Baseline-only:
  Triggered when identity inputs are unavailable but registry verification still succeeds. Expect warning codes `baseline_only`, `identity_unavailable`, and `reputation_unavailable`, with commitments and relationship context left empty.
- Fail-closed:
  Triggered by `registry=revoked`. Expect warning codes `registry_revoked` and `compose_fail_closed`, a prompt prefix that starts with `FAIL-CLOSED:`, and stripped commitments, relationships, and adaptive notes.

When diagnosing a degraded or fail-closed session:

- Check whether the workspace still has `soul.toml`, `.soul/patterns.sqlite`, and `.soul/adaptation_log.jsonl`.
- Re-run with explicit fixture or snapshot paths if you need to isolate upstream input problems:
  `agents-soul compose --workspace ~/.souls/alpha --identity-snapshot /tmp/identity.json --registry-verification /tmp/verification.json --registry-reputation /tmp/reputation.json --json`
- Treat `.soul/context_cache.json` as disposable. Delete it if you suspect stale cache state, then rerun `inspect` or `compose`.
- Do not try to "fix" fail-closed in this repo. A revoked result must stay revoked until upstream registry state changes.

## Validation Commands

Use these command tiers as the canonical definition of done for local work and CI:

- Fast unit slice:
  - `cargo test services::templates:: -- --nocapture`
  - `cargo test --lib`
- Integration and parity slice:
  - `cargo test --test compose_modes -- --nocapture`
  - `cargo test --test transport_parity -- --nocapture`
  - `cargo test --test rendering_snapshots -- --nocapture`
- Full verification gate:
  - `cargo fmt --check`
  - `cargo check --all-targets`
  - `cargo test --all-targets`
  - `ubs --diff .`

## Fixture And Snapshot Flow

- `tests/fixtures/compose_modes/` contains the explicit identity and registry JSON used by
  compose-mode and transport-parity tests.
- `tests/fixtures/rendering/` contains the golden Markdown snapshots for end-to-end prompt,
  full-context, and explain rendering.
- `tests/transport_parity.rs` compares the stable CLI, REST, and MCP payload slices and
  normalizes float serialization so the assertions track semantic parity instead of JSON
  encoding noise.
- `tests/rendering_snapshots.rs` re-renders the `normal`, `restricted`, `degraded`, and
  `fail_closed` cases into the snapshot fixtures.
- To refresh rendering fixtures intentionally, run:
  - `UPDATE_SNAPSHOTS=1 cargo test --test rendering_snapshots -- --nocapture`
- After updating snapshots, rerun the integration/parity slice and then the full verification
  gate so spacing, escaping, and transport drift regressions are caught before review.
