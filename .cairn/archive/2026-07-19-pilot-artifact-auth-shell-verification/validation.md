# Validation evidence

## Baseline and completion contract

Before core changes, `nix develop -c cargo test -p molten-core fabric_crypto_identity` and `nix develop -c cargo test -p molten fabric_crypto_identity` pass. The core baseline has six focused identity/adoption tests; the product-shell baseline has six production key, exact legacy-domain signing, tamper, rotation, readback, and integration tests. Native Cairn validation and proposal/design/tasks gates pass.

The exact goal is a production-managed Molten Ed25519 key signing `artifact_auth_core::canonical_statement_bytes` for the pure mapped signer statement, followed by verification of an independently reconstructed statement through `artifact-auth-ed25519`. Completion evidence requires real positive verification, tamper/wrong-preimage/wrong-key/malformed/currentness/carrier negatives, public bounded evidence, strict validation, and retained legacy authority. Reusing a legacy signature or boolean, exposing unrestricted/private-key signing, fixture-only verification, task checkboxes, or setting standalone authority true are false completion.

The source remains `ssh://git@github.com/OnixResearch/artifact-auth.git` revision `799459346d5416fbd7b9f55840a7371441b55afa`. SSH credentials authorize retrieval only. The shell-verifier package must resolve from that same source and revision.

## Portfolio-search registry

| Family | Mechanism | Evidence/state |
| --- | --- | --- |
| Legacy signature reuse | Existing canonical-domain signature or supplied verification boolean | Falsified: distinct preimage; cannot establish exact standalone verification. |
| Generic signer export | Expose private material or unrestricted signing | Rejected: widens key and signing authority. |
| Fixture-only signing | Deterministic standalone test key | Blocked: no product-shell evidence. |
| Purpose-bounded exact-statement shell | Pure statement map, capability-file sign, pinned standalone verification | Selected for implementation and adversarial audit. |

The bounded search budget is four mechanism families, targeted review of the core mapper, standalone verifier, capability-file adapter, integration shell, and focused tests, followed by two deterministic validation rounds. Audit risks are reconstruction drift, purpose/profile mismatch, full-key encoding, carrier substitution, malformed signatures, revoked/unknown currentness, unrelated-failure parity, secret leakage, and authority promotion.

## Implemented shell boundary

`molten-core::map_artifact_auth_statement` now supplies the one pure signer-specific statement mapping used by both verification preparation and dual-run evaluation. It accepts no legacy cryptographic result. The product shell pins `artifact-auth-core` and `artifact-auth-ed25519` at the reviewed revision, asks the capability-file adapter to sign only standalone canonical bytes with a current purpose/profile/full-key-matched handle, independently reconstructs the statement, recomputes statement/public-key/signature carrier refs and lowercase signature hex, and verifies with the standalone Ed25519 implementation.

The public shell report records bounded refs, signature hex, stable standalone failure code, and the existing dual-run report. It carries no private key, secret record, backend locator, credential, capability, membership, deployment, lifecycle, or release decision. The comparator now records both legacy and standalone issue families and blocks disjoint dual rejection as `unrelated-rejection-causes`.

## Focused evidence

The real positive shell test generates a capability-file evidence-signing key, signs the exact standalone canonical statement, verifies it through `artifact-auth-ed25519`, observes accepted legacy and standalone decisions, checks full-key/ref/hex evidence, confirms no secret-record bytes appear in public debug evidence, and retains legacy authority with rollback while standalone authority remains false.

The adversarial shell test passes for signature tamper, changed standalone preimage, wrong full key, malformed signature length, signature-carrier identity substitution, revoked currentness, unknown currentness reference, legacy-boolean non-reuse, and disjoint legacy-payload/standalone-signature false parity. The audit also found that exact signing must not accept an arbitrary mapped request merely because its profile/purpose/key identity matched. The shell now requires an accepted legacy observation plus exact current-handle generation, signing currentness, and currentness-evidence identity before it asks the adapter to sign; a dedicated negative assertion covers that denial.

Focused core/shell tests, rustfmt, and strict all-target Clippy pass. A secondary VibeThinker audit was attempted but the local endpoint returned `fetch failed`; deterministic source review and repository checks remain authoritative.

## Full validation

Full workspace tests, strict workspace all-target Clippy, the repository Octet check, native Cairn validation/gates, and `nix flake check -L` pass on `x86_64-linux`. The pinned unit2nix tool regenerated the default include-dev plan and package-scoped release-policy plan; both bind Cargo lock SHA-256 `e4a442cb06c9f31737e782b72448d79ef8fbb3fcf5a8e590688253b802db9bf2` and the exact standalone source revision.

The release-dependency profile now declares both standalone packages against the one reviewed Nix input. Its Nickel contract permits a shared Nix input only when every package row has the same source coordinate and immutable revision. The current profile is the positive fixture; a conflicting-source/revision shared-input fixture is rejected. The focused release-dependency Nix check and complete flake check both pass.

The Nix dogfood rail completed with release-bundle verification receipt `blake3:97f0ca313e2499cc1d2c3d9c256cd3f4ffc761ff7ce5734aecd51a8de62edf10`, promotion receipt `blake3:88719296f402e84dcf8373ae0f03c8605b6baa0e8fa1516e2acdd814f3ae1eec`, and export-verification receipt `blake3:c27d67befaabaf825724704d3d29d58aa41a629c8e77d351169a2a72b1aa5601`. Those receipts remain evidence-only and do not admit standalone authority.
