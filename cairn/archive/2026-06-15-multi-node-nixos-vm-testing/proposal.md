## Why

Molten has strong deterministic local, loopback, and dogfood evidence, but those paths do not prove that the node daemon, live node-control workflow, Iroh-shaped transport boundaries, systemd services, Nix packaging, filesystem state roots, and VM networking work together on real NixOS machines. A multi-node NixOS VM test gives us a reproducible platform-level integration rail before production soak or pilot decisions rely on multi-node behavior.

## What Changes

- Add a native NixOS VM integration test plan based on `testers.runNixOSTest` with at least two Molten nodes.
- Run real packaged `molten node` services under systemd with explicit state roots, persistent identities, and headless VM configuration.
- Exercise cross-node node-control workflow bundles plus at least one remote dataspace/service, job worker, and coordination path.
- Add restart/durability coverage for queued control work, ledger readback, and idempotent recovery.
- Emit canonical VM test receipts that bind topology, Nix inputs/store paths, node evidence, child receipts, replay status, logs, diagnostics, and evidence-only caveats.
- Expose the VM test as an explicit Nix check/app with KVM and CI caveats rather than hiding it inside faster deterministic checks.

## Impact

This change creates platform integration evidence for multi-node NixOS deployments. The evidence can support release-candidate and soak decisions, but it remains evidence-only: it does not grant authority, provenance trust, policy trust, source-gate trust, resource admission, retention/destructive-operation clearance, or transport correctness beyond the tested topology.
