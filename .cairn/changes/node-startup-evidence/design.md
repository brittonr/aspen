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

`molten-core::node_startup` owns descriptor admission, exact cohort matching, finite member plans, source-inventory constraints, and measurement comparisons. It performs no I/O. The `molten-node-runtime` adapter owns a no-follow capability directory, bounded reads, JSON decoding, current executable measurement, and report serialization. The Octet module owns strict receipt reconstruction and metadata formulas; the snapshot path supplies bytes explicitly and does not change process cwd.

A digest preserves approved identity; it is not proof that Octet executed or that the binary came from the claimed source.

The unchanged root `molten` executable compiled a broad unrelated closure; the pinned strict Octet gate reported 14,529 errors on that boundary. The operator selected a separate complete `molten-node` executable with no root-crate dependency instead of excluding root source from the unchanged executable's evidence. The standalone executable still needs a real compiler source-unit inventory, including local dependency, test, build-script, and generated inputs relevant to the checked target, and a strict Octet check of every selected first-party package. `build-inputs.json` is an exact, closed claim whose union must match the bound `.rs` inventory; independent review must compare it with actual compiler inputs, link source bytes to the build, and bind the resulting executable before approval. No snapshot JSON can self-attest those facts.

## Negative cases

Deny unsupported schemas, malformed identities, mismatched cohort/binary/source/tools, missing/duplicate/extra/reordered member roles, length/hash drift, unknown fields, traversal, symlinks, special files, oversized or changing files, missing context, stale metadata, warnings/errors, inconsistent summary/status, and synthetic toolchain markers. Require full source-inventory linkage, not only facade filenames or replay-command hints.

## Tool observations

The worker has Rust 1.97.1 for focused compilation and an installed Octet wrapper using nightly-2026-03-21. Read-only desktop discovery found a working nightly-2026-05-26 compiler. Both pinned Octet inputs (`fc38f593` and `cf04e894`) declare nightly-2026-03-21. These are different roles; neither observation establishes a complete passing gate for this source. No tool was installed, replaced, or rebuilt.
