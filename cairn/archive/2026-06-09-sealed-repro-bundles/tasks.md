# Tasks: sealed-repro-bundles

- [x] [serial] r[molten.testing.sealed_repro_bundles.schema] Extend report repro bundles with seal metadata, embedded gate receipt, artifact refs, and seal checks.
- [x] [serial] r[molten.testing.sealed_repro_bundles.export] Make CLI repro export emit sealed bundles and write the embedded report gate receipt.
- [x] [serial] r[molten.testing.sealed_repro_bundles.validation] Validate report/suite/state/effect-log/gate evidence refs and embedded receipt refs while parsing sealed bundles.
- [x] [serial] r[molten.testing.sealed_repro_bundles.gate] Require sealed bundle gate checks to recompute and match the embedded report gate receipt before accepting the bundle as pass evidence.
- [x] [parallel] r[molten.testing.sealed_repro_bundles.negative_tests] Add negative tests for tampered embedded reports, tampered embedded receipts, and mismatched suite refs.
- [x] [parallel] r[molten.testing.sealed_repro_bundles.failure_diagnostics] Keep failure repro bundles diagnostic-only.
- [x] [parallel] r[molten.testing.sealed_repro_bundles.docs] Update CLI/docs/Cairn evidence notes.
