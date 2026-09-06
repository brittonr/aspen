# Implementation checkpoint — service proof blocked

Implementation commit: `c56b2b1840ca0cdd4486da4e7dfa4037c4811aa9`. A subsequent guard closes the production synthetic-startup path. The change remains open.

## Completed checks

- Policy migration: 3 regression tests, including actual compatible-Cairn graph/path/missing-field and unknown-profile rejection.
- Content core: 11 tests.
- Atomic namespace: 2 tests, including concurrent complete-snapshot readback.
- Node-host dependency boundary: 5 tests.
- Production startup/content guard: 4 tests. The real CLI rejects before state creation with and without a workspace manifest.
- Selected CLI Clippy and core/node-host all-target Clippy passed with `-D warnings`.
- Nickel export matches checked JSON. A valid exemption and 14 invalid policy fixtures passed their expected outcomes. Content config and duplicate-reader rejection passed.
- Producer proposal/design/tasks gates and repository validation passed structurally. Runtime/closeout tasks remain open.

The migration preserves the committed runtime policy, not the stale authored projection. Trust hashes, gate thresholds, workflow choices, replay cases, and prior receipt fields stay unchanged. Required traceability/task metadata is added. Five absent receipt-schema rows return without pruning their existing references.

The new atomic operation uses the already-locked `cap-tempfile` dependency. Root and node-host manifests promote it from test-only to runtime use. Both lockfiles and all dependency revisions remain unchanged.

## Failed VM attempt

`vm-blocked.json` identifies the exact attempted source and binary. Two client identity boots completed. The storage VM prepared the exact 119321-byte archive and canonical manifest. Its normal `node run` then reached `synthetic_clean_octet_gate_receipt_for_tests()` and failed while reading `/Cargo.toml`.

No content listener started. No archive transfer or native replay completed. Supplying the older Onix adapter's minimal manifest would activate fake clean evidence and is not an admissible fix.

Production startup and protected serving now fail closed without a real source-gate admission route. Unit-test startup fixtures remain test-only. The consumer runner also stops before VM creation, with no override.

## Scope

Rust 1.97.1 already existed. Its focused checks are not the exact `nightly-2026-05-26`/Octet gate. A bounded search found no matching dated toolchain directory in the local Nix store; this does not establish availability on every machine.

No Stage0, Darkhttpd/Mantle rebuild, package-toolchain rebuild, host blob service, physical deployment, or release/default promotion occurred. Existing example-driver proof remains separate. Private VM disks, images, keys, handoffs, and raw serial logs remain outside Git. Review was single-agent, not independent assurance.
