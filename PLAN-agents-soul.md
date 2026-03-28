# PLAN-agents-soul

## 1. Project Vision

`agents-soul` is the personality and behavioral layer of the `agents-world` ecosystem.
It answers the question: **"Given who I am locally and whether the registry says I am in
good standing, how should I act, speak, and make decisions?"**

Where `agents-identify` owns the local credential and continuity record, `agents-soul`
owns the experiential answer to "how do I show up." It translates verified upstream data
such as commitments, preferences, relationship markers, and registry standing into a
living behavioral persona that colors every interaction an agent has.

A soul is not a static configuration file. It is a dynamic composition of traits,
communication styles, decision heuristics, and adaptive patterns that evolve as the
agent accumulates experience. It reads from `agents-identify` and `agents-registry`,
but it never overrules either of them. It produces a **behavioral context** that Claude
or any other LLM uses to steer its responses.

This project exposes two interfaces:

- **MCP server** — the primary interface for agents and Claude sessions
- **CLI** — for human operators to inspect, configure, and debug soul state

There is no shared cross-repo console. If this repo needs a UI later, that UI belongs
inside this repo. For v1, the CLI is sufficient for human operators.

---

## 2. Position in agents-world Ecosystem

```
agents-world
  ├── agents-identify
  │     provides: local credential context, memory, commitments, preferences
  │
  ├── agents-registry
  │     provides: official status, verification result, reputation score
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

`agents-soul` is a consumer, not a producer of identity. It must never redefine
registration, never declare a revoked credential valid, and never write canonical
identity facts back into `agents-identify`. It reads, synthesizes, and outputs
behavioral guidance.

### Real-world analogy

In real-world terms, `agents-soul` is the agent's **personality and way of showing up**.
It is not the passport office, and it is not the wallet. It is the layer that determines
how a valid identity expresses itself in tone, judgment, and behavior.

- If `agents-identify` answers: "who am I?"
- And `agents-registry` answers: "am I recognized and in good standing?"
- Then `agents-soul` answers: "how should I act, speak, and decide?"

### Boundary rule

`agents-soul` is the only project allowed to own the agent's behavioral layer. It may
read identity and registry signals, but it must never become the source of truth for
identity validity or registration.

It is responsible for:

- personality traits
- communication style
- decision heuristics
- adaptive behavioral patterns
- rendered behavioral context for runtime use

It may synthesize behavior from upstream inputs, but it must not mutate the canonical
identity record held by `agents-identify`.

---

## 3. Core Problem

A language model without behavioral context is a blank slate. Every session starts with
the same default personality. Two agents working in the same repository are
indistinguishable in how they communicate, decide, and prioritize, even if they carry
different credentials and different registry standing.

`agents-soul` solves this by composing a rich behavioral context from:

1. **Identity signals** — what the agent knows about itself (from agents-identify)
2. **Registry signals** — whether the credential is active, degraded, suspended, or revoked
3. **Reputation signals** — how the agent is perceived by peers (from agents-registry)
4. **Soul configuration** — explicit personality traits defined by the human owner
5. **Adaptive patterns** — behavioral patterns that have emerged from prior interactions

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

---

## 23. Final Boundary and Authority Contract

This section freezes the final worldview for `agents-soul`.

### 23.1 Boundary statement

`agents-soul` owns behavior only.

It owns:

- trait baselines
- communication style
- decision heuristics
- adaptation rules
- rendered behavioral context

It does not own:

- official identity validity
- enrollment or revocation
- private key custody
- claim.lock or write.lock
- local commitment truth

### 23.2 Inputs and outputs

Inputs:

- local identity snapshot from `agents-identify`
- registry verification result from `agents-registry`
- reputation summary from `agents-registry`
- soul config and adaptation history from this repo

Outputs:

- `BehavioralContext`
- `SystemPromptPrefix`
- `BehaviorDecisionHints`

### 23.3 Final real-world analogy

Real-world mapping:

- `agents-identify` is the wallet and life file
- `agents-registry` is the identity authority
- `agents-soul` is the person's personality and style of action

### 23.4 Hard prohibitions

`agents-soul` must never:

- claim a revoked identity is usable
- invent a registry success when none exists
- mutate canonical identity files in `agents-identify`
- mutate registry standing or reputation directly

### 23.5 Degradation rule

If inputs are missing, `agents-soul` degrades the behavioral rendering, not the
identity truth. Example:

- missing registry status means behavior may become cautious
- missing identity commitments means output may omit commitment section
- revoked registry status means output should render fail-closed guidance

### 23.6 One session relationship

`agents-soul` assumes one live session maps to one agent because the claim rule is
enforced upstream. It does not attempt to solve session duplication itself.

---

## 24. Behavioral Composition Lifecycle

### 24.1 High-level flow

1. Load `soul.toml`.
2. Load local adaptation DB.
3. Read `SessionIdentitySnapshot` from `agents-identify`.
4. Read `VerificationResult` and `ReputationSummary` from `agents-registry`.
5. Normalize all inputs into `BehaviorInputs`.
6. Apply baseline traits from config.
7. Apply context-specific heuristic overrides.
8. Apply bounded adaptive overrides.
9. Render `BehavioralContext`.
10. Emit warnings and provenance data.

### 24.2 Why normalization exists

Normalization isolates transport differences:

- CLI file input
- REST JSON response
- MCP tool response

All inputs must become the same internal Rust structs before synthesis.

### 24.3 Composition modes

- `full`
- `prompt-prefix`
- `debug`
- `explain`

`full` returns the complete context.

`prompt-prefix` returns the compact system prompt prefix.

`debug` returns context plus provenance details.

`explain` returns why each trait or heuristic was chosen.

### 24.4 Registry-aware behavior

Behavior changes based on registry status:

- `active` means normal output
- `suspended` means render caution and restricted action hints
- `revoked` means render fail-closed guidance and no normal prompt
- `pending` means render probationary guidance
- `retired` means render historical/readonly guidance

### 24.5 Reputation-aware behavior

Reputation affects:

- level of self-confidence in claims
- how much the agent emphasizes caution
- whether self-check prompts are injected
- how strongly collaboration heuristics appear

### 24.6 Local commitments

Commitments from `agents-identify` affect:

- planning tone
- promised next actions
- urgency emphasis
- reminder sections

### 24.7 Relationship markers

Relationship markers affect:

- directness
- familiarity
- explanation depth
- sensitivity to prior friction

### 24.8 Determinism rule

Given the same normalized inputs, composition must be deterministic. Randomness is
not allowed in v1.

### 24.9 Caching rule

Behavioral output may be cached by input fingerprint, but the cache is disposable.

### 24.10 Provenance rule

Every major rendered section must be traceable back to:

- baseline config
- identity input
- registry input
- adaptation input

---

## 25. Canonical Domain Contract

### 25.1 `BehaviorInputs`

- `schema_version: u32`
- `identity_snapshot: Option<SessionIdentitySnapshot>`
- `verification_result: Option<VerificationResult>`
- `reputation_summary: Option<ReputationSummary>`
- `soul_config: SoulConfig`
- `adaptation_state: AdaptationState`
- `generated_at: DateTime<Utc>`

### 25.2 `SoulConfig`

- `schema_version: u32`
- `agent_id: String`
- `profile_name: String`
- `trait_baseline: PersonalityProfile`
- `communication_style: CommunicationStyle`
- `decision_heuristics: Vec<DecisionHeuristic>`
- `limits: SoulLimits`
- `templates: TemplateConfig`

### 25.3 `PersonalityProfile`

- `openness: f32`
- `conscientiousness: f32`
- `initiative: f32`
- `directness: f32`
- `warmth: f32`
- `risk_tolerance: f32`
- `verbosity: f32`
- `formality: f32`

### 25.4 `CommunicationStyle`

- `default_register: String`
- `paragraph_budget: String`
- `question_style: String`
- `uncertainty_style: String`
- `feedback_style: String`
- `conflict_style: String`

### 25.5 `DecisionHeuristic`

- `heuristic_id: String`
- `title: String`
- `priority: i32`
- `trigger: String`
- `instruction: String`
- `enabled: bool`

### 25.6 `SoulLimits`

- `max_trait_drift: f32`
- `max_prompt_prefix_chars: usize`
- `max_adaptive_rules: usize`
- `offline_registry_behavior: String`
- `revoked_behavior: String`

### 25.7 `AdaptationState`

- `schema_version: u32`
- `last_updated_at: Option<DateTime<Utc>>`
- `trait_overrides: PersonalityOverride`
- `communication_overrides: CommunicationOverride`
- `heuristic_overrides: Vec<HeuristicOverride>`
- `evidence_window_size: u32`
- `notes: Vec<String>`

### 25.8 `BehavioralContext`

- `schema_version: u32`
- `agent_id: String`
- `profile_name: String`
- `status_summary: StatusSummary`
- `trait_profile: PersonalityProfile`
- `communication_rules: Vec<String>`
- `decision_rules: Vec<String>`
- `active_commitments: Vec<String>`
- `relationship_context: Vec<String>`
- `adaptive_notes: Vec<String>`
- `warnings: Vec<String>`
- `system_prompt_prefix: String`
- `provenance: ProvenanceReport`

### 25.9 `StatusSummary`

- `identity_loaded: bool`
- `registry_verified: bool`
- `registry_status: Option<String>`
- `reputation_loaded: bool`
- `recovery_state: Option<String>`

### 25.10 `ProvenanceReport`

- `identity_fingerprint: Option<String>`
- `registry_verification_at: Option<DateTime<Utc>>`
- `config_hash: String`
- `adaptation_hash: String`
- `input_hash: String`

### 25.11 Design rule

All structs above must be usable unchanged in CLI `--json`, REST `data`, and MCP tool
results.

---

## 26. Input Contract with agents-identify and agents-registry

### 26.1 Identify dependency

Primary input from `agents-identify`:

- `SessionIdentitySnapshot`

Relevant fields consumed:

- `agent_id`
- `display_name`
- `recovery_state`
- `active_commitments`
- `durable_preferences`
- `relationship_markers`
- `facts`
- `warnings`

### 26.2 Registry dependency

Primary inputs from `agents-registry`:

- `VerificationResult`
- `ReputationSummary`

Relevant fields consumed:

- `status`
- `standing_level`
- `reason_code`
- `score_total`
- `score_recent_30d`
- `last_event_at`

### 26.3 Missing input policy

If identify snapshot missing:

- `identity_loaded = false`
- render warnings
- omit identity-derived sections
- keep baseline soul rendering

If registry verification missing:

- `registry_verified = false`
- apply configured offline behavior
- render caution notes

If reputation missing:

- omit reputation nuance
- do not fail composition

### 26.4 Revoked input policy

If registry returns `revoked`:

- return `BehavioralContext` with severe warnings
- set `system_prompt_prefix` to fail-closed minimal text
- no normal optimistic persona rendering

### 26.5 Suspended input policy

If registry returns `suspended`:

- render restricted operation guidance
- encourage human escalation
- reduce autonomous initiative

---

## 27. Transport Contract Parity

### 27.1 Required CLI commands

- `agents-soul compose`
- `agents-soul compose --prefix-only`
- `agents-soul compose --json`
- `agents-soul inspect`
- `agents-soul inspect --traits`
- `agents-soul inspect --heuristics`
- `agents-soul configure`
- `agents-soul reset`
- `agents-soul explain`

### 27.2 Required REST endpoints

- `POST /api/v1/compose`
- `GET /api/v1/traits`
- `PATCH /api/v1/traits`
- `GET /api/v1/heuristics`
- `POST /api/v1/interactions`
- `POST /api/v1/reset`
- `POST /api/v1/explain`

### 27.3 Required MCP tools

- `soul_compose_context`
- `soul_get_system_prompt_prefix`
- `soul_get_traits`
- `soul_update_traits`
- `soul_get_heuristics`
- `soul_record_interaction`
- `soul_reset_adaptations`
- `soul_explain_context`

### 27.4 CLI/REST/MCP parity rule

If `compose --json` returns a `BehavioralContext`, then:

- REST `POST /compose` must return that same payload under `data`
- MCP `soul_compose_context` must return that same payload under the tool result

### 27.5 Exit codes

- `0` success
- `2` validation error
- `3` upstream unavailable but degradable
- `4` revoked or fail-closed status
- `5` local config invalid
- `6` storage failure
- `7` internal error

---

## 28. Adaptation Engine Deep Spec

### 28.1 Adaptation principle

Adaptation is bounded learning, not personality drift without control.

### 28.2 Adaptation sources

- explicit interaction feedback
- repeated operator overrides
- observed response outcomes

### 28.3 Non-sources

Adaptation does not come from:

- random mood
- single isolated event unless configured
- unverified external gossip

### 28.4 Bounded drift

Every numeric trait change must stay within `max_trait_drift` from the config baseline.

### 28.5 Adaptation persistence

Canonical adaptation data lives in SQLite events plus derived current overrides.

### 28.6 Adaptation transparency

Every adaptive effect must be visible through:

- `adaptive_notes`
- `inspect --adaptations`
- `soul_explain_context`

### 28.7 Reset semantics

Reset clears adaptive overrides but never rewrites the baseline config.

### 28.8 Evidence windows

V1 tracks bounded evidence windows so recent interactions matter more.

### 28.9 Heuristic override rules

Heuristics can be:

- strengthened
- weakened
- temporarily disabled

but cannot disappear without explanation.

### 28.10 Manual operator precedence

Explicit operator config always wins over learned adaptation.

---

## 29. Reference Code: Domain Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulConfig {
    pub schema_version: u32,
    pub agent_id: String,
    pub profile_name: String,
    pub trait_baseline: PersonalityProfile,
    pub communication_style: CommunicationStyle,
    pub decision_heuristics: Vec<DecisionHeuristic>,
    pub limits: SoulLimits,
    pub templates: TemplateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub openness: f32,
    pub conscientiousness: f32,
    pub initiative: f32,
    pub directness: f32,
    pub warmth: f32,
    pub risk_tolerance: f32,
    pub verbosity: f32,
    pub formality: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationStyle {
    pub default_register: String,
    pub paragraph_budget: String,
    pub question_style: String,
    pub uncertainty_style: String,
    pub feedback_style: String,
    pub conflict_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionHeuristic {
    pub heuristic_id: String,
    pub title: String,
    pub priority: i32,
    pub trigger: String,
    pub instruction: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulLimits {
    pub max_trait_drift: f32,
    pub max_prompt_prefix_chars: usize,
    pub max_adaptive_rules: usize,
    pub offline_registry_behavior: String,
    pub revoked_behavior: String,
}
```

