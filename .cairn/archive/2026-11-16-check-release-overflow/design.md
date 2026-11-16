## Context

Rust release builds disable overflow checks unless a profile enables them, and nixpkgs `buildRustCrate`, which the unit2nix package build uses, ignores Cargo profiles and passes no overflow option. Pure cores already use checked arithmetic with typed outcomes for bounds and time, but nothing caught a missed site at runtime.

## Decisions

### Decision: Trap overflow in Molten-owned crates in both release builds

**Choice:** set `overflow-checks = true` in `[profile.release.package.<member>]` for the four workspace members, and add an `extraRustcOpts = [ "-C" "overflow-checks=on" ]` crate override for the same crates in `flake.nix`. A flake check evaluates the workspace crate derivations and fails if any member lacks the option.

**Rationale:** a wrapped index, counter or budget can turn into a wrong admission or an unbounded loop; a trap fails closed. Limiting the setting to Molten-owned crates keeps the Cargo and Nix builds identical without per-crate overrides for several hundred dependencies, and leaves upstream crates' performance assumptions alone. No paired measurement exists; the cost is accepted as unmeasured.

### Decision: Fix the known sites instead of relying on the trap

**Choice:** make the two named sites total: the quorum-index search starts at `commit_index.checked_add(1)` and returns the commit index when none exists, and the allocator's frees use `checked_sub` with a sticky accounting fault that denies further allocations.

**Rationale:** a trap in a pure transition would turn a boundary case into a crash, and a panic in an `extern "C"` function aborts the process. Explicit outcomes keep both paths deterministic.

## Risks / Trade-offs

- Release builds are somewhat slower and larger where arithmetic is hot; this was not measured. Reopen with paired representative measurements if a profile shows a regression.
- Overflow checks do not police lossy `as` conversions; those remain a review concern.
