# Verification: review-auth-ticket-public-api

## Implementation Evidence

- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/auth-ticket.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-portable-smoke/Cargo.toml`
- Changed file: `openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-portable-smoke/src/lib.rs`
- Changed file: `openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-runtime-negative/Cargo.toml`
- Changed file: `openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-runtime-negative/src/lib.rs`
- Changed file: `openspec/changes/review-auth-ticket-public-api/specs/auth-ticket-extraction/spec.md`
- Changed file: `openspec/changes/review-auth-ticket-public-api/proposal.md`
- Changed file: `openspec/changes/review-auth-ticket-public-api/design.md`
- Changed file: `openspec/changes/review-auth-ticket-public-api/tasks.md`
- Changed file: `openspec/changes/review-auth-ticket-public-api/verification.md`

- Public API ownership, canonical imports, compatibility re-export ownership, runtime-shell exclusions, and readiness state are recorded in `docs/crate-extraction/auth-ticket.md`.
- The extraction inventory and Nickel policy promote the auth/ticket family to `extraction-ready-in-workspace` while keeping publishable/repo-split states blocked on license/publication policy.
- `fixtures/auth-ticket-portable-smoke` is an independent downstream fixture that imports portable auth/ticket crates directly and patches `iroh-tickets` to the workspace-vendored graph.
- `fixtures/auth-ticket-runtime-negative` proves `aspen-auth` verifier/revocation APIs are unavailable when a portable consumer depends only on `aspen-auth-core`.
- Evidence files under `evidence/` cover compatibility tests, downstream metadata, dependency graph/forbidden runtime boundary, and readiness checker output.

## Task Coverage

- [x] Create proposal, design, delta spec, and task rail for `review-auth-ticket-public-api`.
  - Evidence: `openspec/changes/review-auth-ticket-public-api/proposal.md`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/design.md`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/specs/auth-ticket-extraction/spec.md`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/tasks.md`
- [x] Inventory portable auth/ticket public types and canonical imports.
  - Evidence: `docs/crate-extraction/auth-ticket.md`
  - Evidence: `docs/crate-extraction.md`
  - Evidence: `docs/crate-extraction/policy.ncl`
- [x] Add/update portable downstream fixture plus negative runtime-verifier boundary fixture.
  - Evidence: `openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-portable-smoke/Cargo.toml`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-portable-smoke/src/lib.rs`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-runtime-negative/Cargo.toml`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-runtime-negative/src/lib.rs`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-downstream-metadata.json`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-forbidden-boundary.txt`
- [x] Run serialization goldens, malformed rejection tests, fixture metadata, and representative consumers; update extraction manifests.
  - Evidence: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-compatibility.txt`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-portable-fixture.txt`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-readiness.md`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-readiness.json`
- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
  - Evidence: `openspec/changes/review-auth-ticket-public-api/verification.md`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-readiness.md`
- [x] Sync/archive only after every implementation/evidence task is complete.
  - Evidence: `openspec/changes/review-auth-ticket-public-api/tasks.md`
  - Evidence: `openspec/changes/review-auth-ticket-public-api/verification.md`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| fixture | `cargo test --manifest-path openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-portable-smoke/Cargo.toml` | pass | `evidence/auth-ticket-portable-fixture.txt` | Exercises direct downstream imports from portable auth/ticket crates without root Aspen or `aspen-auth`. | Add a published-crate fixture after license policy is decided. |
| negative | `cargo check --manifest-path openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-runtime-negative/Cargo.toml` | expected fail | `evidence/auth-ticket-compatibility.txt` | Proves portable defaults cannot import runtime `TokenVerifier`/revocation shell APIs. | Add compile-fail UI tests if the boundary becomes source-level API policy. |
| tests | focused `cargo test -p aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket` filters | pass | `evidence/auth-ticket-compatibility.txt` | Covers token/ticket goldens and malformed input rejection for the reviewed public surface. | Full workspace quick profile is broader than this API-review drain. |
| readiness | `scripts/check-crate-extraction-readiness.rs --candidate-family auth-ticket ...` | pass | `evidence/auth-ticket-readiness.md` | Covers policy/inventory/docs readiness contract for the virtual auth-ticket family. | Full publication package verification remains blocked on license policy. |
| format | `git diff --check` | pass | closeout transcript | Markdown/TOML/Rust fixture diffs are whitespace-clean. | Repo-wide lint is outside this docs/evidence slice. |

## Verification Commands

- Artifact: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-compatibility.txt`
- Artifact: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-portable-fixture.txt`
- Artifact: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-forbidden-boundary.txt`
- Artifact: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-downstream-metadata.json`
- Artifact: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-readiness.md`
- Artifact: `openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-readiness.json`

```bash
cargo test --manifest-path openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-portable-smoke/Cargo.toml
cargo metadata --manifest-path openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-portable-smoke/Cargo.toml --format-version 1 --no-deps > openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-downstream-metadata.json
cargo tree --manifest-path openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-portable-smoke/Cargo.toml -e normal
cargo check --manifest-path openspec/changes/review-auth-ticket-public-api/fixtures/auth-ticket-runtime-negative/Cargo.toml # expected failure: unresolved import aspen_auth
cargo test -p aspen-auth-core capability_token_golden_roundtrips_through_binary_and_base64
cargo test -p aspen-auth-core malformed_token_inputs_are_rejected
cargo test -p aspen-ticket cluster_ticket_golden_stays_stable
cargo test -p aspen-ticket malformed_unsigned_ticket_returns_deserialize_error
cargo test -p aspen-hooks-ticket hook_ticket_golden_stays_stable
cargo test -p aspen-hooks-ticket test_invalid_ticket_string
scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family auth-ticket --output-json openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-readiness.json --output-markdown openspec/changes/review-auth-ticket-public-api/evidence/auth-ticket-readiness.md
openspec validate review-auth-ticket-public-api --strict
scripts/openspec-preflight.sh review-auth-ticket-public-api
git diff --check
```
