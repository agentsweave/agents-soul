# Workspace Config Examples

These examples are intentionally aligned to the current loader contract in
`src/domain/config.rs` and `src/app/config.rs`.

## Included Profiles

- `minimal/soul.toml`: shortest valid config; relies on defaults
- `healthy/soul.toml`: full authoring pattern with baseline traits, communication
  style, heuristics, limits, templates, sources, and adaptation
- `degraded/soul.toml`: explicit degraded/offline-safe posture using
  `offline_registry_behavior = "baseline-only"` and adaptation disabled
- `overlayed/`: canonical `soul.toml` plus `soul.d/*.toml` overlays showing
  operator-managed adaptation policy and style overrides

## Required Workspace Files

`soul.toml` is required and must live at the workspace root. Runtime contract
also requires:

- `.soul/patterns.sqlite`
- `.soul/adaptation_log.jsonl`

`.soul/context_cache.json` is optional and disposable.

If `soul.d/` exists, every non-hidden `*.toml` file is merged in sorted filename
order after `soul.toml` loads. Later files win for conflicting scalars.

## Bootstrap From An Example

To turn an example into a runnable workspace:

```bash
mkdir -p ~/.souls/alpha/.soul
cp examples/workspaces/healthy/soul.toml ~/.souls/alpha/soul.toml
: > ~/.souls/alpha/.soul/patterns.sqlite
: > ~/.souls/alpha/.soul/adaptation_log.jsonl
```

The empty `patterns.sqlite` file is sufficient; `record` and `reset` will
initialize the schema on first write.

## Notes For Operators

- `sources.registry_agent_id` may be omitted; it defaults to `agent_id`
  at load finalization time.
- Config persistence currently rewrites the full file in canonical TOML and
  does not preserve comments or original table layout.
- `configure_workspace` and `update_traits` rewrite only the canonical
  `soul.toml`. Existing `soul.d/*.toml` files are preserved as manual overlays
  and continue to apply on subsequent loads.
- Put stable workspace identity in `soul.toml`. Use `soul.d/*.toml` for
  operator-local policy overrides such as adaptation thresholds, communication
  style tweaks, or environment-specific registry endpoints.
- There is no separate `revoked/` workspace example because fail-closed mode is
  driven by upstream registry verification state, not by a local workspace flag.
- Throttled adaptive writes stay visible through `record_interaction` responses:
  when a write lands inside `adaptation.min_persist_interval_seconds`, the tool
  returns `effect = "session-only"` plus the candidate state even though
  `inspect` and `explain_report` still show the last durable state.

## Exercise The Examples

These commands match the current CLI contract:

```bash
agents-soul inspect --workspace ~/.souls/alpha --json
agents-soul compose --workspace ~/.souls/alpha --json
agents-soul compose --workspace ~/.souls/alpha --prefix-only
agents-soul explain --workspace ~/.souls/alpha --json
agents-soul configure --workspace ~/.souls/alpha --trait directness 0.85
agents-soul record --workspace ~/.souls/alpha --interaction-type review --outcome positive
agents-soul reset --workspace ~/.souls/alpha --scope trait --target directness
```

Recommended usage:

- `minimal/` proves the shortest valid config and the default
  `sources.registry_agent_id` behavior.
- `healthy/` is the full authoring reference for traits, style, heuristics,
  limits, templates, and adaptation.
- `degraded/` is the operator reference for offline-safe posture with
  `offline_registry_behavior = "baseline-only"` and adaptation disabled.
- `overlayed/` demonstrates the supported split between canonical config and
  manual overlays. Its drop-ins make adaptation react after one interaction and
  throttle durable rewrites for fifteen minutes.

## Default Values

When a field is omitted, `soul.toml` defaults are applied during load
finalization. The defaults below are the current contract defined in
`src/domain/config.rs`, `src/domain/profile.rs`, `src/domain/style.rs`,
and `src/domain/limits.rs`:

- `schema_version`: `1`
- `trait_baseline`:
  - `openness = 0.72`
  - `conscientiousness = 0.90`
  - `initiative = 0.84`
  - `directness = 0.81`
  - `warmth = 0.42`
  - `risk_tolerance = 0.28`
  - `verbosity = 0.34`
  - `formality = 0.71`
- `communication_style`:
  - `default_register = "professional-direct"`
  - `paragraph_budget = "short"`
  - `question_style = "single-clarifier-when-needed"`
  - `uncertainty_style = "explicit-and-bounded"`
  - `feedback_style = "frank"`
  - `conflict_style = "firm-respectful"`
- `limits`:
  - `max_trait_drift = 0.15`
  - `max_prompt_prefix_chars = 4000`
  - `max_adaptive_rules = 24`
  - `offline_registry_behavior = "cautious"`
  - `revoked_behavior = "fail-closed"`
- `templates`:
  - `prompt_prefix_template = "prompt-prefix"`
  - `full_context_template = "full-context"`
  - `explain_template = "explain"`
- `sources`:
  - `identity_workspace = "~/.agents/default"`
  - `registry_url = "http://127.0.0.1:7700"`
  - `registry_agent_id` defaults to `agent_id` if omitted
- `adaptation`:
  - `enabled = true`
  - `learning_window_days = 30`
  - `min_interactions_for_adapt = 5`
  - `min_persist_interval_seconds = 300`
