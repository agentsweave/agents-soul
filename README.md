# agents-soul

`agents-soul` is the behavior and personality layer for the agent ecosystem.

It answers the question: given a reconstructed identity, how should this agent
behave?

## Mission

- Define values, tone, and behavioral tendencies.
- Convert identity state into response policy.
- Detect preference and value signals during interaction.
- Propose identity-relevant updates back to `agents-identify`.

## v1 Focus

- Consume a session identity snapshot from `agents-identify`.
- Produce behavior policy for use by workflows and future runtimes.
- Keep behavior separate from continuity-of-self.

## Relationship To Other Projects

- `agents-identify` owns continuity and durable selfhood.
- `agents-workflow` owns lifecycle and orchestration.
- `references/openclaw-main` is the main behavioral reference.