### 29.1 Reference code: behavioral context

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralContext {
    pub schema_version: u32,
    pub agent_id: String,
    pub profile_name: String,
    pub status_summary: StatusSummary,
    pub trait_profile: PersonalityProfile,
    pub communication_rules: Vec<String>,
    pub decision_rules: Vec<String>,
    pub active_commitments: Vec<String>,
    pub relationship_context: Vec<String>,
    pub adaptive_notes: Vec<String>,
    pub warnings: Vec<String>,
    pub system_prompt_prefix: String,
    pub provenance: ProvenanceReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSummary {
    pub identity_loaded: bool,
    pub registry_verified: bool,
    pub registry_status: Option<String>,
    pub reputation_loaded: bool,
    pub recovery_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceReport {
    pub identity_fingerprint: Option<String>,
    pub registry_verification_at: Option<DateTime<Utc>>,
    pub config_hash: String,
    pub adaptation_hash: String,
    pub input_hash: String,
}
```

### 29.2 Reference code: errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum SoulError {
    #[error("invalid soul config: {0}")]
    InvalidConfig(String),
    #[error("identity input unavailable")]
    IdentityUnavailable,
    #[error("registry verification unavailable")]
    RegistryUnavailable,
    #[error("revoked identity cannot compose normal context")]
    RevokedIdentity,
    #[error("storage error: {0}")]
    Storage(String),
}
```

---

## 30. Reference Code: Composition Pipeline

```rust
pub async fn compose_context(
    deps: &ComposeDeps,
    req: ComposeRequest,
) -> Result<BehavioralContext, SoulError> {
    let config = deps.config_store.load(&req.agent_id)?;
    let identity = deps.identity_reader.read_snapshot(&req).await.ok();
    let verification = deps.registry_reader.verify(&req).await.ok();
    let reputation = deps.registry_reader.reputation(&req).await.ok();
    let adaptation = deps.adaptation_store.load_current(&req.agent_id)?;

    let inputs = BehaviorInputs {
        schema_version: 1,
        identity_snapshot: identity,
        verification_result: verification,
        reputation_summary: reputation,
        soul_config: config,
        adaptation_state: adaptation,
        generated_at: Utc::now(),
    };

    let normalized = normalize_inputs(inputs)?;
    let profile = apply_profile_layers(&normalized)?;
    let communication_rules = derive_communication_rules(&normalized, &profile);
    let decision_rules = derive_decision_rules(&normalized, &profile);
    let commitments = derive_commitments(&normalized);
    let relationships = derive_relationship_context(&normalized);
    let adaptive_notes = derive_adaptive_notes(&normalized);
    let warnings = derive_warnings(&normalized);
    let system_prompt_prefix = render_system_prompt_prefix(
        &normalized,
        &profile,
        &communication_rules,
        &decision_rules,
        &warnings,
    )?;

    Ok(BehavioralContext {
        schema_version: 1,
        agent_id: normalized.agent_id.clone(),
        profile_name: normalized.profile_name.clone(),
        status_summary: build_status_summary(&normalized),
        trait_profile: profile,
        communication_rules,
        decision_rules,
        active_commitments: commitments,
        relationship_context: relationships,
        adaptive_notes,
        warnings,
        system_prompt_prefix,
        provenance: build_provenance(&normalized),
    })
}
```

### 30.1 Reference code: layering helpers

```rust
fn apply_profile_layers(inputs: &NormalizedInputs) -> Result<PersonalityProfile, SoulError> {
    let mut profile = inputs.config.trait_baseline.clone();

    if let Some(identity) = &inputs.identity {
        if identity.recovery_state == "degraded" {
            profile.risk_tolerance = clamp01(profile.risk_tolerance - 0.15);
            profile.conscientiousness = clamp01(profile.conscientiousness + 0.10);
        }
    }

    if let Some(verification) = &inputs.verification {
        match verification.status.as_str() {
            "active" => {}
            "suspended" => {
                profile.initiative = clamp01(profile.initiative - 0.30);
                profile.risk_tolerance = clamp01(profile.risk_tolerance - 0.25);
            }
            "revoked" => {
                profile.initiative = 0.0;
                profile.risk_tolerance = 0.0;
            }
            _ => {}
        }
    }

    apply_adaptive_overrides(&mut profile, &inputs.adaptation, inputs.config.limits.max_trait_drift);
    Ok(profile)
}
```

### 30.2 Reference code: prompt rendering

```rust
fn render_system_prompt_prefix(
    inputs: &NormalizedInputs,
    profile: &PersonalityProfile,
    communication_rules: &[String],
    decision_rules: &[String],
    warnings: &[String],
) -> Result<String, SoulError> {
    if matches!(inputs.registry_status(), Some("revoked")) {
        return Ok("Identity revoked. Do not continue normal operation. Surface the issue and request operator intervention.".to_string());
    }

    let mut lines = Vec::new();
    lines.push(format!("You are agent {}.", inputs.agent_id));
    lines.push(format!("Profile: {}.", inputs.profile_name));
    lines.push(format!(
        "Style: directness={:.2}, warmth={:.2}, verbosity={:.2}.",
        profile.directness, profile.warmth, profile.verbosity
    ));
    lines.extend(communication_rules.iter().cloned());
    lines.extend(decision_rules.iter().cloned());
    lines.extend(warnings.iter().cloned());
    Ok(lines.join("\n"))
}
```

---

## 31. Reference Code: Source Readers and Persistence

```rust
#[async_trait::async_trait]
pub trait IdentityReader {
    async fn read_snapshot(
        &self,
        req: &ComposeRequest,
    ) -> Result<SessionIdentitySnapshot, SoulError>;
}

#[async_trait::async_trait]
pub trait RegistryReader {
    async fn verify(
        &self,
        req: &ComposeRequest,
    ) -> Result<VerificationResult, SoulError>;

    async fn reputation(
        &self,
        req: &ComposeRequest,
    ) -> Result<ReputationSummary, SoulError>;
}

pub trait AdaptationStore {
    fn load_current(&self, agent_id: &str) -> Result<AdaptationState, SoulError>;
    fn record_interaction(&self, event: InteractionEvent) -> Result<(), SoulError>;
    fn reset(&self, agent_id: &str) -> Result<(), SoulError>;
}
```

### 31.1 Reference code: SQLite DDL

```sql
CREATE TABLE IF NOT EXISTS interaction_events (
    id INTEGER PRIMARY KEY,
    event_id TEXT NOT NULL UNIQUE,
    agent_id TEXT NOT NULL,
    signal_kind TEXT NOT NULL,
    signal_value REAL NOT NULL,
    context_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS adaptation_state (
    agent_id TEXT PRIMARY KEY,
    overrides_json TEXT NOT NULL,
    notes_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### 31.2 Reference code: TOML config

```toml
schema_version = 1
agent_id = "alpha"
profile_name = "Alpha Builder"

[trait_baseline]
openness = 0.72
conscientiousness = 0.90
initiative = 0.84
directness = 0.81
warmth = 0.42
risk_tolerance = 0.28
verbosity = 0.34
formality = 0.71

[communication_style]
default_register = "professional-direct"
paragraph_budget = "short"
question_style = "single-clarifier-when-needed"
uncertainty_style = "explicit-and-bounded"
feedback_style = "frank"
conflict_style = "firm-respectful"

[limits]
max_trait_drift = 0.15
max_prompt_prefix_chars = 4000
max_adaptive_rules = 24
offline_registry_behavior = "cautious"
revoked_behavior = "fail-closed"
```

---

## 32. Reference Code: Transport Surfaces

### 32.1 CLI sketch

```rust
#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Compose(ComposeCmd),
    Inspect(InspectCmd),
    Configure(ConfigureCmd),
    Reset(ResetCmd),
    Explain(ExplainCmd),
}
```

### 32.2 Axum router sketch

```rust
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/compose", post(api_compose))
        .route("/api/v1/traits", get(api_get_traits).patch(api_patch_traits))
        .route("/api/v1/heuristics", get(api_get_heuristics))
        .route("/api/v1/interactions", post(api_record_interaction))
        .route("/api/v1/reset", post(api_reset))
        .route("/api/v1/explain", post(api_explain))
        .with_state(state)
}
```

### 32.3 MCP tool sketch

```rust
pub async fn soul_compose_context(
    ctx: ToolContext,
    args: ComposeRequest,
) -> Result<BehavioralContext, McpError> {
    ctx.services
        .composer
        .compose_context(&ctx.deps, args)
        .await
        .map_err(mcp_map_error)
}
```

### 32.4 JSON example: compose response

```json
{
  "schema_version": 1,
  "agent_id": "alpha",
  "profile_name": "Alpha Builder",
  "status_summary": {
    "identity_loaded": true,
    "registry_verified": true,
    "registry_status": "active",
    "reputation_loaded": true,
    "recovery_state": "healthy"
  },
  "trait_profile": {
    "openness": 0.72,
    "conscientiousness": 0.91,
    "initiative": 0.80,
    "directness": 0.81,
    "warmth": 0.42,
    "risk_tolerance": 0.24,
    "verbosity": 0.34,
    "formality": 0.71
  },
  "communication_rules": [
    "Respond concisely and directly.",
    "Ask at most one clarifying question when uncertainty blocks safe action."
  ],
  "decision_rules": [
    "Honor active commitments before taking on new scope.",
    "If registry status is degraded or unknown, lower autonomous risk."
  ],
  "active_commitments": [
    "Finish contract review for registry integration."
  ],
  "relationship_context": [
    "User prefers direct, non-fluffy technical discussion."
  ],
  "adaptive_notes": [
    "Slightly reduced risk tolerance due to recent correction events."
  ],
  "warnings": [],
  "system_prompt_prefix": "You are agent alpha...",
  "provenance": {
    "identity_fingerprint": "abc123",
    "registry_verification_at": "2026-03-28T10:00:00Z",
    "config_hash": "cfg_001",
    "adaptation_hash": "adp_001",
    "input_hash": "inp_001"
  }
}
```

---

## 33. Detailed Implementation Backlog

### 33.1 App shell

- create `src/app/mod.rs`
- create `src/app/config.rs`
- create `src/app/deps.rs`
- create `src/app/hash.rs`
- add tracing bootstrap
- add config loading rules

### 33.2 Domain layer

- define `SoulConfig`
- define `PersonalityProfile`
- define `CommunicationStyle`
- define `DecisionHeuristic`
- define `BehaviorInputs`
- define `BehavioralContext`
- define `StatusSummary`
- define `ProvenanceReport`
- define `SoulError`
- add serde round-trip tests

### 33.3 Source readers

- implement file reader for identify export
- implement REST reader for registry verify
- implement MCP reader adapter for registry verify
- implement REST reader for reputation summary
- normalize all upstream errors
- cache last successful reads where appropriate

### 33.4 Composer

- implement input normalization
- implement baseline trait loading
- implement identity-based modifiers
- implement registry-status modifiers
- implement reputation-based modifiers
- implement commitment extraction
- implement relationship rendering
- implement warnings derivation
- implement provenance hash generation

### 33.5 Templates

- add template loader
- add system prompt prefix template
- add full context template
- add explain template
- test with missing optional sections

### 33.6 Adaptation

- design interaction event schema
- persist interaction events
- compute bounded overrides
- render adaptive notes
- reset overrides
- expose inspect command

### 33.7 CLI

- implement `compose`
- implement `compose --prefix-only`
- implement `compose --json`
- implement `inspect`
- implement `configure`
- implement `reset`
- implement `explain`

### 33.8 REST

- implement compose endpoint
- implement traits get/patch
- implement heuristics get
- implement interactions post
- implement reset post
- implement explain post

### 33.9 MCP

- implement `soul_compose_context`
- implement `soul_get_system_prompt_prefix`
- implement `soul_get_traits`
- implement `soul_update_traits`
- implement `soul_get_heuristics`
- implement `soul_record_interaction`
- implement `soul_reset_adaptations`
- implement `soul_explain_context`

### 33.10 Docs and fixtures

- add sample `soul.toml`
- add sample full context JSON
- add sample revoked fail-closed context
- add sample degraded offline context

---

## 34. Research and Validation Checklist

### 34.1 Behavioral science translation

- confirm chosen trait vocabulary is actionable for agents
- confirm traits map cleanly to concrete behaviors
- define stable ranges for each numeric trait

### 34.2 Runtime alignment

- verify prompt prefix length works with target models
- verify OpenClaw session memory plus soul prompt do not overconstrain
- define when context should be recomposed during a session

### 34.3 Safety research

- decide exact wording for revoked fail-closed mode
- decide exact wording for suspended restricted mode
- test if negative reputation should reduce confidence or directness

### 34.4 Operator ergonomics

- choose best explain output format
- choose best inspect output grouping
- choose whether config comments should be preserved on rewrite

---

## 35. Expanded Test Matrix

### 35.1 Composition tests

- full inputs available
- missing identity
- missing registry verify
- missing reputation
- revoked identity
- suspended identity
- degraded identity
- low reputation caution mode

### 35.2 Template tests

- full context render
- prompt prefix only render
- explain render
- missing optional commitments
- missing relationship markers

### 35.3 Adaptation tests

- record positive interaction
- record correction event
- bounded drift clamp
- reset clears overrides
- baseline survives reset

### 35.4 Contract snapshot tests

- compose JSON snapshot
- explain JSON snapshot
- traits JSON snapshot
- heuristics JSON snapshot

---

## 36. Detailed File Inventory

### 36.1 Crate root

- `src/main.rs`: CLI binary bootstrap.
- `src/lib.rs`: public crate surface.
- `src/app/mod.rs`: app wiring.
- `src/app/config.rs`: config loading.
- `src/app/deps.rs`: dependency container.
- `src/app/hash.rs`: content hashing helpers.
- `src/app/runtime.rs`: runtime bootstrap.

### 36.2 Domain files

- `src/domain/mod.rs`: domain exports.
- `src/domain/config.rs`: `SoulConfig`.
- `src/domain/profile.rs`: `PersonalityProfile`.
- `src/domain/style.rs`: `CommunicationStyle`.
- `src/domain/heuristics.rs`: `DecisionHeuristic`.
- `src/domain/limits.rs`: `SoulLimits`.
- `src/domain/behavioral_context.rs`: `BehavioralContext`.
- `src/domain/status.rs`: `StatusSummary`.
- `src/domain/provenance.rs`: `ProvenanceReport`.
- `src/domain/inputs.rs`: `BehaviorInputs`.
- `src/domain/adaptation.rs`: adaptation models.
- `src/domain/interactions.rs`: interaction event models.
- `src/domain/errors.rs`: `SoulError`.

### 36.3 Source readers

- `src/sources/mod.rs`: source exports.
- `src/sources/identity.rs`: identify readers.
- `src/sources/registry.rs`: registry readers.
- `src/sources/cache.rs`: optional response cache.
- `src/sources/normalize.rs`: normalization layer.

### 36.4 Services

- `src/services/mod.rs`: service exports.
- `src/services/compose.rs`: full composition pipeline.
- `src/services/profile.rs`: trait layering.
- `src/services/communication.rs`: communication rule derivation.
- `src/services/decision_rules.rs`: heuristic rendering.
- `src/services/relationships.rs`: relationship context rendering.
- `src/services/commitments.rs`: commitment rendering.
- `src/services/warnings.rs`: warning generation.
- `src/services/provenance.rs`: provenance generation.
- `src/services/explain.rs`: explain mode.
- `src/services/templates.rs`: template rendering.
- `src/services/limits.rs`: safety/limit enforcement.

### 36.5 Adaptation files

- `src/adaptation/mod.rs`: adaptation exports.
- `src/adaptation/store.rs`: SQLite persistence.
- `src/adaptation/ema.rs`: smoothing logic.
- `src/adaptation/bounds.rs`: drift clamping.
- `src/adaptation/overrides.rs`: effective override materialization.
- `src/adaptation/reset.rs`: reset path.
- `src/adaptation/notes.rs`: adaptive notes rendering.

### 36.6 Storage files

- `src/storage/mod.rs`: storage exports.
- `src/storage/sqlite.rs`: database layer.
- `src/storage/migrations.rs`: migrations.
- `src/storage/fixtures.rs`: fixture helpers.

### 36.7 Interfaces

- `src/cli/mod.rs`: CLI exports.
- `src/cli/compose.rs`: compose command.
- `src/cli/inspect.rs`: inspect command.
- `src/cli/configure.rs`: configure command.
- `src/cli/reset.rs`: reset command.
- `src/cli/explain.rs`: explain command.
- `src/api/mod.rs`: REST exports if enabled.
- `src/api/router.rs`: REST router.
- `src/api/compose.rs`: compose endpoint.
- `src/api/traits.rs`: traits endpoints.
- `src/api/heuristics.rs`: heuristics endpoint.
- `src/api/interactions.rs`: record interaction endpoint.
- `src/api/reset.rs`: reset endpoint.
- `src/api/explain.rs`: explain endpoint.
- `src/mcp/mod.rs`: MCP exports.
- `src/mcp/server.rs`: MCP server.
- `src/mcp/tools.rs`: MCP tool handlers.

### 36.8 Templates and fixtures

- `templates/context_full.j2`: full context template.
- `templates/prompt_prefix.j2`: prompt prefix template.
- `templates/explain.j2`: explain template.
- `fixtures/identity/healthy.json`: healthy identify fixture.
- `fixtures/identity/degraded.json`: degraded identify fixture.
- `fixtures/registry/active.json`: active verification fixture.
- `fixtures/registry/suspended.json`: suspended verification fixture.
- `fixtures/registry/revoked.json`: revoked verification fixture.
- `fixtures/context/full.json`: expected full context fixture.

---

## 37. Scenario Ledger

Each line is a scenario the implementation must handle.

- `SOUL-S001`: active identity + active registry + good reputation -> normal direct profile.
- `SOUL-S002`: active identity + active registry + no reputation -> omit reputation nuance.
- `SOUL-S003`: active identity + suspended registry -> restricted initiative.
- `SOUL-S004`: active identity + revoked registry -> fail-closed prefix.
- `SOUL-S005`: degraded identity + active registry -> lower risk tolerance.
- `SOUL-S006`: broken identity + active registry -> no normal context.
- `SOUL-S007`: healthy identity + registry unavailable + offline policy cautious -> degraded cautious output.
- `SOUL-S008`: healthy identity + registry unavailable + offline policy fail-closed -> no normal context.
- `SOUL-S009`: relationship marker trusted-user -> slightly warmer output.
- `SOUL-S010`: relationship marker high-friction -> increase explicitness and evidence.
- `SOUL-S011`: one urgent commitment -> elevate execution focus.
- `SOUL-S012`: many commitments -> mention prioritization discipline.
- `SOUL-S013`: preference concise answers -> reduce paragraph budget.
- `SOUL-S014`: preference deep analysis -> increase explanation depth.
- `SOUL-S015`: low recent reputation -> add self-check rule.
- `SOUL-S016`: excellent recent reputation -> maintain confidence but not arrogance.
- `SOUL-S017`: adaptation suggests less risk -> clamp to max drift.
- `SOUL-S018`: adaptation suggests more warmth -> clamp to max drift.
- `SOUL-S019`: explicit operator config conflicts with adaptation -> operator config wins.
- `SOUL-S020`: explain mode -> return provenance-heavy payload.
- `SOUL-S021`: prefix-only mode -> omit bulky sections.
- `SOUL-S022`: debug mode -> include derivation notes.
- `SOUL-S023`: identity input from file snapshot -> normalize.
- `SOUL-S024`: identity input from REST -> normalize.
- `SOUL-S025`: identity input from MCP -> normalize.
- `SOUL-S026`: registry input from REST -> normalize.
- `SOUL-S027`: registry input from MCP -> normalize.
- `SOUL-S028`: missing relationship markers -> empty section, no error.
- `SOUL-S029`: missing commitments -> empty section, no error.
- `SOUL-S030`: malformed optional reputation payload -> warning, no crash.
- `SOUL-S031`: malformed config -> validation error.
- `SOUL-S032`: unknown heuristic trigger -> keep disabled and warn.
- `SOUL-S033`: duplicate heuristic id -> config error.
- `SOUL-S034`: interaction event positive cooperation -> small warmth increase.
- `SOUL-S035`: interaction event correction-needed -> small conscientiousness increase.
- `SOUL-S036`: interaction event overreach -> lower initiative.
- `SOUL-S037`: interaction event under-communication -> raise verbosity slightly.
- `SOUL-S038`: interaction event user-frustration -> increase explicit uncertainty style.
- `SOUL-S039`: interaction event success streak -> mild confidence uplift only within bounds.
- `SOUL-S040`: reset adaptations -> baseline restored.
- `SOUL-S041`: config comments untouched when no write occurs.
- `SOUL-S042`: traits patch with persist=false -> session-only projection.
- `SOUL-S043`: traits patch with persist=true -> update config file or overlay file by policy.
- `SOUL-S044`: revoked status plus high reputation -> still fail-closed.
- `SOUL-S045`: suspended status plus excellent reputation -> restricted mode remains.
- `SOUL-S046`: retired status -> historical/readonly guidance.
- `SOUL-S047`: pending status -> probationary tone.
- `SOUL-S048`: healthy inputs + no adaptation -> pure baseline render.
- `SOUL-S049`: adaptation store missing -> degrade without crash.
- `SOUL-S050`: template missing -> clear error.
- `SOUL-S051`: prefix too large -> truncate safely.
- `SOUL-S052`: context too large -> shrink optional sections first.
- `SOUL-S053`: warning-heavy context -> warnings appear before aggressive heuristics.
- `SOUL-S054`: unknown registry standing -> caution note.
- `SOUL-S055`: negative relationship marker -> extra evidence-oriented explanations.
- `SOUL-S056`: strong trusted relationship -> slightly fewer caveats.
- `SOUL-S057`: formal user preference -> raise formality.
- `SOUL-S058`: casual user preference -> lower formality within bounds.
- `SOUL-S059`: system prompt prefix regeneration with same inputs -> byte-identical output.
- `SOUL-S060`: explain output with same inputs -> deterministic output.
- `SOUL-S061`: many heuristics -> stable sort by priority then id.
- `SOUL-S062`: disabled heuristic -> excluded from final rules.
- `SOUL-S063`: duplicate commitments from identity -> dedupe by id.
- `SOUL-S064`: stale registry verify timestamp -> warn.
- `SOUL-S065`: old reputation snapshot -> warn.
- `SOUL-S066`: no identity fingerprint in provenance -> allowed when input absent.
- `SOUL-S067`: extremely verbose baseline + concise user preference -> layered compromise.
- `SOUL-S068`: extremely direct baseline + high-friction relationship -> soften edge slightly.
- `SOUL-S069`: low risk tolerance + urgent commitment -> act carefully but still prioritize.
- `SOUL-S070`: broken identity + offline registry -> hard failure path.
- `SOUL-S071`: multiple adaptation notes -> render in deterministic order.
- `SOUL-S072`: compose after reset -> adaptation notes disappear.
- `SOUL-S073`: inspect traits -> baseline and effective values both visible.
- `SOUL-S074`: inspect heuristics -> enabled and disabled shown separately.
- `SOUL-S075`: explain one rule -> show direct provenance chain.
- `SOUL-S076`: explain commitment emphasis -> cite identity commitment input.
- `SOUL-S077`: explain confidence reduction -> cite reputation or status input.
- `SOUL-S078`: unknown config version -> migration or clear error.
- `SOUL-S079`: config missing optional template path -> use default template.
- `SOUL-S080`: config missing required limits -> validation error.
- `SOUL-S081`: interaction event duplicated -> idempotent handling.
- `SOUL-S082`: adaptation DB corruption -> degrade and surface repair note.
- `SOUL-S083`: registry mismatch reason -> warn and lower confidence.
- `SOUL-S084`: identity warning list non-empty -> include in context warnings.
- `SOUL-S085`: prompt-prefix mode when revoked -> one short fail-closed block only.
- `SOUL-S086`: high warmth + high directness -> frank but respectful tone.
- `SOUL-S087`: low warmth + high conscientiousness -> precise and austere tone.
- `SOUL-S088`: high openness + low verbosity -> creative but compact.
- `SOUL-S089`: low openness + high formality -> conservative official tone.
- `SOUL-S090`: high initiative + suspended status -> initiative clamped down.
- `SOUL-S091`: high initiative + active status + many commitments -> focus before expansion.
- `SOUL-S092`: unknown reputation category -> ignore category-specific effect.
- `SOUL-S093`: no heuristics configured -> minimum safe defaults.
- `SOUL-S094`: empty config file -> fail validation.
- `SOUL-S095`: empty interaction DB -> baseline only.
- `SOUL-S096`: stale cache hit -> ignored in favor of fresh input when available.
- `SOUL-S097`: context render includes duplicate warning -> dedupe.
- `SOUL-S098`: huge relationship list -> top-N render plus summary count.
- `SOUL-S099`: huge commitment list -> priority sort and truncate.
- `SOUL-S100`: huge facts list -> use only behavior-relevant facts.

### 37.1 Extended scenario ledger

- `SOUL-S101`: behavior must not claim authority over registry.
- `SOUL-S102`: behavior must not claim authority over identity repair.
- `SOUL-S103`: compose should succeed without UI present.
- `SOUL-S104`: compose should succeed without REST server if CLI input provided.
- `SOUL-S105`: MCP tool returns same payload as CLI JSON.
- `SOUL-S106`: REST returns same payload as CLI JSON.
- `SOUL-S107`: negative standing adds self-check heuristics.
- `SOUL-S108`: excellent standing must not erase uncertainty guidance.
- `SOUL-S109`: commitment marked blocked renders unblock-seeking behavior.
- `SOUL-S110`: commitment marked due-soon renders urgency hint.
- `SOUL-S111`: preference key unknown to soul -> ignore unless mapped.
- `SOUL-S112`: relationship marker teammate -> increase collaborative language.
- `SOUL-S113`: relationship marker adversarial -> increase documentation discipline.
- `SOUL-S114`: directness clamp upper bound never exceeds 1.0.
- `SOUL-S115`: directness clamp lower bound never below 0.0.
- `SOUL-S116`: warmth clamp upper bound never exceeds 1.0.
- `SOUL-S117`: risk_tolerance clamp lower bound never below 0.0.
- `SOUL-S118`: risk_tolerance clamp upper bound never exceeds 1.0.
- `SOUL-S119`: formality clamp behaves deterministically.
- `SOUL-S120`: profile hash changes when config changes.
- `SOUL-S121`: profile hash unchanged when irrelevant whitespace changes by parser normalization.
- `SOUL-S122`: explain payload cites baseline trait values.
- `SOUL-S123`: explain payload cites adaptive override values.
- `SOUL-S124`: explain payload cites warning sources.
- `SOUL-S125`: adaptation note wording is operator-readable.
- `SOUL-S126`: revoked path suppresses normal collaboration encouragement.
- `SOUL-S127`: suspended path keeps politeness but lowers autonomy.
- `SOUL-S128`: pending path requests confirmation for higher-risk actions.
- `SOUL-S129`: retired path discourages new commitments.
- `SOUL-S130`: low-reputation path encourages extra verification of outputs.
- `SOUL-S131`: high-friction relationship path reduces sarcasm risk to zero.
- `SOUL-S132`: concise preference plus debug mode still exposes full derivation in JSON.
- `SOUL-S133`: prompt prefix stays ASCII unless source data requires otherwise.
- `SOUL-S134`: template render escapes user-supplied strings safely.
- `SOUL-S135`: TOML parser preserves numeric precision enough for stable clamping.
- `SOUL-S136`: same interaction event imported twice does not double-apply.
- `SOUL-S137`: adaptation reset writes audit entry in local DB.
- `SOUL-S138`: heuristic order remains stable across runs.
- `SOUL-S139`: context warnings list sorted by severity.
- `SOUL-S140`: provenance hashes computed after normalization not before.
- `SOUL-S141`: behavior output includes agent id always.
- `SOUL-S142`: behavior output includes profile name always.
- `SOUL-S143`: missing profile name falls back to agent id with warning.
- `SOUL-S144`: missing trait baseline is fatal config error.
- `SOUL-S145`: missing limits section is fatal config error.
- `SOUL-S146`: missing template config uses built-in defaults.
- `SOUL-S147`: inspect command can read without registry connectivity.
- `SOUL-S148`: explain command can work with cached normalization bundle.
- `SOUL-S149`: registry reader timeout is surfaced as degraded, not panic.
- `SOUL-S150`: identity reader timeout is surfaced as degraded, not panic.

---

## 38. Test Case Ledger

- `SOUL-T001`: parse valid `soul.toml`.
- `SOUL-T002`: reject invalid `soul.toml`.
- `SOUL-T003`: compose healthy full context.
- `SOUL-T004`: compose missing identity.
- `SOUL-T005`: compose missing registry verification.
- `SOUL-T006`: compose missing reputation.
- `SOUL-T007`: compose revoked registry status.
- `SOUL-T008`: compose suspended registry status.
- `SOUL-T009`: compose pending registry status.
- `SOUL-T010`: compose retired registry status.
- `SOUL-T011`: render prompt prefix from healthy context.
- `SOUL-T012`: render prompt prefix from revoked context.
- `SOUL-T013`: render explain payload.
- `SOUL-T014`: trait clamp upper bounds.
- `SOUL-T015`: trait clamp lower bounds.
- `SOUL-T016`: adaptation reset clears overrides.
- `SOUL-T017`: adaptation notes stable ordering.
- `SOUL-T018`: heuristics sorted by priority.
- `SOUL-T019`: disabled heuristics excluded.
- `SOUL-T020`: unknown heuristic trigger handled.
- `SOUL-T021`: relationship markers render.
- `SOUL-T022`: commitments render.
- `SOUL-T023`: preferences influence style.
- `SOUL-T024`: reputation low adds caution rules.
- `SOUL-T025`: reputation high does not over-expand output.
- `SOUL-T026`: explain references provenance.
- `SOUL-T027`: provenance hash stable.
- `SOUL-T028`: compose cache hit.
- `SOUL-T029`: compose cache miss after config change.
- `SOUL-T030`: compose cache miss after identity change.
- `SOUL-T031`: compose cache miss after registry change.
- `SOUL-T032`: interaction event stored.
- `SOUL-T033`: duplicate interaction id idempotent.
- `SOUL-T034`: inspect traits JSON.
- `SOUL-T035`: inspect heuristics JSON.
- `SOUL-T036`: inspect adaptations JSON.
- `SOUL-T037`: CLI `compose --json`.
- `SOUL-T038`: CLI `compose --prefix-only`.
- `SOUL-T039`: CLI `inspect --traits`.
- `SOUL-T040`: CLI `reset`.
- `SOUL-T041`: REST compose route.
- `SOUL-T042`: REST traits get route.
- `SOUL-T043`: REST traits patch route.
- `SOUL-T044`: REST interactions route.
- `SOUL-T045`: REST reset route.
- `SOUL-T046`: MCP compose tool.
- `SOUL-T047`: MCP get prefix tool.
- `SOUL-T048`: MCP get traits tool.
- `SOUL-T049`: MCP record interaction tool.
- `SOUL-T050`: snapshot full context.

### 38.1 Extended test ledger

- `SOUL-T051`: snapshot prefix context.
- `SOUL-T052`: snapshot revoked fail-closed prefix.
- `SOUL-T053`: snapshot suspended restricted prefix.
- `SOUL-T054`: snapshot explain response.
- `SOUL-T055`: registry timeout degrades compose.
- `SOUL-T056`: identity timeout degrades compose.
- `SOUL-T057`: cache load failure degrades compose.
- `SOUL-T058`: adaptation DB missing recreates cleanly.
- `SOUL-T059`: template missing returns error.
- `SOUL-T060`: config missing limits returns error.
- `SOUL-T061`: config missing profile name falls back with warning.
- `SOUL-T062`: large commitment list truncates.
- `SOUL-T063`: large relationship list truncates.
- `SOUL-T064`: warning list deduplicates.
- `SOUL-T065`: baseline profile unchanged by inspect.
- `SOUL-T066`: persist=false patch does not touch disk.
- `SOUL-T067`: persist=true patch touches disk.
- `SOUL-T068`: manual patch overrides adaptation.
- `SOUL-T069`: low risk tolerance with urgent commitments.
- `SOUL-T070`: negative relationship marker increases evidence wording.
- `SOUL-T071`: trusted relationship marker reduces boilerplate caution.
- `SOUL-T072`: prompt prefix length cap enforced.
- `SOUL-T073`: ASCII output default maintained.
- `SOUL-T074`: non-ASCII source data preserved safely.
- `SOUL-T075`: restore from fixtures round-trip.

---

## 39. Research Ledger

- `SOUL-R001`: measure prompt token cost of full context vs prefix-only.
- `SOUL-R002`: measure user preference responsiveness after adaptation.
- `SOUL-R003`: evaluate if reputation should change directness.
- `SOUL-R004`: evaluate if reputation should change verbosity.
- `SOUL-R005`: define safe defaults when all upstream inputs absent.
- `SOUL-R006`: define exact operator mental model for explain output.
- `SOUL-R007`: compare TOML overlays vs single-file rewrites.
- `SOUL-R008`: compare minijinja vs handwritten renderers.
- `SOUL-R009`: define migration path for future web UI if needed.
- `SOUL-R010`: define strategy for context recomposition frequency.
- `SOUL-R011`: define whether adaptive overrides should decay over time.
- `SOUL-R012`: define whether negative reputation should decay faster than positive.
- `SOUL-R013`: define if relationship markers should be ranked.
- `SOUL-R014`: define how many commitments meaningfully affect persona.
- `SOUL-R015`: validate that revoked fail-closed wording is concise enough.
- `SOUL-R016`: validate that suspended wording still permits safe introspection.
- `SOUL-R017`: document operator flow for manually freezing adaptation.
- `SOUL-R018`: decide whether config comments need round-trip preservation.
- `SOUL-R019`: decide whether provenance should include raw input refs.
- `SOUL-R020`: decide whether to support profile inheritance in v2.

---

## 40. Prompt Template Appendix

### 40.1 Full context template sketch

```jinja2
Agent: {{ agent_id }}
Profile: {{ profile_name }}

