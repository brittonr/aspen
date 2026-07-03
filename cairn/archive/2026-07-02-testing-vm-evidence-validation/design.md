## Context

`nixos-vm-multinode` currently exercises the right platform boundary: packaged Molten under systemd in multiple NixOS VMs, cross-node control workflow behavior, restart durability, and production-shaped soak evidence. The current gate still relies on in-test `grep` assertions for receipt marker kinds, and the derivation output can be empty even when the VM produced useful evidence inside guest state roots. That makes the check hard to audit after the build and leaves room for marker-only false positives.

## Decisions

### 1. Validate VM evidence with a pure semantic core

**Choice:** Add a deterministic validator that accepts parsed canonical VM evidence values plus expected topology inputs and returns a structured pass/deny result with diagnostics.

**Rationale:** Receipt validation should be testable without launching VMs. The NixOS test driver and filesystem copying remain the imperative shell; semantic checks over receipt values stay pure.

### 2. Preserve evidence through a manifest-backed Nix output

**Choice:** Copy canonical VM evidence receipts and diagnostic logs from guest-accessible locations into the Nix check output, alongside a manifest that binds paths, content refs, receipt kinds, and evidence-only caveats.

**Rationale:** Release review should be able to inspect the realized check output without replaying the VM run or scraping truncated build logs.

### 3. Require negative fixtures before trusting the validator

**Choice:** Add fixtures that alter decision status, topology membership, child refs, replay status, and receipt bytes, and assert fail-closed diagnostics.

**Rationale:** Marker existence is too weak for release evidence. Negative fixtures prove the gate rejects realistic corruptions before a VM pass can satisfy higher-level review.

## Risks / Trade-offs

- Preserving logs may increase Nix output size; keep logs bounded and classify them as diagnostic refs.
- Some VM evidence may include runtime-specific paths or store refs; the validator must compare declared refs and explicit volatile fields, not ambient formatting.
- The VM test should remain an explicit heavy gate, not a hidden dependency of fast local checks.
