# Architecture

## Goal

`agents-soul` maps identity into behavior.

Inputs:

- session identity snapshot
- active task context
- workflow state

Outputs:

- tone and style guidance
- value priorities
- interaction constraints
- identity-event proposals

## Core Boundary

Identity says who the agent is.

Soul says how that agent tends to speak, decide, and act.

## Main Subsystems

### Soul profile

- declared values
- response style
- relationship stance
- tolerance for ambiguity

### Behavior policy

- response shaping
- escalation rules
- caution thresholds
- decision heuristics

### Identity feedback

- preference learned
- value conflict detected
- commitment suggested
- repair concern raised

## Non-Goals For v1

- distributed multi-soul composition
- independent continuity storage
- provider-specific prompt engines
