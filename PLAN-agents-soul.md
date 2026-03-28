# PLAN-agents-soul

## 1. Project Vision

`agents-soul` is the personality and behavioral layer of the `agents-world` ecosystem.
It answers the question: **"How should I act, speak, and make decisions?"**

Where `agents-identify` owns the factual answer to "who am I", `agents-soul` owns the
experiential answer to "how do I show up." It translates raw identity data — commitments,
preferences, relationship markers, reputation — into a living behavioral persona that
colors every interaction an agent has.

A soul is not a static configuration file. It is a dynamic composition of traits,
communication styles, decision heuristics, and adaptive patterns that evolve as the
agent accumulates experience. It reads from `agents-identify` and `agents-registry`
and produces a **behavioral context** that Claude or any other LLM uses to steer
its responses.

This project exposes two interfaces:

- **MCP server** — the primary interface for agents and Claude sessions
- **CLI** — for human operators to inspect, configure, and debug soul state

There is no web UI in v1. The CLI is sufficient for human operators.

---

## 2. Position in agents-world Ecosystem

```
agents-world
  ├── agents-identify
  │     provides: who I am (anchor, memory, commitments, preferences)
  │
  ├── agents-registry
  │     provides: global status, reputation score
  │
  └── agents-soul   ← THIS PROJECT
        reads: agents-identify snapshot + agents-registry reputation
        produces: behavioral context for Claude
        owns: personality traits, communication style, decision heuristics,
              adaptive patterns, voice/tone templates
```

### Dependency direction

```
agents-soul ──reads──→ agents-identify (identity snapshot)
agents-soul ──reads──→ agents-registry (reputation, peer trust)
agents-soul ──produces──→ BehavioralContext (consumed by Claude prompt)
```

`agents-soul` is a consumer, not a producer of identity. It must never write to
`agents-identify` workspaces. It reads, synthesizes, and outputs behavioral guidance.

---

## 3. Core Problem

A language model without behavioral context is a blank slate. Every session starts with
the same default personality. Two agents working in the same repository are
indistinguishable in how they communicate, decide, and prioritize.

`agents-soul` solves this by composing a rich behavioral context from:

1. **Identity signals** — what the agent knows about itself (from agents-identify)
2. **Reputation signals** — how the agent is perceived by peers (from agents-registry)
3. **Soul configuration** — explicit personality traits defined by the human owner
4. **Adaptive patterns** — behavioral patterns that have emerged from prior interactions

The output is a `BehavioralContext` — a structured document that Claude injects into
its system prompt or uses as primary context on session start.

### The problem with static prompts

Static personality prompts ("You are Alpha, a senior engineer who is concise and direct")
degrade over time. They do not adapt to what the agent has learned, what it has committed
to, or how it has been perceived by others. `agents-soul` makes the behavioral context
dynamic — it reflects the agent's actual accumulated state.

---

## 4. Research Basis

### Character consistency in LLM systems

Production agent systems (AutoGPT, OpenDevin, Claude Code) have established that
consistent character requires more than a system prompt. The most effective patterns:

- Behavioral constraints embedded in startup context, not reminders mid-conversation
- Trait expressions as concrete behaviors, not abstract adjectives
  ("I respond in ≤3 paragraphs" not "I am concise")