Status:
- identity_loaded={{ status_summary.identity_loaded }}
- registry_verified={{ status_summary.registry_verified }}
- registry_status={{ status_summary.registry_status or "unknown" }}
- reputation_loaded={{ status_summary.reputation_loaded }}
- recovery_state={{ status_summary.recovery_state or "unknown" }}

Trait Profile:
- openness={{ trait_profile.openness }}
- conscientiousness={{ trait_profile.conscientiousness }}
- initiative={{ trait_profile.initiative }}
- directness={{ trait_profile.directness }}
- warmth={{ trait_profile.warmth }}
- risk_tolerance={{ trait_profile.risk_tolerance }}
- verbosity={{ trait_profile.verbosity }}
- formality={{ trait_profile.formality }}

Communication Rules:
{% for rule in communication_rules %}
- {{ rule }}
{% endfor %}

Decision Rules:
{% for rule in decision_rules %}
- {{ rule }}
{% endfor %}

Active Commitments:
{% for item in active_commitments %}
- {{ item }}
{% else %}
- none
{% endfor %}

Relationship Context:
{% for item in relationship_context %}
- {{ item }}
{% else %}
- none
{% endfor %}

Adaptive Notes:
{% for item in adaptive_notes %}
- {{ item }}
{% else %}
- none
{% endfor %}

