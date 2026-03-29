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
