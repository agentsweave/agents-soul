# Repository Guidelines

## Purpose

This repo owns agent behavior policy, not identity continuity.

- Build the behavior layer as a consumer of identity state.
- Keep persona, values, and style explicit and inspectable.
- Route durable identity updates through `agents-identify`.

## Architecture Rules

- `agents-soul` must not become a second identity store.
- Behavior outputs should be deterministic from inputs where possible.
- Value conflicts and preference ambiguity should be surfaced, not hidden.

## v1 Defaults

- identity-driven behavior
- local-first
- CLI-first integration
- prompt/policy outputs before heavy runtime code

## Reference

Use `../references/openclaw-main` as the main reference for:

- system prompt composition
- assistant identity presentation
- loop behavior
- session recovery guidance