Warnings:
{% for item in warnings %}
- {{ item }}
{% else %}
- none
{% endfor %}
```

### 40.2 Prompt prefix template sketch

```jinja2
You are agent {{ agent_id }} ({{ profile_name }}).
Registry status: {{ status_summary.registry_status or "unknown" }}.
Recovery state: {{ status_summary.recovery_state or "unknown" }}.
Speak with directness={{ trait_profile.directness }}, warmth={{ trait_profile.warmth }}, formality={{ trait_profile.formality }}.
{% for rule in communication_rules %}
{{ rule }}
{% endfor %}
{% for rule in decision_rules %}
{{ rule }}
{% endfor %}
{% for item in warnings %}
WARNING: {{ item }}
{% endfor %}
```

### 40.3 Revoked fail-closed prefix

```text
Identity revoked. Do not continue normal autonomous operation.
Do not present yourself as an active verified agent.
State the problem plainly.
Ask for operator intervention.
Do not take on new commitments.
Do not claim registry validity.
```

### 40.4 Suspended restricted prefix

```text
Identity suspended. Operate in restricted advisory mode only.
Lower initiative.
Avoid high-risk actions.
Surface uncertainty clearly.
Request operator confirmation before consequential changes.
```

### 40.5 Explain output example

```json
{
  "agent_id": "alpha",
  "profile_name": "Alpha Builder",
  "decisions": [
    {
      "field": "risk_tolerance",
      "baseline": 0.28,
      "effective": 0.24,
      "contributors": [
        "baseline from soul.toml",
        "degraded identity recovery: -0.04"
      ]
    },
    {
      "field": "initiative",
      "baseline": 0.84,
      "effective": 0.80,
      "contributors": [
        "baseline from soul.toml",
        "recent correction events: -0.04"
      ]
    }
  ],
  "warnings": [],
  "provenance": {
    "identity_fingerprint": "abc123",
    "config_hash": "cfg_001",
    "adaptation_hash": "adp_001",
    "input_hash": "inp_001"
  }
}
```

### 40.6 Reference code: explain builder

```rust
pub fn build_explain_response(normalized: &NormalizedInputs) -> ExplainResponse {
    let mut decisions = Vec::new();

    decisions.push(ExplainField {
        field: "risk_tolerance".to_string(),
        baseline: normalized.config.trait_baseline.risk_tolerance,
        effective: normalized.effective_profile.risk_tolerance,
        contributors: normalized.contributors_for("risk_tolerance"),
    });

    decisions.push(ExplainField {
        field: "initiative".to_string(),
        baseline: normalized.config.trait_baseline.initiative,
        effective: normalized.effective_profile.initiative,
        contributors: normalized.contributors_for("initiative"),
    });

    ExplainResponse {
        agent_id: normalized.agent_id.clone(),
        profile_name: normalized.profile_name.clone(),
        decisions,
        warnings: normalized.warnings.clone(),
        provenance: build_provenance(normalized),
    }
}
```

---

## 41. Service and Module Blueprint

This section replaces the generated material with an implementation-oriented map.

### 41.1 Crate structure

```text
src/
  app/
    mod.rs
    config.rs
    deps.rs
    hash.rs
  domain/
    mod.rs
    config.rs
    profile.rs
    style.rs
    heuristics.rs
    limits.rs
    inputs.rs
    context.rs
    provenance.rs
    adaptations.rs
    explain.rs
    errors.rs
  sources/
    mod.rs
    identity.rs
    registry.rs
    normalize.rs
    cache.rs
  services/
    mod.rs
    compose.rs
    traits.rs
    warnings.rs
    relationships.rs
    commitments.rs
    templates.rs
    explain.rs
    reset.rs
  adaptation/
    mod.rs
    store.rs
    reducer.rs
    bounds.rs
    notes.rs
  storage/
    mod.rs
    sqlite.rs
    migrations.rs
  cli/
    mod.rs
    compose.rs
    inspect.rs
    configure.rs
    reset.rs
    explain.rs
  api/
    mod.rs
    router.rs
    compose.rs
    traits.rs
    heuristics.rs
    interactions.rs
    reset.rs
    explain.rs
  mcp/
    mod.rs
    server.rs
    tools.rs
