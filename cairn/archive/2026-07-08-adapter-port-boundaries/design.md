## Context

The architecture already separates policy/evidence from adapters, but many practical workflows combine validation, planning, persistence, and transport. A modular design needs the pure core to decide what should happen and the shell to do it only after explicit pass evidence.

## Design

### Port model

Each side-effect family should be represented by a narrow port boundary:

- ledger/artifact store port for read/write/import plans;
- blob/chunk store port for manifest and byte movement;
- transport port for publish, receive, and delivery evidence;
- executor port for hostcall and sandbox execution;
- policy/evidence port for external gate inputs;
- clock/seed/effect-log port for replay-sensitive observations.

Ports may be Rust traits, command structs, or deterministic plan/result records. The key property is that pure logic can be tested with in-memory inputs and planned outputs without performing the effect.

### Admission before effects

Pure planners should return a decision and a bounded list of planned adapter operations. Shells must execute only pass decisions and must emit deny evidence without mutating stores or transports.

### Replay evidence

Adapter results should be captured as canonical receipts or effect-log entries so deterministic replay can distinguish planned effects, performed effects, denied effects, and externally unavailable effects.

### Test strategy

Positive tests should show admitted plans produce the expected planned operations. Negative tests should show missing authority, stale evidence, malformed inputs, resource denial, or unsupported adapter capability produces no planned mutation or transport operation.

## Non-goals

- Do not replace existing adapters wholesale.
- Do not make trait mocks the primary proof surface when pure plan records are sufficient.
- Do not treat transport identity, storage presence, or executor availability as authority.
