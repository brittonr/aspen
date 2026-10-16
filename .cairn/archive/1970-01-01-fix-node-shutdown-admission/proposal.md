## Why

F01 identifies shutdown effects before request admission at source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

An active local node accepts a queued shutdown request with `authority_refs=[]`. Dispatch writes passing adapter and node shutdown receipts. Control admission then returns `deny`, but the handler removes the active lock. Expected behavior preserves the active state and performs no shutdown effects.

The audit evidence is static control-flow analysis, not an executed shutdown. The full audit reports eight executed bugs and six static findings. F01 belongs to the static group. Its core baseline recorded 359 passing tests, but those tests do not reproduce F01.

## What Changes

- Admit shutdown requests before adapter shutdown, shutdown receipt publication, or active-lock removal.
- Keep denied requests separate from admitted shutdown effect plans.
- Record observed effect outcomes without converting denial or partial failure into successful shutdown.
- Add normal repository regressions for local stop and queued dispatch.

## Impact

- Current consumer: local `molten node` stop and node-control dispatch.
- Maintenance owner: Molten node-runtime maintainers, with node-host maintainers for capability filesystem adapters.
- Source scope: `src/node/parts/daemon/p018/body.rs`, `p027/body.rs`, and `p029/body.rs`.
- Spec delta: `node-runtime`, under this package only.
- Durable capability: repeatable admission-before-effect tests and an explicit shutdown decision boundary.
- Repeatability evidence: planned pure decision tests and controlled filesystem tests in the normal test suite.

## Non-goals and authority

This proposal grants no implementation permission. It does not establish remote exploitability, complete capability validation, live adapter shutdown, or release readiness. Existing authority gates remain required. F14 separately owns historical shutdown observations after restart. No accepted spec or metadata changes belong to this planning work.