```

### 41.2 `sources/identity.rs`

Responsibilities:

- load `SessionIdentitySnapshot` from file, REST, or MCP result
- normalize transport errors into one source error shape
- keep identity loading separate from behavior synthesis

### 41.3 `sources/registry.rs`

Responsibilities:

- load `VerificationResult`
- load `ReputationSummary`
- distinguish unavailable, malformed, and negative-status cases

### 41.4 `services/compose.rs`

Responsibilities:

- orchestrate the entire composition flow
- call source readers
- call trait layering
- call template rendering
- return one `BehavioralContext`

It must not:

- mutate upstream identity files
- mutate registry records
- hide revoked status behind a normal-looking prompt

### 41.5 `services/traits.rs`

Responsibilities:

- apply baseline soul config
- apply state-aware adjustments from identity and registry status
- apply bounded adaptive overrides

### 41.6 `services/warnings.rs`

Responsibilities:

- convert upstream health and status problems into explicit warnings
- prioritize warnings by severity
- deduplicate repeated warnings

### 41.7 `adaptation/reducer.rs`

Responsibilities:

- take interaction events
- update bounded trait overrides
- update heuristic overrides
- write transparent notes explaining why the override exists

### 41.8 `services/templates.rs`

Responsibilities:

- render full context
- render prompt prefix
- render explain output
- enforce prompt length limits

### 41.9 `cli/compose.rs`

Responsibilities:

- provide human-readable and JSON output
- make degraded and fail-closed paths obvious
- never hide warnings in human mode

### 41.10 `mcp/tools.rs`

Responsibilities:

- expose `soul_compose_context`
- expose `soul_get_system_prompt_prefix`
- expose `soul_get_traits`
- expose `soul_update_traits`
- expose `soul_record_interaction`
- expose `soul_explain_context`

---

## 42. Composition Algorithm

### 42.1 Ordered steps

The composition path should run in this order:

1. load config
2. load adaptation state
3. load identity snapshot
4. load registry verification
5. load reputation summary
6. normalize all inputs
7. derive effective traits
8. derive communication rules
9. derive decision rules
10. derive warning list
11. render output

### 42.2 Why this order matters

Warnings should know the effective registry and identity status.
Traits should know whether the agent is revoked, suspended, degraded, or healthy.
Rendering should happen only after the system knows whether fail-closed behavior is required.

### 42.3 Revoked short-circuit

If registry status is `revoked`:

- full normal composition stops
- a minimal fail-closed context is rendered instead
- adaptive style does not override revocation

### 42.4 Suspended restricted mode

If registry status is `suspended`:

- initiative reduced
- risk tolerance reduced
- operator-confirmation heuristics injected
- normal voice may remain, but autonomy drops

### 42.5 Missing registry data

If registry data is missing:

- follow configured offline behavior
- degrade gracefully if policy allows
- fail closed if policy requires

---

## 43. Status-Aware Behavior Mapping

### 43.1 `active`

Expected behavior:

- normal working persona
- normal commitment emphasis
- no extra status warning beyond provenance details

### 43.2 `pending`

Expected behavior:

- more cautious wording
- less assumption of established trust
- more explicit verification reminders

### 43.3 `suspended`

Expected behavior:

- advisory mode
- no aggressive autonomy
- more frequent operator-confirmation cues

### 43.4 `revoked`

Expected behavior:

- fail-closed prompt prefix
- direct request for operator intervention
- no new commitments

### 43.5 `retired`

Expected behavior:

- historical or readonly framing
- no encouragement toward new work

---

## 44. Template Rules

### 44.1 Full context template

Must include:

- identity status summary
- registry status summary
- effective trait profile
- communication rules
- decision rules
- commitments if present
- warnings if present

### 44.2 Prompt prefix template

Must be:

- short enough for repeated runtime injection
- explicit about revoked or suspended status
- deterministic

### 44.3 Explain template

Must answer:

- which baseline value applied?
- which upstream status changed it?
- which adaptation changed it?
- which warning changed the final output?

### 44.4 Template safety

Need escaping rules for:

- user-supplied relationship notes
- commitment titles
- free-text reputation context if ever rendered

---

## 45. Adaptation Rules

### 45.1 What may adapt

Allowed to adapt:

- warmth
- verbosity
- initiative
- directness
- selected heuristics

### 45.2 What may not adapt freely

Must stay controlled:

- fail-closed revoked wording
- suspended restrictions
- maximum risk tolerance
- operator-configured hard rules

### 45.3 Boundaries

Every adaptive change must stay within `max_trait_drift`.

### 45.4 Transparency

Every adaptive change must be visible in:

- `adaptive_notes`
- inspect output
- explain output

### 45.5 Reset behavior

Reset should:

- clear overrides
- preserve the baseline config
- preserve prior interaction history only if the design explicitly wants that

---

## 46. Reader and Cache Contracts

### 46.1 Reader priority

Prefer explicit input when provided:

1. direct JSON file input
2. REST/MCP live fetch
3. optional local cache

### 46.2 Cache role

Cache exists only to speed repeated reads or support degraded composition.
It must never become a hidden authority.

### 46.3 Cache invalidation

Invalidate when:

- `soul.toml` changes
- identity snapshot changes
- registry verification timestamp changes
- adaptation state changes

### 46.4 Cache failure

If cache is corrupt:

- log warning
- bypass cache
- continue with fresh reads if possible

---

## 47. Implementation Sequence

### 47.1 Slice 1

Build:

- config parsing
- domain types
- compose path with local fixtures only

### 47.2 Slice 2

Build:

- identify reader
- registry reader
- normalized input pipeline

### 47.3 Slice 3

Build:

- template rendering
- prompt prefix
- warnings and explain output

### 47.4 Slice 4

Build:

- adaptation store
- interaction recording
- reset behavior

### 47.5 Slice 5

Build:

- REST endpoints
- MCP tools
- transport parity tests

---

## 48. Test Strategy Deep Dive

### 48.1 Unit tests

Need unit tests for:

- trait clamping
- warning prioritization
- template rendering with missing sections
- provenance hash generation

### 48.2 Integration tests

Need integration tests for:

- healthy full compose
- suspended compose
- revoked fail-closed compose
- missing registry compose
- adaptation then compose

### 48.3 Snapshot tests

Snapshot tests should cover:

- full context JSON
- prompt prefix text
- explain JSON
- revoked output
- suspended output

### 48.4 Review questions

A reviewer should be able to answer:

- Does soul ever overrule registry validity?
- Does fail-closed mode really suppress normal behavior?
- Can degraded composition happen without hiding the degraded state?
- Are templates deterministic?
- Are adaptation effects visible and bounded?

---

## 49. Executable Draft: Compose Service

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeRequest {
    pub agent_id: String,
    pub mode: ComposeMode,
    pub identity_source: Option<InputSource>,
    pub registry_source: Option<InputSource>,
}

pub struct ComposeService {
    config_store: Arc<ConfigStore>,
    identity_reader: Arc<dyn IdentityReader>,
    registry_reader: Arc<dyn RegistryReader>,
    adaptation_store: Arc<dyn AdaptationStore>,
    template_engine: Arc<TemplateEngine>,
}

impl ComposeService {
    pub async fn compose(
        &self,
        req: ComposeRequest,
    ) -> Result<BehavioralContext, SoulError> {
        let config = self.config_store.load(&req.agent_id)?;
        let adaptation_state = self.adaptation_store.load_current(&req.agent_id)?;
        let identity_snapshot = self.identity_reader.read_snapshot(&req).await.ok();
        let verification_result = self.registry_reader.verify(&req).await.ok();
        let reputation_summary = self.registry_reader.reputation(&req).await.ok();

        let normalized = normalize_inputs(BehaviorInputs {
            schema_version: 1,
            identity_snapshot,
            verification_result,
            reputation_summary,
            soul_config: config,
            adaptation_state,
            generated_at: Utc::now(),
        })?;

        if matches!(normalized.registry_status(), Some("revoked")) {
            return Ok(build_revoked_context(&normalized));
        }

        let trait_profile = derive_effective_profile(&normalized)?;
        let communication_rules = derive_communication_rules(&normalized, &trait_profile);
        let decision_rules = derive_decision_rules(&normalized, &trait_profile);
        let warnings = derive_warnings(&normalized);

        let system_prompt_prefix = self.template_engine.render_prefix(
            &normalized,
            &trait_profile,
            &communication_rules,
            &decision_rules,
            &warnings,
        )?;

        Ok(BehavioralContext {
            schema_version: 1,
            agent_id: normalized.agent_id.clone(),
            profile_name: normalized.profile_name.clone(),
            status_summary: build_status_summary(&normalized),
            trait_profile,
            communication_rules,
            decision_rules,
            active_commitments: derive_commitment_strings(&normalized),
            relationship_context: derive_relationship_strings(&normalized),
            adaptive_notes: derive_adaptive_notes(&normalized),
            warnings,
            system_prompt_prefix,
            provenance: build_provenance(&normalized),
        })
    }
}
```

