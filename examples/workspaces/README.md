# Workspace Config Examples

These examples are intentionally aligned to the current loader contract in
`src/domain/config.rs` and `src/app/config.rs`.

## Included Profiles

- `minimal/soul.toml`: shortest valid config; relies on defaults
- `healthy/soul.toml`: full authoring pattern with baseline traits, communication
  style, heuristics, limits, templates, sources, and adaptation
- `degraded/soul.toml`: explicit degraded/offline-safe posture using
  `offline_registry_behavior = "baseline-only"` and adaptation disabled

## Required Workspace Files

`soul.toml` is required and must live at the workspace root. Runtime contract
also requires:

- `.soul/patterns.sqlite`
- `.soul/adaptation_log.jsonl`

`.soul/context_cache.json` is optional and disposable.

## Notes For Operators

- `sources.registry_agent_id` may be omitted; it defaults to `agent_id`
  at load finalization time.
- Config persistence currently rewrites the full file in canonical TOML and
  does not preserve comments or original table layout.

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
