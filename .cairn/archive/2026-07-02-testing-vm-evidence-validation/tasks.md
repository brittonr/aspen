## Phase 1: Semantic validator

- [x] [serial] r[molten.testing.vm_evidence.semantic_validation] Add a pure VM evidence validator for topology, node evidence, test-run, replay, child refs, diagnostics, decisions, and caveats.
- [x] [parallel] r[molten.testing.vm_evidence.negative_fixtures] Add positive and negative fixtures for valid, missing, stale, tampered, wrong-topology, wrong-decision, and incomplete-child-ref VM evidence.

## Phase 2: Nix output evidence

- [x] [serial] r[molten.testing.vm_evidence.artifact_preservation] Update the NixOS VM test to preserve canonical evidence receipts and diagnostic logs in the check output with a manifest.
- [x] [serial] r[molten.testing.vm_evidence.log_boundary] Bind terminal, QEMU, and systemd logs as diagnostic evidence rather than authoritative pass evidence.

## Phase 3: End-to-end validation and docs

- [x] [parallel] r[molten.testing.vm_evidence.docs] Document how to inspect the VM evidence output and which receipts are authoritative.
- [x] [serial] [depends:molten.testing.vm_evidence.semantic_validation] Run focused validator tests, `nix build .#checks.x86_64-linux.nixos-vm-multinode --no-link -L`, and Cairn validation gates.