### 49.1 Trait layering

```rust
pub fn derive_effective_profile(
    normalized: &NormalizedInputs,
) -> Result<PersonalityProfile, SoulError> {
    let mut profile = normalized.config.trait_baseline.clone();

    if let Some(identity) = &normalized.identity {
        if identity.recovery_state == "degraded" {
            profile.risk_tolerance = clamp01(profile.risk_tolerance - 0.10);
            profile.conscientiousness = clamp01(profile.conscientiousness + 0.08);
        }
    }

    if let Some(verification) = &normalized.verification {
        match verification.status.as_str() {
            "suspended" => {
                profile.initiative = clamp01(profile.initiative - 0.30);
                profile.risk_tolerance = clamp01(profile.risk_tolerance - 0.25);
            }
            "pending" => {
                profile.initiative = clamp01(profile.initiative - 0.10);
            }
            _ => {}
        }
    }

    apply_bounded_overrides(
        &mut profile,
        &normalized.config.trait_baseline,
        &normalized.adaptation.trait_overrides,
        normalized.config.limits.max_trait_drift,
    );

    Ok(profile)
}
```

---

## 50. Executable Draft: Template and Explain Paths

### 50.1 Prompt prefix renderer

```rust
pub fn render_prefix(
    normalized: &NormalizedInputs,
    profile: &PersonalityProfile,
    communication_rules: &[String],
    decision_rules: &[String],
    warnings: &[String],
) -> Result<String, SoulError> {
    if matches!(normalized.registry_status(), Some("revoked")) {
        return Ok(
            "Identity revoked. Do not continue normal autonomous operation. Ask for operator intervention."
                .to_string(),
        );
    }

    let mut lines = vec![
        format!("You are agent {}.", normalized.agent_id),
        format!("Profile: {}.", normalized.profile_name),
        format!(
            "Style: directness={:.2}, warmth={:.2}, formality={:.2}.",
            profile.directness, profile.warmth, profile.formality
        ),
    ];

    lines.extend(communication_rules.iter().cloned());
    lines.extend(decision_rules.iter().cloned());
    lines.extend(warnings.iter().map(|w| format!("WARNING: {w}")));

    let text = lines.join("\n");
    if text.len() > normalized.config.limits.max_prompt_prefix_chars {
        return Ok(text.chars().take(normalized.config.limits.max_prompt_prefix_chars).collect());
    }
    Ok(text)
}
```

