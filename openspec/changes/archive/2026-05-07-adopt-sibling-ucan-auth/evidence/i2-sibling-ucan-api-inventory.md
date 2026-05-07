# I2 sibling `../ucan` public API inventory

- Change: `adopt-sibling-ucan-auth`
- Task: Inventory sibling `../ucan` public APIs and classify portable/runtime Aspen ownership.
- Started: 2026-05-06T23:24:19Z
- Completed: 2026-05-06T23:27:31Z
- Status: captured
- Sources inspected:
  - `../ucan/AGENTS.md` repository boundary notes from project context
  - `../ucan/crates/ucan-core/src/lib.rs`
  - `../ucan/crates/ucan-core/src/token.rs`
  - `../ucan/src/lib.rs`
  - `../ucan/src/token.rs`
  - `../ucan/src/shell.rs`
  - `../ucan/src/error.rs`
  - `../ucan/src/readiness.rs`
  - `../ucan/docs/public-api.md`
  - `../ucan/examples/issue_and_verify.rs`
  - `../ucan/examples/invocation.rs`

## Source boundary summary

Sibling `../ucan` already has the same split Aspen needs:

- `ucan-core` is `#![no_std]` plus `alloc`. It owns deterministic UCAN capability/claim shape validation with no filesystem, network, ambient clock, signer, resolver, revocation, replay, or serde/base64 shell behavior.
- root `ucan` is the `std` shell. It owns compact token parsing/encoding, signer-bound issuance, Ed25519 DID/signature helpers via vendored `verified-logic`, proof-chain traversal, revocation/replay/caveat hooks, examples, file/reader/writer helpers, and production-readiness reporting.
- `verified` exports are advanced/finite-predicate support APIs surfaced by the runtime shell. They are not Aspen-facing API commitments unless the adapter needs them internally for evidence or guardrails.

## APIs appropriate for `aspen-auth-core`

These are portable/no-std candidates because they come from `../ucan/crates/ucan-core` and only validate/hold deterministic borrowed/owned data:

| Sibling API | Source anchor | Aspen ownership decision |
| --- | --- | --- |
| `CapabilityView<'a>`, `CapabilityView::new`, `.resource()`, `.ability()` | `ucan-core/src/lib.rs:17` | Good fit for `aspen-auth-core` validation/adaptation of Aspen `Capability`/`Operation` into UCAN resource/ability strings. |
| `parse_capability_document`, `validate_resource_value`, `validate_ability_value` | `ucan-core/src/lib.rs:119`, `:180`, `:191` | Good fit for portable validation helpers if Aspen core needs string-level UCAN admission without pulling runtime token machinery. |
| `CapabilityField`, `CapabilityError` | `ucan-core/src/lib.rs:47`, `:71` | Good fit as lower-level error/source types; Aspen-facing errors should still wrap/redact into existing `AuthError` shapes. |
| `token::{DidView, DidField, TimeField, TimeBounds}` | `ucan-core/src/token.rs:18`, `:34`, `:206`, `:507` | Good fit for portable claim validation if `aspen-auth-core` needs structural token-claim adapters. Do not make runtime clock decisions here. |
| `token::{Caveat, CaveatInput, CaveatView, CaveatError}` | `ucan-core/src/token.rs:228`, `:270`, `:290`, `:170` | Good fit for portable caveat shape admission only. Aspen-specific caveat policy/evaluation belongs in runtime adapter or later mapping work. |
| `token::{Capability, CapabilityInput}` | `ucan-core/src/token.rs:337`, `:399` | Good fit for owned portable capability claims. Use to preserve no-std capability claim construction. |
| `token::{ProofReference, ProofReferenceView, ProofReferenceInput, ProofReferenceError}` | `ucan-core/src/token.rs:436`, `:458`, `:489`, `:191` | Good fit for portable proof-reference byte admission. Proof fetching/storage belongs outside core. |
| `token::{ClaimsInput, ClaimsView, Claims}` and `Claims::from_input` | `ucan-core/src/token.rs:544`, `:554`, `:614`, `:629` | Good fit for deterministic UCAN claim-shape validation if Aspen core needs it; full compact parsing/signature verification remains runtime. |
| Bound constants `MAX_CAPABILITY_COUNT`, `MAX_PROOF_REFERENCE_COUNT`, `MAX_CAVEAT_COUNT` | `ucan-core/src/token.rs:9` | Good fit for preserving Tiger-style boundedness at the portable boundary. |

`aspen-auth-core` should not depend on root `ucan` unless that crate offers a no-std feature in the future. Keep root `ucan` out of protected `aspen-core --no-default-features` paths.

## APIs appropriate for runtime `aspen-auth`

These require `std`, serde/base64, Ed25519/signing, resolver/proof/revocation/replay hooks, file IO, examples, or production readiness reporting. They belong in `aspen-auth` or a runtime adapter that `aspen-auth` owns:

