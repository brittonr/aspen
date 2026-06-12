# Design: deterministic replay fixture

## Fixture scope

The first replay fixture is local and bounded. It records a deterministic in-process run with a fixed artifact ref, dependency-closure ref, initial-state ref, handler profile, policy/schema refs, seed or effect-log ref, and runtime/tool version refs. The fixture may use a vat or two-actor local run as its first consumer, but the replay contract is runtime-level and not specific to vats.

This change does not introduce distributed replay, production network replay, transcript rendering, or cache admission. Those integrations consume this evidence later.

## Record artifact

A recorded fixture stores canonical Preserves records for:

- `deterministic-run-identity-v1`: artifact refs, dependency closure, initial state, schema refs, policy refs, capability/revocation refs, handler profile ref, seed or effect-log ref, runtime/tool version refs, and optional scenario label.
- `deterministic-turn-journal-v1`: turn id, cause/parent id, actor/session/vat ids, scheduler key, input ref, before-state ref, effect request/response refs, policy/receipt refs, committed action refs, output refs, and after-state ref.
- `deterministic-effect-log-v1`: ordered request/response pairs with handler profile id, effect kind, sequence, request ref, response ref or denial ref, and safe diagnostics.
- `deterministic-fixture-record-v1`: the run identity plus the ordered journal and effect-log refs needed for replay.

All refs are canonical content refs over Preserves values or authenticated snapshot refs. Rendered text is diagnostic only.

## Replay verification

Replay verification loads the fixture record and installs the replay handler profile. For each recorded turn it:

1. selects the next admitted event by canonical scheduler key;
2. compares the input/event ref;
3. runs until each effect request;
4. compares the request ref and injects the recorded response;
5. compares policy decisions, committed action refs, receipt refs, output refs, and after-state ref;
6. stops immediately on the first mismatch.

The verifier emits `deterministic-replay-verify-v1` with `pass` only when all checked boundaries match. Any denial emits a receipt with the first-divergence ref.

## First divergence evidence

`deterministic-first-divergence-v1` identifies the first semantic boundary that changed, not downstream fallout. It records divergence kind, turn id, actor/session/vat id, log position, handler profile ref, expected ref, actual ref when safe, and redacted diagnostics when the boundary may contain secrets or capabilities.

Supported initial divergence kinds are: scheduler, input, effect-request, effect-response, policy-decision, action, receipt, output, and state-hash.

## CLI contract

The implementation slice should expose fixture-oriented commands, naming can evolve with the existing `molten test` tree:

```sh
molten test replay-fixture record --out target/replay.preserves
molten test replay-fixture verify target/replay.preserves --receipt-out target/replay.verify.preserves
molten test replay-fixture tamper target/replay.preserves --kind effect-response --out target/replay.tampered.preserves
molten test replay-fixture show target/replay.verify.preserves
```

The commands are evidence producers and validators. They must not grant authority or bypass normal runtime policy, capability, resource, source-gate, provenance, or transport gates.

## Validation approach

Tests should cover:

- unchanged fixture replay passes and reproduces final state/output refs;
- changed identity input reports an identity/input divergence;
- changed effect response reports an effect-response divergence;
- changed policy/receipt boundary reports policy or receipt divergence;
- replay profile denies live external effects;
- show/readback parses the produced Preserves records without trusting rendered text.