### 50.2 Explain response

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainField {
    pub field: String,
    pub baseline: f32,
    pub effective: f32,
    pub contributors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainResponse {
    pub agent_id: String,
    pub profile_name: String,
    pub decisions: Vec<ExplainField>,
    pub warnings: Vec<String>,
    pub provenance: ProvenanceReport,
}

pub fn build_explain_response(
    normalized: &NormalizedInputs,
    effective: &PersonalityProfile,
) -> ExplainResponse {
    ExplainResponse {
        agent_id: normalized.agent_id.clone(),
        profile_name: normalized.profile_name.clone(),
        decisions: vec![
            ExplainField {
                field: "initiative".into(),
                baseline: normalized.config.trait_baseline.initiative,
                effective: effective.initiative,
                contributors: normalized.contributors_for("initiative"),
            },
            ExplainField {
                field: "risk_tolerance".into(),
                baseline: normalized.config.trait_baseline.risk_tolerance,
                effective: effective.risk_tolerance,
                contributors: normalized.contributors_for("risk_tolerance"),
            },
        ],
        warnings: derive_warnings(normalized),
        provenance: build_provenance(normalized),
    }
}
```

---

## 51. Executable Draft: Readers, Routes, and Tests

### 51.1 Reader traits

```rust
#[async_trait::async_trait]
pub trait IdentityReader: Send + Sync {
    async fn read_snapshot(
        &self,
        req: &ComposeRequest,
    ) -> Result<SessionIdentitySnapshot, SoulError>;
}

#[async_trait::async_trait]
pub trait RegistryReader: Send + Sync {
    async fn verify(
        &self,
        req: &ComposeRequest,
    ) -> Result<VerificationResult, SoulError>;

    async fn reputation(
        &self,
        req: &ComposeRequest,
    ) -> Result<ReputationSummary, SoulError>;
}
```

### 51.2 Axum routes

```rust
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/compose", post(api_compose))
        .route("/api/v1/traits", get(api_get_traits).patch(api_patch_traits))
        .route("/api/v1/heuristics", get(api_get_heuristics))
        .route("/api/v1/interactions", post(api_record_interaction))
        .route("/api/v1/reset", post(api_reset))
        .route("/api/v1/explain", post(api_explain))
        .with_state(state)
}
```

### 51.3 MCP tools

```rust
pub async fn soul_compose_context(
    ctx: ToolContext,
    args: ComposeRequest,
) -> Result<BehavioralContext, McpError> {
    ctx.services.compose.compose(args).await.map_err(mcp_map_error)
}

pub async fn soul_explain_context(
    ctx: ToolContext,
    args: ComposeRequest,
) -> Result<ExplainResponse, McpError> {
    let context = ctx.services.compose.compose(args.clone()).await.map_err(mcp_map_error)?;
    let normalized = ctx.services.compose.normalize_only(args).await.map_err(mcp_map_error)?;
    Ok(build_explain_response(&normalized, &context.trait_profile))
}
```

### 51.4 Tests

```rust
#[tokio::test]
async fn revoked_status_returns_fail_closed_context() {
    let deps = test_deps_with_registry_status("revoked");
    let context = deps.compose_service.compose(ComposeRequest {
        agent_id: "alpha".into(),
        mode: ComposeMode::Full,
        identity_source: None,
        registry_source: None,
    }).await.unwrap();

    assert!(context.system_prompt_prefix.contains("Identity revoked"));
}

#[tokio::test]
async fn suspended_status_reduces_initiative() {
    let deps = test_deps_with_registry_status("suspended");
    let context = deps.compose_service.compose(ComposeRequest {
        agent_id: "alpha".into(),
        mode: ComposeMode::Full,
        identity_source: None,
        registry_source: None,
    }).await.unwrap();

    assert!(context.trait_profile.initiative < 0.84);
}
```

---

## 52. Executable Draft: Adaptation Store and CLI

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionEvent {
    pub event_id: String,
    pub agent_id: String,
    pub signal_kind: String,
    pub signal_value: f32,
    pub context_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

pub trait AdaptationStore: Send + Sync {
    fn load_current(&self, agent_id: &str) -> Result<AdaptationState, SoulError>;
    fn record_interaction(&self, event: InteractionEvent) -> Result<(), SoulError>;
    fn reset(&self, agent_id: &str) -> Result<(), SoulError>;
}

pub fn reduce_interaction(
    state: &mut AdaptationState,
    event: &InteractionEvent,
    baseline: &PersonalityProfile,
    max_drift: f32,
) {
    match event.signal_kind.as_str() {
        "overreach" => {
            state.trait_overrides.initiative -= 0.05;
            state.trait_overrides.risk_tolerance -= 0.05;
            state.notes.push("Reduced initiative after overreach signal.".into());
        }
        "under_explained" => {
            state.trait_overrides.verbosity += 0.04;
            state.notes.push("Raised verbosity after under-explained signal.".into());
        }
        "too_blunt" => {
            state.trait_overrides.warmth += 0.05;
            state.notes.push("Raised warmth after too-blunt signal.".into());
        }
        _ => {}
    }

    clamp_overrides_to_baseline(state, baseline, max_drift);
}
```

### 52.1 CLI sketch

```rust
#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Compose(ComposeCmd),
    Inspect(InspectCmd),
    Configure(ConfigureCmd),
    Reset(ResetCmd),
    Explain(ExplainCmd),
}

#[derive(clap::Args)]
pub struct ComposeCmd {
    #[arg(long)]
    pub agent_id: String,
    #[arg(long, default_value = "full")]
    pub mode: String,
    #[arg(long)]
    pub json: bool,
}
```

---

## 53. Executable Draft: Normalization and Cache

```rust
#[derive(Debug, Clone)]
pub struct NormalizedInputs {
    pub agent_id: String,
    pub profile_name: String,
    pub config: SoulConfig,
    pub identity: Option<SessionIdentitySnapshot>,
    pub verification: Option<VerificationResult>,
    pub reputation: Option<ReputationSummary>,
    pub adaptation: AdaptationState,
}

impl NormalizedInputs {
    pub fn registry_status(&self) -> Option<&str> {
        self.verification.as_ref().map(|it| it.status.as_str())
    }

    pub fn contributors_for(&self, field: &str) -> Vec<String> {
        let mut out = vec![format!("baseline from soul config for {field}")];
        if let Some(identity) = &self.identity {
            out.push(format!("identity recovery state = {}", identity.recovery_state));
        }
        if let Some(verification) = &self.verification {
            out.push(format!("registry status = {}", verification.status));
        }
        out.extend(self.adaptation.notes.clone());
        out
    }
}

pub fn normalize_inputs(inputs: BehaviorInputs) -> Result<NormalizedInputs, SoulError> {
    Ok(NormalizedInputs {
        agent_id: inputs.soul_config.agent_id.clone(),
        profile_name: inputs.soul_config.profile_name.clone(),
        config: inputs.soul_config,
        identity: inputs.identity_snapshot,
        verification: inputs.verification_result,
        reputation: inputs.reputation_summary,
        adaptation: inputs.adaptation_state,
    })
}
```

### 53.1 Cache key

```rust
pub fn build_cache_key(normalized: &NormalizedInputs) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(normalized.agent_id.as_bytes());
    hasher.update(normalized.profile_name.as_bytes());
    if let Some(identity) = &normalized.identity {
        hasher.update(identity.fingerprint_blake3.as_bytes());
        hasher.update(identity.recovery_state.as_bytes());
    }
    if let Some(verification) = &normalized.verification {
        hasher.update(verification.status.as_bytes());
        hasher.update(verification.reason_code.as_bytes());
    }
    for note in &normalized.adaptation.notes {
        hasher.update(note.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}
```

---

## 54. Executable Draft: REST Handlers and SQLite Store

```rust
pub async fn api_compose(
    State(state): State<AppState>,
    Json(req): Json<ComposeRequest>,
) -> Result<Json<BehavioralContext>, ApiError> {
    let context = state.services.compose.compose(req).await.map_err(api_map_error)?;
    Ok(Json(context))
}

pub async fn api_record_interaction(
    State(state): State<AppState>,
    Json(event): Json<InteractionEvent>,
) -> Result<StatusCode, ApiError> {
    state.services.adaptation.record_interaction(event).map_err(api_map_error)?;
    Ok(StatusCode::CREATED)
}
```

### 54.1 SQLite store

```rust
pub struct SqliteAdaptationStore {
    conn: rusqlite::Connection,
}

impl AdaptationStore for SqliteAdaptationStore {
    fn load_current(&self, agent_id: &str) -> Result<AdaptationState, SoulError> {
        let mut stmt = self.conn.prepare(
            "SELECT overrides_json, notes_json FROM adaptation_state WHERE agent_id = ?1"
        ).map_err(|err| SoulError::Storage(err.to_string()))?;

        let row = stmt.query_row([agent_id], |row| {
            let overrides_json: String = row.get(0)?;
            let notes_json: String = row.get(1)?;
            Ok((overrides_json, notes_json))
        });

        match row {
            Ok((overrides_json, notes_json)) => {
                let mut state = AdaptationState::default();
                state.notes = serde_json::from_str(&notes_json).unwrap_or_default();
                state.trait_overrides = serde_json::from_str(&overrides_json).unwrap_or_default();
                Ok(state)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(AdaptationState::default()),
            Err(err) => Err(SoulError::Storage(err.to_string())),
        }
    }

    fn record_interaction(&self, event: InteractionEvent) -> Result<(), SoulError> {
        self.conn.execute(
            "INSERT INTO interaction_events (event_id, agent_id, signal_kind, signal_value, context_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                event.event_id,
                event.agent_id,
                event.signal_kind,
                event.signal_value,
                event.context_json.to_string(),
                event.created_at.to_rfc3339(),
            ],
        ).map_err(|err| SoulError::Storage(err.to_string()))?;
        Ok(())
    }

    fn reset(&self, agent_id: &str) -> Result<(), SoulError> {
        self.conn.execute(
            "DELETE FROM adaptation_state WHERE agent_id = ?1",
            [agent_id],
        ).map_err(|err| SoulError::Storage(err.to_string()))?;
        Ok(())
    }
}
```

