## Why

Molten's runtime and harness laws depend on source-level discipline: pure core functions must stay free of ambient effects, adapters must expose explicit effect/evidence boundaries, capability refs must not collapse into untyped strings, harness fixtures must not become private backdoors, and golden/replay drift must be reviewed. Runtime checks and replay catch many problems, but they run after code has already compiled and often after the mistake has entered a critical path.

Octet and Valence can provide an earlier static/evidence rail. Octet checks source shape, boundaries, resource discipline, and reviewable suppressions. Valence function objects/fingerprints identify selected source surfaces with caveats. Molten should use those artifacts as fail-closed evidence inputs to the harness, Cairn receipts, and CI/release/admission gates.

Octet/Valence evidence is bounded. It can say that a source shape, fingerprint, caveat set, and review record were checked. It must not be treated as proof of semantic correctness, formal verification, cross-toolchain determinism, or complete effect absence.

## What Changes

- Define Octet/Valence as a source/evidence enforcement rail for Molten, not the runtime policy engine.
- Gate pure core transition functions against ambient filesystem, network, wall-clock, entropy, process, unsafe, panic, unwrap/expect, and unreviewed scripting effects.
- Gate adapter boundary functions so they declare their effect surface, route through Molten effect manifests, and emit trace/receipt evidence.
- Gate authority-bearing boundary APIs against stringly ids, refs, capabilities, secrets, receipts, schemas, and effect logs.
- Gate harness code against direct runtime-store mutation, private backdoors, invisible fixture mutation, or production-available test bypasses.
- Gate production/test separation so test-only capabilities and debug hooks are feature/profile/policy isolated.
- Gate secret/capability rendering so sensitive refs cannot leak through debug/export/report surfaces without redaction policy.
- Gate resource discipline for unbounded loops, queues, deferred work, trace growth, and missing budget checkpoints.
- Treat Valence function-object/fingerprint drift on critical surfaces as requiring harness replay, adapter conformance, golden update, migration, or review receipts before acceptance.
- Require harness reports and Cairn receipts to reference Octet artifacts, Valence function objects, caveats, suppressions, and review manifests where source gates were part of the evidence.

## Impact

This gives Molten a static preflight rail before deterministic harness execution. The archived milestone defines the Octet config/CI command shape, critical source-surface evidence model, canonical Octet artifact imports, strict gate receipts, source-gate validation receipts, warning quarantine/review receipts, downstream consumer binding, and remediation-plan surface inventory. Marker attributes and broader custom lint coverage may evolve, but the completed scope already fails closed for missing/stale/tampered Octet artifacts, warning-only strict runs, missing object-corpus/fingerprint evidence, quarantine-only evidence in strict consumers, and unreviewed critical findings.
