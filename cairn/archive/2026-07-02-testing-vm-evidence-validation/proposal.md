## Why

The multi-node NixOS VM check now proves that the VM script completes and that expected receipt marker strings exist in VM-local files, but the realized Nix output does not preserve the VM evidence for later release review and the gate does not semantically validate the receipt contents. Release and pilot decisions need durable canonical evidence: topology, node identities, child workflow refs, replay status, diagnostics, and caveats must be parsed and bound rather than inferred from terminal output.

## What Changes

- Preserve VM test evidence as explicit Nix output artifacts with a canonical manifest.
- Add semantic validation for `nixos-vm-topology-v1`, `nixos-vm-node-evidence-v1`, `nixos-vm-test-run-v1`, and production-soak receipts.
- Validate decisions, topology membership, node ids, state roots, Nix store refs, child receipt refs, replay status, diagnostics, and evidence-only caveats.
- Add negative coverage for missing, stale, tampered, wrong-topology, wrong-decision, and incomplete-child-ref VM evidence.
- Keep QEMU/systemd logs diagnostic-only by binding them through receipt refs or manifest entries instead of treating logs as primary pass evidence.

## Impact

- **Files**: `flake.nix`, VM test helpers, VM evidence receipt parsing/validation code, CLI or library tests, docs/README evidence notes.
- **Testing**: focused unit tests for the semantic validator, negative fixture tests for malformed VM evidence, and `nix build .#checks.x86_64-linux.nixos-vm-multinode --no-link -L` as the end-to-end gate.