| Sibling API | Source anchor | Aspen ownership decision |
| --- | --- | --- |
| `CompactToken`, `IssuerDid`, `AudienceDid`, `CapabilitySet`, `AuthorizationRequest`, `AuthorizationDecision` | `ucan/src/token.rs:489`, `:560`, `:626`, `:698`, `:746`, `:1258` | Runtime adapter should translate Aspen token strings and capability checks through these while preserving Aspen-facing types and error redaction. |
| `TokenSigner`, `issue_token_with_signer`, `IssueRequest`, `IssueRequestBuilder`, `issue_token` | `ucan/src/token.rs:1689`, `:2445`, `:1893`, `:1925`, `:2433` | Runtime issuance surface. Prefer `TokenSigner`/`issue_token_with_signer` for key-custody boundary; keep `IssueRequest` compatibility only behind Aspen adapter if needed. |
| `Ed25519InMemorySigner`, `Ed25519SigningKey`, `Ed25519PublicKey`, `Ed25519VerificationKey`, constants | `ucan/src/token.rs:1716`, `:1785`, plus `ED25519_*` constants at `:114` | Runtime key/signature material. Do not expose secret-bearing values through core or logs; sibling Debug redacts secret material. |
| `KeyResolver`, `KeyResolutionContext`, `ProofStore`, `ProofCollection`, `RevocationChecker`, `NoRevocations`, `RevocationList`, `ReplayAdmission`, `AllowAllReplayAdmission` | `ucan/src/token.rs:1313`, `:1862`, `:1324`, `:1336`, `:1373`, `:1382`, `:1421`, `:1392`, `:1407` | Runtime admission hooks. Aspen should implement these over existing RPC/federation/revocation/replay stores and fail closed on backend errors. |
| `VerificationTime`, `VerificationLimits`, `VerificationContext`, `VerificationContextBuilder` | `ucan/src/token.rs:1457`, `:1992`, `:2047`; public API doc lines 12,17 | Runtime verification context. Aspen must supply explicit time through its time boundary; no ambient clock reads in adapter core logic. |
| `parse_compact_token`, `validate_decoded_token`, `verify_token_signature`, `verify_compact_token`, `verify_compact_token_str`, `verify_compact_token_with_resolvers`, `verify_compact_token_with_resolvers_and_revocations` | `ucan/src/token.rs:2472`, `:2513`, `:2544`, `:2616`, `:2626`, `:2639` | Runtime verification pipeline. Aspen adapter can use staged parse/validate for CLI inspection and full verify for RPC admission. |
| `VerifiedToken`, `EffectiveDelegation`, `VerifiedToken::authorize*` | `ucan/src/token.rs:2185`, `:2153`, `:2221` | Runtime authorization. Aspen should wrap these behind existing `TokenVerifier::authorize`/RPC admission semantics. |
| `CaveatPolicy`, `CaveatPolicySet`, `DenyUnknownCaveats`, `VerifiedTypedCaveatPolicySet`, `TypedCaveatContext` | `ucan/src/token.rs:866`, `:870`, `:875`, `:910` | Runtime caveat policy. Aspen must explicitly decide supported caveats; missing policy must deny. |
| `InvocationRequest`, `InvocationResult`, `verify_invocation`, `verify_invocation_with_replay` | `ucan/src/token.rs:1024`, `:1104`, `:1124`, `:1190`; `examples/invocation.rs` | Runtime invocation/admission surface. Useful for RPC handler authorization once Aspen maps request resource/ability pairs. |
| `CapabilityDocument`, `CaveatDocument`, `load_capability_from_path`, `load_capability_from_reader`, `write_capability_to_writer` | `ucan/src/shell.rs:18`, `:61`, `:122`, `:133`, `:145` | Runtime shell/document IO only. Do not place file/path/reader/writer helpers in `aspen-auth-core`. |
| `production_readiness_report`, `ProductionReadinessReport`, `ReadinessSurface`, `IntegrationProfile` | `ucan/src/readiness.rs:31`, `:44`, `:55`, `:212` | Runtime/operator guardrail. Useful for docs/evidence and maybe deploy checks, not portable auth-core API. |

## APIs that should remain adaptation-layer/internal, not direct Aspen-facing commitments

- `ucan::verified::*` low-level finite predicates and transcript structs are support APIs. Aspen may rely on the sibling runtime's live verification path, but should not re-export broad verified internals from `aspen-auth-core` or `aspen-auth` unless a specific OpenSpec task needs them.
- Remote DID resolution, proof transport, registry-backed revocation, durable replay persistence, non-Ed25519 algorithms, and arbitrary caveat vocabularies are explicitly unsupported by sibling turnkey code. Aspen must supply these backends/policies or document them unsupported.
- `AllowAllReplayAdmission` and `NoRevocations` are compatibility/local defaults. Production Aspen admission should wire caller-owned revocation/replay backends where required by the capability mapping and RPC/federation paths.
- `Ed25519InMemorySigner` and `IssueRequest` own secret material and are safe only because Debug is redacted; Aspen-facing CLI/docs should still avoid printing compact tokens or key material.

## Mapping guidance for next task

Next boundary work should define an Aspen `Capability`/`Operation` to UCAN `resource`/`ability` table. This inventory implies:

1. `aspen-auth-core` can own only string/claim shape conversion using `ucan-core` types.
2. `aspen-auth` owns issuance, compact token strings, signature verification, proof/revocation/replay/caveat hooks, and RPC/federation admission adapters.
3. Aspen-specific operations, node/resource identity, bearer-token parsing, federation credentials, and CLI UX remain Aspen adapter behavior even if the adapter delegates UCAN token mechanics to sibling APIs.

## Verification IDs touched

- `ucan-auth-integration.sibling-source-of-truth`
- `ucan-auth-integration.adapter-preserves-aspen-boundary`
- `ucan-auth-integration.no-std-boundary`
- `ucan-auth-integration.runtime-shell-boundary`
- `ucan-auth-integration.explicit-policy-backends`
