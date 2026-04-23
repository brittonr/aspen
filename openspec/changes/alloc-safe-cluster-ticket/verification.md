# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

List the files changed for the change under review.
Each path must be repo-relative and currently appear in `git status`.

- Changed file: `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs`
- Changed file: `crates/aspen-ticket/tests/errors.rs`
- Changed file: `crates/aspen-ticket/tests/topic.rs`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/design.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/tasks.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/verification.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/alloc-safe-validation.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/broader-workspace-follow-up.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/i1-implementation.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/i2-implementation.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/i3-implementation.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/i4-implementation.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/i5-review-plumbing.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/final-validation.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/signed-validation.md`
- Changed file: `openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md`

## Task Coverage

Copy each checked task from `tasks.md` exactly and cite the evidence paths that justify it.
Every checked task must appear here.
For implementation-complete claims, prefer citing a saved diff artifact in addition to the source paths.

- [x] R1 Capture the current `aspen-ticket` dependency and consumer baseline under `openspec/changes/alloc-safe-cluster-ticket/evidence/baseline-validation.md`, including full-graph `cargo tree -p aspen-ticket -e normal`, full-graph `cargo tree -p aspen-ticket -e features`, full-graph `cargo tree -p aspen-ticket --no-default-features -e normal`, full-graph `cargo tree -p aspen-ticket --no-default-features -e features`, `cargo check -p aspen-ticket`, a deterministic direct-consumer discovery snapshot from the workspace dependency graph, and the baseline helper-usage classification search transcript recorded as a labeled baseline section in `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md` using the design's `rg -n 'SignedAspenClusterTicket|parse_ticket_to_addrs|with_bootstrap_addr|with_bootstrap\(|endpoint_addrs\(|endpoint_ids\(|AspenClusterTicket::deserialize|AspenClusterTicket::new|iroh::EndpointAddr|iroh::EndpointId|iroh_gossip::proto::TopicId|ClusterTopicId|try_into_iroh|to_topic_id|from_topic_id' ...` audit. [covers=architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/baseline-validation.md]
  - Evidence: `openspec/changes/alloc-safe-cluster-ticket/evidence/baseline-validation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`

- [x] I1 Refactor the unsigned `crates/aspen-ticket` core into an alloc-safe leaf surface: add `no_std` / `alloc` scaffolding, switch the unsigned payload to crate-local `ClusterTopicId` plus `aspen_cluster_types::NodeAddress`, and keep the `aspen-cluster-types` dependency edge alloc-safe with `default-features = false`. [covers=architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.bare-cluster-ticket-dependency-stays-alloc-safe,architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.topic-and-bootstrap-roundtrip-in-alloc-safe-form,architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/i1-implementation.md]
  - Evidence: `crates/aspen-ticket/Cargo.toml`, `crates/aspen-ticket/src/lib.rs`, `crates/aspen-ticket/src/v2.rs`, `openspec/changes/alloc-safe-cluster-ticket/evidence/i1-implementation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/alloc-safe-validation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`

- [x] I2 Add the unsigned boundary rules around that alloc-safe core: keep `ClusterTopicId` fixed-width and lossless at the optional iroh boundary, replace unsigned parse/validation `anyhow` results with crate-local errors, reject malformed topics through that error surface, reject legacy unsigned payloads through the checked-in fixture path `crates/aspen-ticket/tests/legacy.rs`, and surface the requirement that callers regenerate tickets with the current alloc-safe schema after legacy rejection. [covers=architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.alloc-safe-topic-identifier-stays-fixed-width-and-lossless-at-the-iroh-boundary,architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-cluster-ticket-errors,architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-unsigned-cluster-tickets-are-rejected-explicitly,architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.unsigned-wire-break-proof-is-reviewable] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/i2-implementation.md]
  - Evidence: `crates/aspen-ticket/src/parse.rs`, `crates/aspen-ticket/src/v2.rs`, `crates/aspen-ticket/tests/errors.rs`, `crates/aspen-ticket/tests/legacy.rs`, `crates/aspen-ticket/tests/topic.rs`, `openspec/changes/alloc-safe-cluster-ticket/evidence/i2-implementation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`

- [x] I3 Set the `crates/aspen-ticket` feature map to `default = []` with explicit `iroh`, `signed`, and `std` features, make `std` imply `signed`, move runtime-only unsigned conversions behind `iroh`, move signed explicit-time helpers behind `signed`, keep convenience wrappers behind `std`, keep signed-only distinct from `std`, and replace the signed encoder's empty-payload fallback with an `expect(...)`-style invariant break that carries contextual diagnostics from the encoder path. [covers=architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-ticket-support-requires-explicit-opt-in,architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-only-surface-stays-distinct-from-std-conveniences,ticket.encoding.signed-cluster-ticket-encoders-never-use-silent-default-fallbacks.signed-cluster-ticket-encoder-fails-loudly-on-impossible-serializer-bug,ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.invalid-signed-cluster-ticket-string-is-still-rejected] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/i3-implementation.md]
  - Evidence: `crates/aspen-ticket/Cargo.toml`, `crates/aspen-ticket/src/parse.rs`, `crates/aspen-ticket/src/signed.rs`, `crates/aspen-ticket/tests/signed.rs`, `crates/aspen-ticket/tests/std.rs`, `crates/aspen-ticket/tests/ui.rs`, `crates/aspen-ticket/tests/ui/iroh_helpers_require_feature.rs`, `crates/aspen-ticket/tests/ui/std_wrappers_require_feature.rs`, `openspec/changes/alloc-safe-cluster-ticket/evidence/i3-implementation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/signed-validation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`

- [x] I4 Update the root workspace dependency stanza plus every direct `aspen-ticket` consumer in the audit scope so feature opt-in is explicit where needed, bare/default consumers stay intentionally alloc-safe, each `iroh` consumer's shell-boundary conversion site is reviewable, and the plan is reopened before checking implementation tasks if the saved `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md` artifact shows `crates/aspen-rpc-handlers` or `crates/aspen-ci` actually use ticket helpers or discovers any new direct `aspen-ticket` consumer outside the current audit list. [covers=architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.iroh-conversion-happens-at-the-shell-boundary] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/i4-implementation.md]
  - Evidence: `Cargo.toml`, `crates/aspen-client/Cargo.toml`, `crates/aspen-cluster-handler/Cargo.toml`, `crates/aspen-cluster/Cargo.toml`, `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs`, `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs`, `openspec/changes/alloc-safe-cluster-ticket/evidence/i4-implementation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt`, `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`

- [x] I5 Add durable review plumbing for this seam: keep default-vs-`--no-default-features` equivalence reviewable, preserve the workspace-stanza proof with a negative guard against reintroducing `iroh`, `signed`, or `std`, preserve the direct-consumer discovery/classification audit, keep the concrete transitive re-export leak proofs `cargo tree -p aspen-fuse -e features -i aspen-ticket` and `cargo tree -p aspen-cli -e features -i aspen-ticket`, and keep `verification.md`, `implementation-diff.txt`, and `openspec-preflight.txt` synchronized with the evidence plan. [covers=architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent,architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable,ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.signed-cluster-ticket-encoding-proof-is-reviewable] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/i5-review-plumbing.md]
  - Evidence: `openspec/changes/alloc-safe-cluster-ticket/verification.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/i5-review-plumbing.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/default-vs-no-default-equivalence.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt`, `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`, `openspec/changes/alloc-safe-cluster-ticket/evidence/openspec-preflight.txt`, `openspec/changes/alloc-safe-cluster-ticket/evidence/final-validation.md`

- [x] V1 Save alloc-safe default-vs-`--no-default-features` proof under `openspec/changes/alloc-safe-cluster-ticket/evidence/alloc-safe-validation.md`, and save the dedicated comparison artifact at `openspec/changes/alloc-safe-cluster-ticket/evidence/default-vs-no-default-equivalence.md`, including full-graph `cargo tree -p aspen-ticket -e normal`, full-graph `cargo tree -p aspen-ticket -e features`, full-graph `cargo tree -p aspen-ticket --no-default-features -e normal`, full-graph `cargo tree -p aspen-ticket --no-default-features -e features`, `cargo check -p aspen-ticket`, `cargo check -p aspen-ticket --no-default-features`, `cargo check -p aspen-ticket --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --no-default-features --target wasm32-unknown-unknown`, the `aspen-cluster-types` alloc-safe dependency-edge proof, an explicit deterministic negative assertion that both alloc-safe surfaces exclude `iroh`, `iroh-gossip`, `rand`, and `anyhow`, and a deterministic proof that bare/default unsigned code cannot reach ambient-clock-only or other std-shell entry points without `std`. [covers=architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.bare-cluster-ticket-dependency-stays-alloc-safe,architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.default-and-explicit-alloc-safe-surfaces-remain-equivalent,architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.nodeaddress-dependency-edge-stays-alloc-safe] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/alloc-safe-validation.md]
  - Evidence: `openspec/changes/alloc-safe-cluster-ticket/evidence/alloc-safe-validation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/default-vs-no-default-equivalence.md`

- [x] V2 Save unsigned payload and wire-break proof under `openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md`, including current alloc-safe roundtrip success, generic malformed unsigned-input rejection through the crate-local error type, malformed-topic rejection, lossless alloc-safe-topic ↔ `iroh_gossip::proto::TopicId` conversion, legacy unsigned payload rejection, explicit regeneration guidance after legacy rejection, and the checked-in fixture path `crates/aspen-ticket/tests/legacy.rs`. [covers=architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.topic-and-bootstrap-roundtrip-in-alloc-safe-form,architecture.modularity.alloc-safe-cluster-tickets-default-to-transport-neutral-bootstrap-metadata.alloc-safe-topic-identifier-stays-fixed-width-and-lossless-at-the-iroh-boundary,architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.parse-and-validation-failures-use-cluster-ticket-errors,architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.legacy-unsigned-cluster-tickets-are-rejected-explicitly,architecture.modularity.cluster-ticket-parse-and-validation-errors-stay-alloc-safe-and-explicit.unsigned-wire-break-proof-is-reviewable] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md]
  - Evidence: `openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md`

- [x] V3 Save signed/signed-vs-std proof under `openspec/changes/alloc-safe-cluster-ticket/evidence/signed-validation.md`, including full-graph `cargo tree -p aspen-ticket --no-default-features --features signed -e normal`, full-graph `cargo tree -p aspen-ticket --features std -e normal`, `cargo check -p aspen-ticket --no-default-features --features signed`, `cargo check -p aspen-ticket --no-default-features --features signed --target wasm32-unknown-unknown`, `cargo check -p aspen-ticket --features std`, positive proof that explicit-time signed helpers work under the signed-only surface, explicit proof that the `--features std` surface also exposes the signed surface's explicit-time helpers/types, explicit negative assertions that the signed-only surface excludes `rand`, `iroh`, `iroh-gossip`, and `anyhow`, deterministic negative proof that `sign` / `sign_with_validity` / `verify` / `is_expired` stay behind `std`, explicit positive `std` proof for those convenience wrappers, explicit negative assertion that `--features std` alone does not enable `iroh` or `iroh-gossip`, malformed/corrupted signed-input rejection, and deterministic source proof that `SignedAspenClusterTicket::to_bytes()` uses an `expect(...)`-style invariant break with contextual diagnostics instead of an empty-payload fallback. [covers=architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-ticket-support-requires-explicit-opt-in,architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.signed-only-surface-stays-distinct-from-std-conveniences,ticket.encoding.signed-cluster-ticket-encoders-never-use-silent-default-fallbacks.signed-cluster-ticket-encoder-fails-loudly-on-impossible-serializer-bug,ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.invalid-signed-cluster-ticket-string-is-still-rejected] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/signed-validation.md]
  - Evidence: `openspec/changes/alloc-safe-cluster-ticket/evidence/signed-validation.md`

- [x] V4 Save consumer, workspace, and transitive leak proof as the final labeled section in `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`, and save the dedicated root-workspace artifact at `openspec/changes/alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt`, including full-graph `cargo tree -p aspen-ticket --features iroh -e normal`, `cargo check -p aspen-ticket --features iroh`, the root `Cargo.toml` `default-features = false` proof plus a deterministic negative assertion that fails if the workspace stanza reintroduces `iroh`, `signed`, or `std`, repo-wide direct-consumer discovery proof from the workspace graph, exact compile rails for every direct consumer named in the audit scope, shell-boundary conversion-site citations for every `iroh` consumer, the concrete transitive re-export leak proofs `cargo tree -p aspen-fuse -e features -i aspen-ticket` and `cargo tree -p aspen-cli -e features -i aspen-ticket`, deterministic negative proof that bare/default `aspen-ticket` cannot use iroh-only constructors/conversions without `iroh`, and an explicit reopen/escalation record if the saved audit for `crates/aspen-rpc-handlers` or `crates/aspen-ci` is non-empty or if repo-wide discovery finds any new direct `aspen-ticket` consumer outside the current audit list. [covers=architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.iroh-conversion-happens-at-the-shell-boundary,architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md]
  - Evidence: `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/workspace-dependency-proof.txt`

- [x] V5 Save final synchronized review packet under `openspec/changes/alloc-safe-cluster-ticket/evidence/final-validation.md`, keeping `verification.md`, `evidence/implementation-diff.txt`, and `evidence/openspec-preflight.txt` aligned with the evidence above so the full seam remains reviewable. [covers=architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable,ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.signed-cluster-ticket-encoding-proof-is-reviewable] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/final-validation.md]
  - Evidence: `openspec/changes/alloc-safe-cluster-ticket/evidence/final-validation.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`, `openspec/changes/alloc-safe-cluster-ticket/evidence/openspec-preflight.txt`

- [x] V6 Capture broader-workspace fallout follow-up under `openspec/changes/alloc-safe-cluster-ticket/evidence/broader-workspace-follow-up.md`, including `cargo check -p aspen-cli`, `cargo check -p aspen-fuse`, and `cargo check -p aspen --no-default-features --features node-runtime`, and record any seam-specific fix or explicit no-fallout result. [covers=architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable] [evidence=openspec/changes/alloc-safe-cluster-ticket/evidence/broader-workspace-follow-up.md]
  - Evidence: `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs`, `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs`, `openspec/changes/alloc-safe-cluster-ticket/evidence/broader-workspace-follow-up.md`, `openspec/changes/alloc-safe-cluster-ticket/evidence/final-validation.md`

## Review Scope Snapshot

If a reviewer needs the exact implementation delta, save a diff artifact that covers the
files you are claiming as complete.

### `git diff 83c6480f2 -- Cargo.lock Cargo.toml crates/aspen-client/Cargo.toml crates/aspen-cluster-handler/Cargo.toml crates/aspen-cluster/Cargo.toml crates/aspen-cluster/src/bootstrap/node/discovery_init.rs crates/aspen-cluster/src/bootstrap/node/sharding_init.rs crates/aspen-ticket/Cargo.toml crates/aspen-ticket/src/lib.rs crates/aspen-ticket/src/parse.rs crates/aspen-ticket/src/signed.rs crates/aspen-ticket/src/v2.rs crates/aspen-ticket/tests openspec/changes/alloc-safe-cluster-ticket`

- Status: captured
- Artifact: `openspec/changes/alloc-safe-cluster-ticket/evidence/implementation-diff.txt`

## Verification Commands

Record the commands actually run plus the durable artifacts that capture their output.
Keep artifacts inside the change directory, usually under `evidence/`.
If you claim preflight or syntax-check validation, save those transcripts too.

### `cargo test -p aspen-ticket --test errors`

- Status: pass
- Artifact: `openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md`

### `cargo test -p aspen-ticket --test topic`

- Status: pass
- Artifact: `openspec/changes/alloc-safe-cluster-ticket/evidence/unsigned-wire-break.md`

### `cargo test -p aspen-ticket --no-default-features --features signed --test signed`

- Status: pass
- Artifact: `openspec/changes/alloc-safe-cluster-ticket/evidence/signed-validation.md`

### `cargo test -p aspen-ticket --features std --test std`

- Status: pass
- Artifact: `openspec/changes/alloc-safe-cluster-ticket/evidence/signed-validation.md`

### `cargo check -p aspen-ticket --features iroh`

- Status: pass
- Artifact: `openspec/changes/alloc-safe-cluster-ticket/evidence/direct-consumer-audit.md`

### `cargo check -p aspen --no-default-features --features node-runtime`

- Status: pass
- Artifact: `openspec/changes/alloc-safe-cluster-ticket/evidence/broader-workspace-follow-up.md`

### `scripts/openspec-preflight.sh alloc-safe-cluster-ticket`

- Status: pass
- Artifact: `openspec/changes/alloc-safe-cluster-ticket/evidence/openspec-preflight.txt`

## Notes

- `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs` and `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs` needed `ticket.topic_id.to_topic_id()` after the alloc-safe topic seam; the broader `node-runtime` rail caught both call sites.
- Stage newly created source files before rerunning preflight so the review packet stays inside the flake source filter.
