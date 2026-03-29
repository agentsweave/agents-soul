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
