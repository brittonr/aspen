## Why

Replay evidence is present in many Molten paths, but operators need a single canonical view of which subsystems are actually covered by fresh run, replay run, second fresh run, negative tamper, and release-bound index evidence. Without a replay coverage/readiness matrix, gaps can hide behind passing local tests or broad release receipts.

## What Changes

- Add a replay coverage matrix that summarizes replay readiness by subsystem and workflow.
- Require deterministic smoke suites to declare replay eligibility, positive replay evidence, negative tamper evidence, and release/readiness caveats.
- Cover representative subsystem paths: harness report replay, node-control workflow bundle, job worker scheduling, coordination duplicate operations, remote dataspace delivery logs, vat replay, retention remote-clearance, and dogfood release replay evidence.
- Add read-only CLI/catalog output for replay readiness that remains evidence-only and cannot replace individual replay receipts or gates.

## Impact

- **Files**: testing hardening/replay-smoke helpers, subsystem fixtures, dogfood/readiness receipts, catalog classifications, docs, and tests.
- **Testing**: positive smoke suites, negative non-replayable or tampered evidence cases, coverage matrix validation, and release-readiness readback.
- **Boundaries**: readiness summaries are review evidence only and do not grant replay pass status, authority, policy, provenance, transport, source-gate, resource, retention, release, or execution trust.
