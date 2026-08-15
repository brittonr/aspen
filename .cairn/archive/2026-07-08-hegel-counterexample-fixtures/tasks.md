# Tasks: hegel-counterexample-fixtures

## Artifact model

- [x] [serial] r[molten.testing.hegel_counterexample.replay_fixture] Define canonical Hegel counterexample fixture artifacts with property id, generator profile ref, generation seed, shrink path, shrunk input, replay identity, trace refs, receipt refs, and diagnostics.
- [x] [parallel] r[molten.testing.hegel_counterexample.redaction] Bind confidentiality metadata so sensitive generated inputs are redacted or encrypted before export.

## Promotion flow

- [x] [serial] r[molten.testing.hegel_counterexample.promotion] Add a reviewed promotion path from counterexample fixture to deterministic regression case or known-deny fixture.
- [x] [parallel] r[molten.testing.hegel_counterexample.replay_fixture] Add replay command/readback support so a counterexample can run without invoking the generator.

## Tests and validation

- [x] [parallel] r[molten.testing.hegel_counterexample.replay_fixture] Add positive fixture validation and negative fixtures for missing seed, missing shrink path, stale replay identity, malformed Preserves input, and missing diagnostics.
- [x] [parallel] r[molten.testing.hegel_counterexample.promotion] Add positive promotion tests and negative tests for promotion without review evidence or with mismatched property ids.
- [x] [serial] r[molten.testing.hegel_counterexample.redaction] Run focused property/counterexample tests and Cairn validation; record any Hegel integration limitations.
