## Why

The node-replication pilot says it uses the reviewed Octet production Verus profile, but its Nix module reconstructs the Verus and Rust packages locally. That duplication makes the pilot's toolchain closure differ from Octet even when the verifier binary and version strings match.

## What Changes

- Add a revision-pinned Octet toolchain flake input while retaining the separate Octet Rust source input.
- Consume Octet's profile and production verifier packages in the compatibility probe.
- Remove the pilot-owned Rust and Verus package derivations.
- Rebind the deterministic blocked decision to the Octet revision, profile digest, and package output.

## Impact

- **Files**: `flake.nix`, `flake.lock`, the node-replication Nix module, profile command metadata, pilot documentation and saved evidence, and this Cairn change.
- **Testing**: profile positive/negative checks, exact source intake, trusted-boundary audit, both verifier invocations, promotion denial, and Cairn gates.
