Evidence-ID: alloc-safe-cluster-ticket.i3-implementation
Task-ID: I3
Artifact-Type: source-citation
Covers: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-ticket-support-requires-explicit-opt-in, architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-only-surface-stays-distinct-from-std-conveniences, ticket.encoding.signed-cluster-ticket-encoders-never-use-silent-default-fallbacks.signed-cluster-ticket-encoder-fails-loudly-on-impossible-serializer-bug, ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.invalid-signed-cluster-ticket-string-is-still-rejected

## Source citations

- `crates/aspen-ticket/Cargo.toml` defines the explicit `iroh`, `signed`, and `std` feature map with `default = []` and `std = ["signed", "dep:rand"]`.
- `crates/aspen-ticket/src/parse.rs` keeps runtime-only unsigned conversions behind `iroh`.
- `crates/aspen-ticket/src/signed.rs` keeps explicit-time signed helpers behind `signed`, wall-clock helpers behind `std`, and uses `expect(...)` instead of an empty-payload fallback in `SignedAspenClusterTicket::to_bytes()`.
- `crates/aspen-ticket/tests/signed.rs` proves signed-only explicit-time validation and malformed signed decode behavior.
- `crates/aspen-ticket/tests/std.rs` proves `std` wrappers still expose the signed surface's explicit-time helpers.
- `crates/aspen-ticket/tests/ui.rs`, `crates/aspen-ticket/tests/ui/iroh_helpers_require_feature.rs`, and `crates/aspen-ticket/tests/ui/std_wrappers_require_feature.rs` keep the negative feature-boundary proofs durable.

## Verification cross-check

- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/signed-validation.md`
- `openspec/changes/archive/2026-04-25-alloc-safe-cluster-ticket/evidence/implementation-diff.txt`
