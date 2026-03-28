# OpenClaw Reference Notes

## Why OpenClaw Matters

OpenClaw already contains practical patterns for behavior shaping:

- assistant identity presentation
- system prompt layering
- startup reminders
- compaction-aware context recovery

## Relevant References

- `references/openclaw-main/docs/concepts/system-prompt.md`
- `references/openclaw-main/docs/concepts/agent-loop.md`
- `references/openclaw-main/src/agents/prompt-composition-scenarios.ts`
- `references/openclaw-main/src/agents/system-prompt.test.ts`
- `references/openclaw-main/src/gateway/assistant-identity.ts`

## Patterns To Reuse

- layered prompt composition
- explicit startup instructions after compaction
- separation between identity presentation and deeper runtime state

## Patterns To Change

- this project should isolate behavior policy from identity storage
- this project should formalize the contract for proposing identity updates
