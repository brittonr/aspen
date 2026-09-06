# Design

## Completion and authority

Completion requires actual pinned Octet output for the selected source, an independently selected operator cohort, exact runtime-binary binding, complete evidence verification, and normal startup before the two-VM proof. Counts, test receipts, command text, hashes, and process exit zero alone are insufficient.

The first implementation slice is verification-only. It must not construct a startup capability or remove either existing guard. Its report separates structural/identity verification from observed execution, operator approval, and startup authority.

## Approach review

1. Ambient workspace replay: rejected. It requires files absent from the VM and does not bind source bytes or the runtime binary.
2. Caller-provided gate receipt: rejected. A caller can manufacture clean counts and checks, as the old test helper did.
3. Exact approved cohort plus complete snapshot: selected for portable verification. The operator chooses expectations outside the evidence directory. The verifier hashes the descriptor before admitting member reads, enforces a closed member inventory, verifies each member once, and re-evaluates the strict gate with explicit Cargo/dylint metadata. Real execution and subsequent approval remain separate obligations.

Review uses one agent and correlated adversarial passes. Budget: one implementation slice, focused core/adapter tests, and read-only tool discovery on worker and desktop. No VM launch without an accepted real cohort.

## Ownership

`molten-core::node_startup` owns descriptor admission, exact cohort matching, finite member plans, source-inventory constraints, and measurement comparisons. It performs no I/O. The root adapter owns a no-follow capability directory, bounded reads, JSON decoding, current executable measurement, and report serialization. The Octet module owns strict receipt reconstruction and metadata formulas; the new snapshot path supplies bytes explicitly and does not change process cwd.

The descriptor contains roles, hashes, lengths, source revision, and separate build-compiler and Octet-tool identities. It contains no host paths or credentials. Member names are fixed by role, never supplied by untrusted metadata. Source inventory entries are relative names plus byte identities, not read capabilities. A digest preserves approved identity; it is not proof that Octet executed or that the binary came from the claimed source.

## Negative cases

Deny unsupported schemas, malformed identities, mismatched cohort/binary/source/tools, missing/duplicate/extra/reordered member roles, length/hash drift, unknown fields, traversal, symlinks, special files, oversized or changing files, missing context, stale metadata, warnings/errors, inconsistent summary/status, and synthetic toolchain markers. Require full source-inventory linkage, not only facade filenames or replay-command hints.

## Tool observations

The worker has Rust 1.97.1 for focused compilation and an installed Octet wrapper using nightly-2026-03-21. Read-only desktop discovery found a working nightly-2026-05-26 compiler. Both pinned Octet inputs (`fc38f593` and `cf04e894`) declare nightly-2026-03-21. These are different roles; neither observation establishes a complete passing gate for this source. No tool was installed, replaced, or rebuilt.