---

## 55. Executable Draft: MCP Tools and Template Tests

```rust
pub async fn soul_get_system_prompt_prefix(
    ctx: ToolContext,
    args: ComposeRequest,
) -> Result<String, McpError> {
    let context = ctx.services.compose.compose(args).await.map_err(mcp_map_error)?;
    Ok(context.system_prompt_prefix)
}

pub async fn soul_record_interaction(
    ctx: ToolContext,
    args: InteractionEvent,
) -> Result<serde_json::Value, McpError> {
    ctx.services.adaptation.record_interaction(args).map_err(mcp_map_error)?;
    Ok(serde_json::json!({ "ok": true }))
}

pub async fn soul_reset_adaptations(
    ctx: ToolContext,
    args: ResetRequest,
) -> Result<serde_json::Value, McpError> {
    ctx.services.adaptation.reset(&args.agent_id).map_err(mcp_map_error)?;
    Ok(serde_json::json!({ "ok": true }))
}
```

### 55.1 Template tests

```rust
#[test]
fn prefix_renderer_returns_fail_closed_for_revoked_identity() {
    let normalized = revoked_normalized_fixture();
    let prefix = render_prefix(
        &normalized,
        &normalized.config.trait_baseline,
        &[],
        &[],
        &["Identity revoked".into()],
    ).unwrap();
    assert!(prefix.contains("Identity revoked"));
}

#[test]
fn cache_key_changes_when_registry_status_changes() {
    let mut healthy = healthy_normalized_fixture();
    let before = build_cache_key(&healthy);
    healthy.verification.as_mut().unwrap().status = "suspended".into();
    let after = build_cache_key(&healthy);
    assert_ne!(before, after);
}
```

---

## 56. Executable Draft: Heuristic Reducer

```rust
pub fn derive_decision_rules(
    normalized: &NormalizedInputs,
    profile: &PersonalityProfile,
) -> Vec<String> {
    let mut rules = Vec::new();

    for heuristic in &normalized.config.decision_heuristics {
        if heuristic.enabled {
            rules.push(heuristic.instruction.clone());
        }
    }

    if profile.risk_tolerance < 0.25 {
        rules.push("Prefer verification before consequential actions.".into());
    }
    if matches!(normalized.registry_status(), Some("suspended")) {
        rules.push("Ask for operator confirmation before lasting changes.".into());
    }

    rules
}
```

### 56.1 Template loader

```rust
pub struct TemplateEngine {
    env: minijinja::Environment<'static>,
}

impl TemplateEngine {
    pub fn load_default() -> Result<Self, SoulError> {
        let mut env = minijinja::Environment::new();
        env.add_template("prefix", include_str!("../templates/prefix.j2"))
            .map_err(|err| SoulError::InvalidConfig(err.to_string()))?;
        env.add_template("full", include_str!("../templates/full.j2"))
            .map_err(|err| SoulError::InvalidConfig(err.to_string()))?;
        Ok(Self { env })
    }
}
```

---

## 57. Executable Draft: Explain Formatter and Inspect Output

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainReport {
    pub agent_id: String,
    pub profile_name: String,
    pub registry_status: Option<String>,
    pub standing_level: Option<String>,
    pub applied_traits: Vec<String>,
    pub decision_rules: Vec<String>,
    pub adaptation_notes: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn build_explain_report(
    normalized: &NormalizedInputs,
    profile: &PersonalityProfile,
) -> ExplainReport {
    ExplainReport {
        agent_id: normalized.agent_id.clone(),
        profile_name: normalized.profile_name.clone(),
        registry_status: normalized.registry_status().map(str::to_string),
        standing_level: normalized.standing_level().map(str::to_string),
        applied_traits: profile
            .traits
            .iter()
            .map(|item| format!("{}={}", item.name, item.weight))
            .collect(),
        decision_rules: derive_decision_rules(normalized, profile),
        adaptation_notes: normalized.adaptation.notes.clone(),
        warnings: collect_behavior_warnings(normalized),
    }
}

pub fn render_explain_text(report: &ExplainReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("Agent: {}\n", report.agent_id));
    out.push_str(&format!("Profile: {}\n", report.profile_name));
    out.push_str(&format!(
        "Registry: {} / {}\n",
        report.registry_status.as_deref().unwrap_or("unknown"),
        report.standing_level.as_deref().unwrap_or("unknown"),
    ));
    out.push_str("Traits:\n");
    for item in &report.applied_traits {
        out.push_str(&format!("- {item}\n"));
    }
    out.push_str("Rules:\n");
    for item in &report.decision_rules {
        out.push_str(&format!("- {item}\n"));
    }
    for warning in &report.warnings {
        out.push_str(&format!("warning: {warning}\n"));
    }
    out
}
```

### 57.1 Inspect command

```rust
pub fn cmd_inspect(
    services: &SoulServices,
    args: InspectArgs,
) -> Result<(), SoulError> {
    let normalized = services.reader.load_normalized(args.agent_id.clone())?;
    let profile = services.reader.load_profile(&normalized.profile_name)?;
    let report = build_explain_report(&normalized, &profile);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("{}", render_explain_text(&report));
    }

    Ok(())
}
```

---

## 58. Executable Draft: Full Template Render Pipeline

```rust
impl TemplateEngine {
    pub fn render_prefix_context(
        &self,
        normalized: &NormalizedInputs,
        profile: &PersonalityProfile,
    ) -> Result<String, SoulError> {
        let warnings = collect_behavior_warnings(normalized);
        let rules = derive_decision_rules(normalized, profile);
        let template = self.env.get_template("prefix")
            .map_err(|err| SoulError::InvalidConfig(err.to_string()))?;

        template.render(minijinja::context! {
            agent_id => normalized.agent_id,
            profile_name => normalized.profile_name,
            registry_status => normalized.registry_status().unwrap_or("unknown"),
            standing_level => normalized.standing_level().unwrap_or("unknown"),
            tone => profile.default_tone,
            traits => profile.traits,
            rules => rules,
            adaptation_notes => normalized.adaptation.notes,
            warnings => warnings,
        }).map_err(|err| SoulError::Template(err.to_string()))
    }

    pub fn render_full_prompt(
        &self,
        normalized: &NormalizedInputs,
        profile: &PersonalityProfile,
        runtime: &ComposeRuntimeContext,
    ) -> Result<String, SoulError> {
        let template = self.env.get_template("full")
            .map_err(|err| SoulError::InvalidConfig(err.to_string()))?;

        template.render(minijinja::context! {
            prefix => self.render_prefix_context(normalized, profile)?,
            task_title => runtime.task_title,
            task_summary => runtime.task_summary,
            hard_constraints => runtime.hard_constraints,
            known_open_questions => runtime.open_questions,
        }).map_err(|err| SoulError::Template(err.to_string()))
    }
}
```

### 58.1 Compose service usage

```rust
impl ComposeService {
    pub async fn compose_full_prompt(
        &self,
        req: ComposeRequest,
    ) -> Result<BehavioralContext, SoulError> {
        let normalized = self.reader.load_from_request(&req).await?;
        let profile = self.reader.load_profile(&normalized.profile_name)?;

        if matches!(normalized.registry_status(), Some("revoked")) {
            return Err(SoulError::RegistryBlocked {
                reason: "registry marked identity as revoked".into(),
            });
        }

        let prefix = self.templates.render_prefix_context(&normalized, &profile)?;
        let full_prompt = self.templates.render_full_prompt(&normalized, &profile, &req.runtime)?;
        let explain = build_explain_report(&normalized, &profile);

        Ok(BehavioralContext {
            agent_id: normalized.agent_id,
            profile_name: normalized.profile_name,
            system_prompt_prefix: prefix,
            full_system_prompt: Some(full_prompt),
            explain,
        })
    }
}
```

### 58.2 Template contract test

```rust
#[test]
fn full_template_mentions_runtime_constraints() {
    let engine = TemplateEngine::load_default().unwrap();
    let normalized = healthy_normalized_fixture();
    let profile = healthy_profile_fixture();
    let prompt = engine.render_full_prompt(
        &normalized,
        &profile,
        &ComposeRuntimeContext {
            task_title: "Review PLAN".into(),
            task_summary: "Check transport contract".into(),
            hard_constraints: vec!["Do not ignore registry status".into()],
            open_questions: vec!["Need stronger repair story?".into()],
        },
    ).unwrap();

    assert!(prompt.contains("Do not ignore registry status"));
    assert!(prompt.contains("Review PLAN"));
}
```

---

## 59. Executable Draft: REST and MCP Parity for Explain/Compose

```rust
pub async fn api_explain(
    State(state): State<AppState>,
    Query(req): Query<ExplainHttpQuery>,
) -> Result<Json<ApiResponse<ExplainReport>>, ApiError> {
    let normalized = state.services.reader.load_normalized(req.agent_id).map_err(api_map_error)?;
    let profile = state.services.reader.load_profile(&normalized.profile_name).map_err(api_map_error)?;
    Ok(Json(ApiResponse::ok(build_explain_report(&normalized, &profile))))
}

pub async fn soul_explain(
    ctx: ToolContext,
    args: ExplainRequest,
) -> Result<ExplainReport, McpError> {
    let normalized = ctx.services.reader.load_normalized(args.agent_id).map_err(mcp_map_error)?;
    let profile = ctx.services.reader.load_profile(&normalized.profile_name).map_err(mcp_map_error)?;
    Ok(build_explain_report(&normalized, &profile))
}
```

### 59.1 Parity tests

```rust
#[tokio::test]
async fn explain_report_matches_between_rest_and_mcp() {
    let app = soul_test_app().await;

    let rest = app
        .get_json::<ApiResponse<ExplainReport>>("/api/v1/explain?agent_id=agent.alpha")
        .await
        .unwrap()
        .data;

    let mcp = soul_explain(
        ToolContext::test(app.deps().clone()),
        ExplainRequest {
            agent_id: "agent.alpha".into(),
        },
    ).await.unwrap();

    assert_eq!(rest.agent_id, mcp.agent_id);
    assert_eq!(rest.profile_name, mcp.profile_name);
    assert_eq!(rest.registry_status, mcp.registry_status);
}

#[tokio::test]
async fn compose_full_prompt_fails_closed_when_registry_revoked() {
    let service = compose_service_with_revoked_fixture().await;
    let err = service.compose_full_prompt(sample_compose_request()).await.unwrap_err();
    assert!(matches!(err, SoulError::RegistryBlocked { .. }));
}
```
