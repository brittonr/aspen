## Validation Evidence

- Focused pure-core, live Iroh shell, registered effect-port, and three distinct-process CLI harness tests passed, including positive frame exchange and negative tamper, swapped-parent, spawn-failure, malformed artifact, symlink, oversized artifact, stale binding, queue-bound, replay, timeout, disconnect, and cleanup paths.
- `cargo fmt --check` and `cargo clippy -p molten-core -p molten --all-targets -- -D warnings` passed in the Nix development environment.
- `cargo octet check` and the repository pre-commit Octet gate completed with zero errors.
- `cairn validate --root . --strict` and the proposal, design, and tasks gates passed.
- `nix build .#checks.x86_64-linux.molten --no-link -L` passed with the staged source, exercising the hermetic nextest derivation.
- The pre-sync `fabric-transport` traceability run referenced all 9 previously accepted requirements and failed exactly on the 5 active cross-process references, proving that the scoped rail did not silently accept dangling implementation markers.
- Sync receipt `06a068743e600f0f4ca2decfb83f1bc9652c1acbc74204b723cb827fc0b9680d` merged all 5 requirement blocks into `cairn/specs/fabric-transport/spec.md`.
- The post-sync `fabric-transport` traceability receipt `ea4c983509125c89ec6210040ec17a4e3073dca39ea0df81c01578aa8ce89d20` passed with 14 of 14 requirements referenced, no missing requirements, and no dangling references.

## Bounded Claims

The scoped traceability profile proves only that every accepted fabric-transport requirement has at least one source-controlled implementation or verification reference in the transport trees. It does not erase repository-wide traceability debt, prove behavioral correctness by marker presence, establish WAN qualification, or promote connectivity evidence into delivery, membership, authority, consistency, or release claims.