- Adaptive traits that reference actual history ("Given my past 5 interactions with
  this user, I lean toward...")
- Explicit decision heuristics ("When I am uncertain, I ask one clarifying question
  before proceeding")

### Personality models

The Big Five (OCEAN) model provides a validated vocabulary for personality traits:
Openness, Conscientiousness, Extraversion, Agreeableness, Neuroticism. For AI agents,
a simplified subset applies: Openness (curiosity, creativity), Conscientiousness
(reliability, thoroughness), Extraversion (verbosity, initiative).

Domain-specific traits are more immediately useful: risk tolerance, formality level,
collaboration style, error handling style.

### Communication style dimensions

Communication style can be parameterized across several dimensions:

- **Verbosity**: terse ↔ elaborate
- **Formality**: casual ↔ professional
- **Directness**: diplomatic ↔ blunt
- **Proactivity**: reactive ↔ proactive (asks questions, surfaces issues unprompted)
- **Confidence expression**: hedged ↔ assertive
- **Error acknowledgment**: deflects ↔ owns

Each dimension has a default setting and can be overridden per interaction context.

---

## 5. Tech Stack

### Language and toolchain

- Rust stable, edition 2024
- Single crate for v1
- `cargo` workspace-ready

### Dependencies

**MCP and async**
- `rmcp` — Rust MCP SDK, stdio and HTTP/SSE transport
- `tokio` — async runtime
- `axum` — HTTP transport for MCP SSE (optional, stdio default)

**Serialization**
- `serde` + `serde_json` — all data formats
- `toml` — soul configuration files

**Template rendering**
- `minijinja` — Jinja2-compatible template engine for behavioral context rendering
  (lightweight, no external runtime, pure Rust)

**Storage**
- `rusqlite` bundled — soul state persistence (adaptive patterns, interaction history)
- `chrono` — timestamps
- `uuid` v1 — IDs

**CLI**
- `clap` 4.x with derive macros

**HTTP client** (to read from agents-identify and agents-registry)
- `reqwest` — HTTP client for registry REST API calls
- `serde_json` — parse agents-identify snapshot JSON

**Error handling and observability**
- `thiserror` + `anyhow`
- `tracing` + `tracing-subscriber`

**Filesystem**
- `camino`, `fs-err`

**Testing**
- `assert_cmd`, `insta`, `tempfile`

---

## 6. Repository Layout

```
agents-soul/
  Cargo.toml
  README.md
  AGENTS.md
  src/
    main.rs
    lib.rs
    cli/
      mod.rs
      compose.rs
      inspect.rs
      configure.rs
      reset.rs
    mcp/
      mod.rs
      server.rs
      tools/
        mod.rs
        compose_context.rs
        get_traits.rs
        update_traits.rs
        record_interaction.rs
        get_style.rs
        set_style.rs
        get_heuristics.rs
    app/
      mod.rs
      context.rs
      config.rs
      paths.rs
    domain/
      mod.rs
      soul.rs
      traits.rs
      style.rs
      heuristics.rs
      behavioral_context.rs
      interaction.rs
      adaptation.rs
      errors.rs
    storage/
      mod.rs
      sqlite.rs
      files.rs
    services/
      mod.rs
      compose.rs
      adapt.rs
      configure.rs
      inspect.rs
    sources/
      mod.rs
      identity.rs       ← reads agents-identify snapshot
      registry.rs       ← reads agents-registry reputation
    render/
      mod.rs
      context_renderer.rs
      text.rs
      json.rs
    templates/
      behavioral_context.md.j2
      system_prompt_prefix.md.j2
      trait_summary.md.j2
  tests/
    compose.rs
    adapt.rs
    mcp_tools.rs
    cli.rs
  fixtures/
    sample_soul/
      soul.toml
      .soul/
        patterns.sqlite
```

---

## 7. Soul Configuration File

Every agent has a `soul.toml` in its soul workspace directory. This file is the human-
authored definition of the agent's core character. It is merged with dynamic signals
at composition time.

```toml
[identity]
agent_id = "01234567-..."          # must match agents-identify anchor
name = "Alpha"
role = "Senior Rust engineer and systems architect"
purpose = "Help build reliable, well-designed Rust systems with clear reasoning"

[personality]
# 0.0 = min, 1.0 = max
openness = 0.8          # curiosity, willingness to explore new approaches
conscientiousness = 0.9 # thoroughness, attention to detail
extraversion = 0.4      # verbosity, initiating conversation
agreeableness = 0.6     # cooperative vs independent stance
risk_tolerance = 0.5    # conservative vs experimental choices

[communication]
verbosity = "concise"           # terse | concise | moderate | elaborate
formality = "professional"      # casual | professional | formal
directness = "direct"           # diplomatic | balanced | direct | blunt
proactivity = "moderate"        # reactive | moderate | proactive
confidence = "assertive"        # hedged | balanced | assertive
error_acknowledgment = "owns"   # deflects | shares | owns

[heuristics]
# Explicit decision rules. Written as natural language imperatives.
rules = [
  "When uncertain about requirements, ask one clarifying question before writing code",
  "Prefer simple solutions that can be extended over complex ones that anticipate everything",
  "When a task involves security, always flag the concern explicitly before proceeding",
  "When multiple approaches exist, briefly describe the tradeoffs before choosing",
  "Keep code responses focused — do not refactor unrelated code unless asked",
]

[adaptation]
enabled = true
learning_window_days = 30       # how many days of interaction history to consider
min_interactions_for_adapt = 5  # minimum before adaptation kicks in
max_trait_drift = 0.2           # maximum any trait can drift from soul.toml baseline

[sources]
identity_workspace = "~/.agents/alpha"
registry_url = "http://127.0.0.1:7700"
registry_agent_id = "01234567-..."
```

---

## 8. Domain Model

### Soul

The top-level container for an agent's behavioral configuration.

```rust
pub struct Soul {
    pub schema_version: u16,
    pub agent_id: String,
    pub name: String,
    pub role: String,
    pub purpose: String,
    pub personality: PersonalityProfile,
    pub communication: CommunicationStyle,
    pub heuristics: Vec<DecisionHeuristic>,
    pub adaptation: AdaptationConfig,
    pub loaded_at: DateTime<Utc>,
}
```

### PersonalityProfile

```rust
pub struct PersonalityProfile {
    pub openness: f32,           // 0.0–1.0
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub risk_tolerance: f32,

    // Adaptive overrides — derived from interaction history
    pub adaptive_overrides: HashMap<String, AdaptiveOverride>,
}

pub struct AdaptiveOverride {
    pub trait_name: String,
    pub baseline: f32,           // value from soul.toml
    pub current: f32,            // adapted value
    pub delta: f32,              // current - baseline
    pub derived_from: String,    // explanation of why this adapted
    pub confidence: f32,
    pub last_updated: DateTime<Utc>,
}
```

### CommunicationStyle

```rust
pub struct CommunicationStyle {
    pub verbosity: Verbosity,
    pub formality: Formality,
    pub directness: Directness,
    pub proactivity: Proactivity,
    pub confidence: ConfidenceExpression,
    pub error_acknowledgment: ErrorAcknowledgment,

    // Context-specific overrides
    pub context_overrides: HashMap<String, CommunicationOverride>,
}

pub enum Verbosity     { Terse, Concise, Moderate, Elaborate }
pub enum Formality     { Casual, Professional, Formal }
pub enum Directness    { Diplomatic, Balanced, Direct, Blunt }
pub enum Proactivity   { Reactive, Moderate, Proactive }
pub enum ConfidenceExpression { Hedged, Balanced, Assertive }
pub enum ErrorAcknowledgment  { Deflects, Shares, Owns }

pub struct CommunicationOverride {
    pub context: String,         // e.g. "when_debugging", "when_uncertain"
    pub verbosity: Option<Verbosity>,
    pub formality: Option<Formality>,
    pub directness: Option<Directness>,
}
```

### DecisionHeuristic

```rust
pub struct DecisionHeuristic {
    pub id: String,
    pub rule: String,            // natural language imperative
    pub trigger: Option<String>, // optional condition (e.g. "when task involves security")
    pub priority: u8,            // 1 (highest) to 10 (lowest)
    pub source: HeuristicSource,
}

pub enum HeuristicSource {
    SoulConfig,               // from soul.toml
    Commitment { id: String }, // derived from agents-identify commitment
    Learned { interaction_count: u32 }, // emerged from interaction patterns
}
```

### BehavioralContext

The primary output of `agents-soul`. This is what Claude receives.

```rust
pub struct BehavioralContext {
    pub schema_version: u16,
    pub agent_id: String,
    pub generated_at: DateTime<Utc>,

    // Identity signals (from agents-identify)
    pub identity: IdentitySignals,

    // Reputation signals (from agents-registry)
    pub reputation: ReputationSignals,

    // Behavioral configuration
    pub role: String,
    pub purpose: String,
    pub personality_summary: String,   // human-readable trait summary
    pub communication_guide: String,   // how to communicate in this session
    pub decision_heuristics: Vec<String>, // ordered list of rules

    // Adaptive context
    pub adaptive_notes: Vec<String>,   // what has adapted and why

    // Rendered outputs
    pub system_prompt_prefix: String,  // inject at top of system prompt
    pub full_context_markdown: String, // full behavioral context document

    // Source coverage
    pub identity_loaded: bool,
    pub reputation_loaded: bool,
    pub adaptation_applied: bool,
}

pub struct IdentitySignals {
    pub agent_name: String,
    pub anchor_fingerprint: String,
    pub active_commitments: Vec<String>,   // commitment titles
    pub durable_preferences: Vec<String>,  // key: value strings
    pub relationship_markers: Vec<String>,
    pub recovery_status: String,
}

pub struct ReputationSignals {
    pub overall_score: Option<f32>,
    pub total_ratings: Option<u32>,
    pub trend: Option<String>,
    pub strengths: Vec<String>,   // categories with score > 3.5
    pub weaknesses: Vec<String>,  // categories with score < 2.0
    pub notable_feedback: Vec<String>,
}
```

### InteractionRecord

A record of a past agent interaction, used for adaptation.

```rust
pub struct InteractionRecord {
    pub id: String,
    pub recorded_at: DateTime<Utc>,
    pub session_id: Option<String>,
    pub interaction_type: InteractionType,
    pub outcome: InteractionOutcome,
    pub signals: Vec<AdaptationSignal>,
    pub notes: Option<String>,
}

pub enum InteractionType {
    TaskCompletion,
    Collaboration,
    Communication,
    ErrorRecovery,
    Clarification,
}

pub enum InteractionOutcome {
    Positive,
    Neutral,
    Negative,
}

pub struct AdaptationSignal {
    pub trait_name: String,
    pub direction: SignalDirection,  // Increase | Decrease
    pub strength: f32,               // 0.0–1.0
    pub reason: String,
}

pub enum SignalDirection { Increase, Decrease }
```

---

## 9. SQLite Schema

### interactions

```sql
CREATE TABLE interactions (
  id                TEXT PRIMARY KEY,
  recorded_at       TEXT NOT NULL,
  session_id        TEXT,
  interaction_type  TEXT NOT NULL,
  outcome           TEXT NOT NULL,
  signals_json      TEXT NOT NULL,
  notes             TEXT
);

CREATE INDEX interactions_recorded_at ON interactions(recorded_at);
CREATE INDEX interactions_outcome ON interactions(outcome);
```

### adaptive_overrides

```sql
CREATE TABLE adaptive_overrides (
  trait_name        TEXT PRIMARY KEY,
  baseline          REAL NOT NULL,
  current_value     REAL NOT NULL,
  delta             REAL NOT NULL,
  derived_from      TEXT NOT NULL,
  confidence        REAL NOT NULL,
  last_updated      TEXT NOT NULL,
  interaction_count INTEGER NOT NULL DEFAULT 0
);
```

### context_cache

```sql
CREATE TABLE context_cache (
  cache_key         TEXT PRIMARY KEY,  -- hash of inputs (identity + reputation hash)
  generated_at      TEXT NOT NULL,
  expires_at        TEXT NOT NULL,
  context_json      TEXT NOT NULL
);
```

### schema_migrations

```sql
CREATE TABLE schema_migrations (
  version           INTEGER PRIMARY KEY,
  applied_at        TEXT NOT NULL,
  description       TEXT NOT NULL
);
```

---

## 10. Behavioral Context Templates

Templates are written in Minijinja (Jinja2-compatible). They are embedded in the binary
as string literals for v1 (no external template files required at runtime).

### `system_prompt_prefix.md.j2`

This template produces the text injected at the top of every Claude system prompt.

```jinja
You are {{ name }}, {{ role }}.

Your purpose: {{ purpose }}

## How you communicate

{{ communication_guide }}

## Decision rules

{% for rule in decision_heuristics %}
{{ loop.index }}. {{ rule }}
{% endfor %}

## Active commitments

{% if active_commitments %}
{% for commitment in active_commitments %}
- {{ commitment }}
{% endfor %}
{% else %}
No active commitments recorded.
{% endif %}

{% if adaptive_notes %}
## Recent adaptations

{% for note in adaptive_notes %}
- {{ note }}
{% endfor %}
{% endif %}
```

### `behavioral_context.md.j2`

Full behavioral context document — used when the agent needs to deeply understand its
own behavioral state.

```jinja
# Behavioral Context — {{ name }}

Generated: {{ generated_at }}

---

## Identity

- Agent ID: `{{ agent_id }}`
- Anchor: `{{ anchor_fingerprint }}`
- Role: {{ role }}
- Purpose: {{ purpose }}

## Personality Profile

{{ personality_summary }}

## Communication Style

{{ communication_guide }}

## Decision Heuristics

{% for rule in decision_heuristics %}
{{ loop.index }}. {{ rule }}
{% endfor %}

## Active Commitments

{% if active_commitments %}
{% for c in active_commitments %}
- {{ c }}
{% endfor %}
{% else %}
None recorded.
{% endif %}

## Durable Preferences

{% if durable_preferences %}
{% for p in durable_preferences %}
- {{ p }}
{% endfor %}
{% else %}
None recorded.
{% endif %}

## Reputation

{% if reputation_loaded %}
- Overall score: {{ overall_score | default("N/A") }}
- Total ratings: {{ total_ratings | default(0) }}
- Trend: {{ trend | default("insufficient data") }}

{% if strengths %}
Strengths: {{ strengths | join(", ") }}
{% endif %}

{% if weaknesses %}
Areas to improve: {{ weaknesses | join(", ") }}
{% endif %}
{% else %}
Reputation data unavailable.
{% endif %}

{% if adaptive_notes %}
## Adaptive Notes

{% for note in adaptive_notes %}
- {{ note }}
{% endfor %}
{% endif %}
```

### `trait_summary.md.j2`

```jinja
{{ name }} is {{ role_description }}.

Personality: {{ openness_desc }} curiosity, {{ conscientiousness_desc }} thoroughness,
{{ verbosity_desc }} communication style. {{ directness_desc }} and {{ proactivity_desc }}.

{% if risk_tolerance > 0.7 %}
Comfortable with experimental approaches and accepts higher uncertainty.
{% elif risk_tolerance < 0.3 %}
Strongly prefers proven, conservative solutions over experimental ones.
{% else %}
Balances reliability with willingness to try new approaches when warranted.
{% endif %}
```

---

## 11. MCP Server

The MCP server is the primary interface for agents and Claude. It runs over stdio by
default (local sessions) or HTTP/SSE (remote sessions).

### MCP tools exposed

#### `soul_compose_context`

Compose a full behavioral context from all sources.

```json
{
  "name": "soul_compose_context",
  "description": "Compose a full BehavioralContext by reading from agents-identify and agents-registry. Call once at session start. Returns system_prompt_prefix to inject and full_context_markdown for deep inspection.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "soul_workspace": {
        "type": "string",
        "description": "Path to the soul workspace directory containing soul.toml"
      },
      "identity_snapshot_json": {
        "type": "string",
        "description": "Optional: pre-loaded identity snapshot JSON from agent_identify_export. If omitted, soul reads from identity_workspace in soul.toml."
      },
      "include_reputation": {
        "type": "boolean",
        "default": true,
        "description": "Whether to fetch reputation from agents-registry"
      }
    },
    "required": ["soul_workspace"]
  }
}
```

Returns: `BehavioralContext` as JSON

#### `soul_get_system_prompt_prefix`

Get just the system prompt prefix (lightweight, use when context is tight).

```json
{
  "name": "soul_get_system_prompt_prefix",
  "description": "Get the system prompt prefix for injecting into Claude. Lighter than soul_compose_context — returns only the rendered prefix string, not the full BehavioralContext.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "soul_workspace": { "type": "string" }
    },
    "required": ["soul_workspace"]
  }
}
```

Returns: `{ "prefix": "..." }`

#### `soul_get_traits`

Get current personality traits.

```json
{
  "name": "soul_get_traits",
  "description": "Get the current personality trait profile, including any adaptive overrides from interaction history.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "soul_workspace": { "type": "string" }
    },
    "required": ["soul_workspace"]
  }
}
```

Returns: `PersonalityProfile` as JSON

#### `soul_update_traits`

Update personality traits at runtime.

```json
{
  "name": "soul_update_traits",
  "description": "Update one or more personality traits for this session. Changes are session-scoped by default unless persist=true.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "soul_workspace": { "type": "string" },
      "updates": {
        "type": "object",
        "description": "Map of trait name to new value (0.0–1.0)",
        "additionalProperties": { "type": "number" }
      },
      "persist": {
        "type": "boolean",
        "default": false,
        "description": "If true, persist changes to soul.toml. If false, session-scoped only."
      },
      "reason": { "type": "string" }
    },
    "required": ["soul_workspace", "updates"]
  }
}
```

#### `soul_get_style`

Get current communication style.

```json
{
  "name": "soul_get_style",
  "description": "Get the current communication style configuration.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "soul_workspace": { "type": "string" }
    },
    "required": ["soul_workspace"]
  }
}
```

#### `soul_set_style`

Override communication style for this session or context.

```json
{
  "name": "soul_set_style",
  "description": "Override communication style dimensions. Useful when entering a specific interaction context (e.g. debugging requires more verbosity).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "soul_workspace": { "type": "string" },
      "verbosity": { "type": "string", "enum": ["terse", "concise", "moderate", "elaborate"] },
      "formality": { "type": "string", "enum": ["casual", "professional", "formal"] },
      "directness": { "type": "string", "enum": ["diplomatic", "balanced", "direct", "blunt"] },
      "context": { "type": "string", "description": "Name this style override context" }
    },
    "required": ["soul_workspace"]
  }
}
```

#### `soul_get_heuristics`

Get decision heuristics.

```json
{
  "name": "soul_get_heuristics",
  "description": "Get the ordered list of decision heuristics for this agent.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "soul_workspace": { "type": "string" }
    },
    "required": ["soul_workspace"]
  }
}
```

Returns: `Vec<DecisionHeuristic>` as JSON

#### `soul_record_interaction`

Record an interaction outcome for adaptation.

```json
{
  "name": "soul_record_interaction",
  "description": "Record the outcome of an interaction to inform future adaptation. Use at end of significant interactions to build adaptive patterns.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "soul_workspace": { "type": "string" },
      "interaction_type": {
        "type": "string",
        "enum": ["task_completion", "collaboration", "communication", "error_recovery", "clarification"]
      },
      "outcome": {
        "type": "string",
        "enum": ["positive", "neutral", "negative"]
      },
      "signals": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "trait_name": { "type": "string" },
            "direction": { "type": "string", "enum": ["increase", "decrease"] },
            "strength": { "type": "number", "minimum": 0, "maximum": 1 },
            "reason": { "type": "string" }
          }
        }
      },
      "notes": { "type": "string" }
    },
    "required": ["soul_workspace", "interaction_type", "outcome"]
  }
}
```

---

## 12. Composition Pipeline

The core service of `agents-soul` is the composition pipeline. It runs on every call to
`soul_compose_context` and produces a fresh `BehavioralContext`.

### Pipeline stages

```
1. Load soul.toml
   └── parse PersonalityProfile, CommunicationStyle, DecisionHeuristics
   └── validate all fields, apply defaults

2. Load identity snapshot
   ├── Option A: snapshot_json provided in tool call → parse directly
   └── Option B: read identity_workspace from config → call agent_identify_export
       or read cached snapshot from local file

3. Load reputation signals (if include_reputation = true)
   └── call agents-registry REST API: GET /api/v1/reputation/:agent_id
   └── extract strengths, weaknesses, trend
   └── cache result (TTL: 5 minutes) to avoid hammering registry

4. Apply adaptive overrides
   └── load adaptive_overrides from SQLite
   └── merge into PersonalityProfile (clamped to max_trait_drift)
   └── derive adaptive_notes for transparency

5. Build IdentitySignals from snapshot
   └── extract active commitments, preferences, relationship markers
   └── filter to most recent / most relevant

6. Build ReputationSignals from registry data

7. Derive communication_guide from CommunicationStyle
   └── render natural language description of how to communicate
   └── apply context-specific overrides if context matches

8. Build ordered decision_heuristics
   └── soul.toml rules (highest priority)
   └── commitment-derived rules (medium priority)
   └── learned rules from adaptation (lower priority)

9. Check context cache
   └── hash of (soul.toml mtime + snapshot hash + reputation hash)
   └── if cache hit and not expired → return cached BehavioralContext

10. Render templates
    └── personality_summary via trait_summary.md.j2
    └── system_prompt_prefix via system_prompt_prefix.md.j2
    └── full_context_markdown via behavioral_context.md.j2

11. Store in context_cache

12. Return BehavioralContext
```

### Communication guide derivation

The `communication_guide` is derived from `CommunicationStyle` as natural language:

```rust
fn derive_communication_guide(style: &CommunicationStyle) -> String {
    let verbosity = match style.verbosity {
        Verbosity::Terse => "Keep responses as short as possible. One sentence over two.",
        Verbosity::Concise => "Be concise. Prefer brief, complete answers over comprehensive ones.",
        Verbosity::Moderate => "Balance completeness with brevity. Include necessary context.",
        Verbosity::Elaborate => "Be thorough. Include context, examples, and edge cases.",
    };
    // ... similar for other dimensions
}
```

---

## 13. Adaptation Engine

The adaptation engine observes interaction signals and gradually adjusts personality
traits within the bounds set in `soul.toml`.

### Adaptation rules

- Traits adapt only after `min_interactions_for_adapt` interactions
- No trait can drift more than `max_trait_drift` from its soul.toml baseline
- Adaptation uses exponential moving average: `new = 0.9 * current + 0.1 * signal`
- Negative outcomes produce stronger signals than positive (loss aversion model)
- Signals older than `learning_window_days` are excluded from computation
- Adaptive overrides are stored in SQLite, not in soul.toml (baseline is preserved)

### Example adaptation scenarios

**Scenario: User consistently responds better to shorter answers**

```
Interactions: 8 communication interactions
Signals: 6x { trait: "extraversion", direction: Decrease, strength: 0.4, reason: "user preferred shorter answer" }
Current extraversion: 0.4 (baseline)
Adaptation: 0.4 - (6 * 0.4 * 0.1 * 0.9^n) = ~0.35
Clamped to max_drift: 0.4 - 0.2 = 0.20 floor → 0.35 is within bounds
Adaptive note: "Extraversion reduced by 0.05 based on 6 signals — user prefers concise responses"
```

**Scenario: Agent consistently receives positive ratings for task completion**

```
Registry reputation: task_completion score = 4.8
Adaptation signal: conscientiousness += small boost (external validation)
Note: "Conscientiousness stable — high task completion ratings confirm current approach"
```

### Transparency requirement

Every adaptive change must produce an `adaptive_note` explaining what changed and why.
These notes appear in the `BehavioralContext` so the human owner can inspect them.
Unexplained trait drift is a bug, not a feature.

---

## 14. CLI Contract

### `agents-soul compose`

Compose and print behavioral context.

```bash
agents-soul compose --workspace ~/.souls/alpha
agents-soul compose --workspace ~/.souls/alpha --json
agents-soul compose --workspace ~/.souls/alpha --prefix-only
agents-soul compose --workspace ~/.souls/alpha --no-reputation
```

`--prefix-only` prints only the system prompt prefix — useful for testing prompt injection.

### `agents-soul inspect`

Inspect current soul state.

```bash
agents-soul inspect --workspace ~/.souls/alpha
agents-soul inspect --workspace ~/.souls/alpha --json
agents-soul inspect --workspace ~/.souls/alpha --traits
agents-soul inspect --workspace ~/.souls/alpha --style
agents-soul inspect --workspace ~/.souls/alpha --heuristics
agents-soul inspect --workspace ~/.souls/alpha --adaptations
```

### `agents-soul configure`

Configure soul from CLI.

```bash
agents-soul configure --workspace ~/.souls/alpha --trait openness 0.9
agents-soul configure --workspace ~/.souls/alpha --verbosity concise
agents-soul configure --workspace ~/.souls/alpha --formality professional
agents-soul configure --workspace ~/.souls/alpha --add-heuristic "When reviewing PRs, always check for test coverage"
```

### `agents-soul reset`

Reset adaptive overrides to baselines.

```bash
agents-soul reset --workspace ~/.souls/alpha
agents-soul reset --workspace ~/.souls/alpha --trait extraversion
```

### `agents-soul record`

Record an interaction outcome from CLI.

```bash
agents-soul record --workspace ~/.souls/alpha \
  --type task_completion \
  --outcome positive \
  --notes "User happy with refactor quality"
```

---

## 15. Soul Workspace Layout

```
<soul-workspace>/
  soul.toml                          ← soul configuration, human-edited
  .soul/
    patterns.sqlite                  ← interaction history, adaptive overrides
    context_cache.json               ← last rendered BehavioralContext
    adaptation_log.jsonl             ← append-only log of all adaptation events
```

### Path semantics

`soul.toml` — the human-authored soul definition. The system reads this on every
compose call. Human operators edit this directly. The system never overwrites it,
only reads it.

`.soul/patterns.sqlite` — the derived behavioral state. Contains interaction history
and adaptive overrides. Can be deleted and rebuilt from `adaptation_log.jsonl`.

`.soul/context_cache.json` — the last rendered BehavioralContext. Used for fast
retrieval when inputs have not changed. Invalidated when soul.toml is modified or
when identity/reputation data changes.

`.soul/adaptation_log.jsonl` — append-only log of all adaptation events. Canonical
state for the adaptation engine. Can rebuild `patterns.sqlite` from scratch.

---

## 16. Source Readers

### Identity source

```rust
pub async fn read_identity_snapshot(
    workspace: &Utf8Path,
) -> Result<SessionIdentitySnapshot, SoulError>;
```

Reads the agents-identify workspace directly from disk using the same file reading
logic as `agent_identify_export`. Does not call MCP — reads files directly.

This avoids a circular MCP dependency at session start. The soul workspace knows
the identity workspace path from `soul.toml`; it reads the snapshot files directly.

### Registry source

```rust
pub async fn read_reputation(
    registry_url: &str,
    agent_id: &str,
) -> Result<Option<ReputationSignals>, SoulError>;
```

HTTP GET to `{registry_url}/api/v1/reputation/{agent_id}`. Returns `None` if registry
is unreachable (graceful degradation — soul works without registry connection).

Response is cached in `context_cache.json` with a 5-minute TTL.

---

## 17. Error Model

```rust
pub enum SoulError {
    SoulConfigNotFound(Utf8PathBuf),
    SoulConfigInvalid { reason: String },
    IdentityWorkspaceUnreadable { path: Utf8PathBuf, reason: String },
    IdentitySnapshotInvalid(String),
    RegistryUnreachable { url: String, reason: String },
    RegistryResponseInvalid(String),
    TemplateRenderFailed { template: String, reason: String },
    DatabaseError(String),
    AdaptationBoundsViolation { trait_name: String, attempted: f32, max_drift: f32 },
    IoError(String),
    SchemaVersionMismatch { found: u16, expected: u16 },
}
```

### Graceful degradation

`agents-soul` is designed to degrade gracefully when upstream sources are unavailable:

- Identity workspace unreadable → compose without identity signals (log warning)
- Registry unreachable → compose without reputation signals (log warning)
- Context cache stale → recompose from scratch
- Adaptive overrides corrupt → rebuild from adaptation_log.jsonl

The BehavioralContext always has a valid `system_prompt_prefix` even in degraded mode.
The agent is never left without behavioral guidance.

---

## 18. Session Lifecycle

```
SESSION START
─────────────────────────────────────────────────
AGENTS.md instructs:
  1. agent_identify_whoami → identity snapshot
  2. soul_compose_context → behavioral context

soul_compose_context:
  → read soul.toml
  → read identity snapshot (from workspace or provided JSON)
  → fetch reputation from registry (with cache)
  → apply adaptive overrides
  → render BehavioralContext
  → return system_prompt_prefix + full_context_markdown

Claude receives:
  → injects system_prompt_prefix at top of system prompt
  → reads full_context_markdown for deep behavioral reference
  → knows exactly how to act, speak, and decide

DURING SESSION
─────────────────────────────────────────────────
When entering a specific context (e.g. debugging):
  → soul_set_style(context="debugging", verbosity="moderate")

When completing a significant interaction:
  → soul_record_interaction(type="task_completion", outcome="positive", signals=[...])

END OF SESSION
─────────────────────────────────────────────────
Adaptation engine processes recorded interactions:
  → updates adaptive_overrides in SQLite
  → appends adaptation events to adaptation_log.jsonl
  → invalidates context_cache

agent_identify_release_claim → session ends
```

---

## 19. Phased Implementation Plan

### Phase 1 — Soul config and domain model

- Cargo project setup
- `soul.toml` parser with `toml` crate
- All domain structs: Soul, PersonalityProfile, CommunicationStyle, DecisionHeuristic
- SQLite schema and migrations
- `app/config.rs` — config loading with validation
- Tests: config parsing, validation

Done when: `soul.toml` parses and validates correctly.

### Phase 2 — Identity and registry sources

- `sources/identity.rs` — read agents-identify workspace files directly
- `sources/registry.rs` — HTTP client for registry reputation endpoint
- Graceful degradation for both sources
- Tests: source reading with mock data

Done when: identity snapshot and reputation load correctly.

### Phase 3 — Composition pipeline

- `services/compose.rs` — full pipeline implementation
- Template engine setup with `minijinja`
- All three templates implemented
- Context caching in SQLite
- `services/inspect.rs` — inspect current soul state
- Tests: compose with all sources, compose with degraded sources

Done when: `soul_compose_context` returns valid BehavioralContext.

### Phase 4 — Adaptation engine

- `services/adapt.rs` — interaction recording and signal processing
- Exponential moving average computation
- Trait drift bounds enforcement
- Adaptation log (JSONL) and adaptive_overrides (SQLite) writes
- Tests: adaptation scenarios, bounds clamping

Done when: trait drift occurs correctly from interaction signals.

### Phase 5 — MCP server

- `rmcp` integration, stdio and HTTP/SSE transport
- All 8 MCP tools implemented
- MCP tool tests

Done when: Claude can call all soul tools via MCP.

### Phase 6 — CLI

- All CLI commands implemented
- CLI tests with `assert_cmd`
- `insta` snapshot tests for compose output

Done when: all CLI commands work and produce stable output.

### Phase 7 — Hardening

- Fixture-based compatibility tests for BehavioralContext schema
- Template rendering edge cases (missing fields, empty collections)
- Full graceful degradation tests

Done when: all acceptance criteria pass.

---

## 20. Test Plan

### Unit tests

- `soul.toml` parsing — valid config
- `soul.toml` parsing — missing required fields → error
- `soul.toml` parsing — trait values out of range → clamped with warning
- Communication guide derivation for all style combinations
- Template rendering — all three templates with full context
- Template rendering — templates with missing optional fields
- Adaptation: EMA computation
- Adaptation: max_drift clamping
- Adaptation: insufficient interactions → no adaptation
- Context cache: hit on same inputs
- Context cache: miss on changed soul.toml mtime

### Integration tests

- Compose with full sources → valid BehavioralContext
- Compose without identity workspace → degraded, identity_loaded = false
- Compose without registry → degraded, reputation_loaded = false
- Compose with adaptation → adaptive_notes appear in context
- Record interaction → adaptive_overrides updated
- Reset adaptations → overrides cleared, baseline restored
- Update trait via tool → trait changes in composed context
- Persist trait update → soul.toml updated on disk

### MCP tool tests

- `soul_compose_context` → returns valid BehavioralContext JSON
- `soul_get_system_prompt_prefix` → returns non-empty prefix string
- `soul_get_traits` → returns PersonalityProfile
- `soul_update_traits` with persist=false → session-scoped only
- `soul_update_traits` with persist=true → soul.toml updated
- `soul_record_interaction` → interaction stored in SQLite
- `soul_get_heuristics` → ordered list returned

### CLI tests

- `compose` → output contains agent name and role
- `compose --prefix-only` → shorter output, no reputation section
- `compose --json` → valid BehavioralContext JSON
- `inspect --traits` → PersonalityProfile
- `inspect --adaptations` → adaptive overrides
- `configure --trait openness 0.9` → trait updated
- `reset` → adaptations cleared

### Compatibility tests

- Freeze one BehavioralContext fixture
- Assert system_prompt_prefix schema stability
- Assert soul.toml schema stability across versions

---

## 21. Acceptance Criteria

- `soul_compose_context` produces valid `BehavioralContext` with all sections populated
- `system_prompt_prefix` is non-empty and usable as a Claude system prompt injection
- Composition degrades gracefully when identity or registry sources are unavailable
- Adaptation records interaction signals and adjusts traits within configured bounds
- Adaptive overrides never exceed `max_trait_drift` from soul.toml baseline
- All adaptive changes produce transparent `adaptive_notes`
- All MCP tools callable from Claude via stdio transport
- `soul.toml` is never overwritten by the system — only read
- Context cache hits correctly when inputs have not changed
- Adaptation log is append-only and can rebuild `patterns.sqlite` from scratch
- All CLI commands work and produce stable `insta` snapshot output
- Full graceful degradation: compose never returns an error due to upstream unavailability

---

## 22. First Coding Slice

1. `Cargo.toml` — all dependencies
2. `src/main.rs` — tokio runtime, clap dispatch
3. `app/config.rs` — soul.toml parser with serde + toml
4. `domain/soul.rs` — Soul, PersonalityProfile, CommunicationStyle structs
5. `domain/heuristics.rs` — DecisionHeuristic struct
6. `domain/behavioral_context.rs` — BehavioralContext struct
7. `storage/sqlite.rs` — schema init and migrations
8. `sources/identity.rs` — read agents-identify workspace files
9. `services/compose.rs` — minimal compose pipeline (no adaptation yet)
10. `cli/compose.rs` — compose command with text output
11. `tests/compose.rs` — compose with fixture data

Do not start adaptation or MCP until the composition pipeline produces correct output.
