## Why

Issue #57: the coordination `rate-limit` service counts acquisitions per key and never resets them, so it behaves as a lifetime quota while its name and documentation suggest a time-window rate. The issue's TLA+ model (`CoordinationRate.tla`) shows a key stays denied in every later state.

## What Changes

- State the actual contract: `rate-limit` is a per-key acquisition quota for the lifetime of the coordination state. Coordination has no admitted logical time source, so no window resets it.
- The denial diagnostic says so (`rate-limit quota exhausted; it does not reset`).
- Wire, manifest and receipt names keep `rate-limit` for compatibility; no schema changes.

## Impact

- **Files**: coordination rate acquisition and tests, the coordination protocol reference, and the coordination specification.
- **Testing**: quota exhaustion persists across later committed operations on another key; each key keeps its own quota; the generated coordination trace keeps its deny step.
- **Owner / consumer**: the coordination control-plane services. A windowed rate limit needs an admitted logical time source in coordination state first; it is not added here.
