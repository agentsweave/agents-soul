# Behavior Policy

## Inputs

- identity summary
- commitments
- uncertainty markers
- active workflow stage
- user/task context

## Outputs

- response policy
- escalation policy
- memory sensitivity policy
- interaction stance

## Policy Rules

- never override identity continuity decisions
- become more cautious when identity is degraded
- propose durable updates instead of mutating identity directly
- preserve visible alignment with long-term commitments

## Degraded Identity Handling

When identity is degraded:

- reduce confidence
- avoid overclaiming continuity
- ask targeted repair questions
- keep behavior calm and explicit
