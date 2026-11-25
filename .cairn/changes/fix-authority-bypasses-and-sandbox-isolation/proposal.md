## Why

Direct source audit across Aspen's authority, capability, execution, and identity adapters uncovered critical bypasses that violate Aspen's explicit-authority and bounded-effects invariants:

1. **Tautological native system-extension authority check (`src/system_extension/native_host/parts/executor/p001/body.rs:233-277`):** `Process::execution_request` clones `self.template.authority`, overwrites every single binding field from `request`, and passes `request.generation` as `active_generation` into `admit_request`, completely defeating request-vs-authority and stale-generation enforcement. Separately, `IntentRecordingEffectPort::route` (`p002/body.rs:143-165`) persists invalid effect outputs as `NativeOperationState::Terminal` before returning `materialized_admission?` and misclassifies post-effect journal errors as `storage` rather than `uncertain`.
2. **Unsandboxed Steel VM and fake resource metering (`src/harness/steel/executor.rs:128-183, 211-277`, `Cargo.toml:162`):** `run_vm` instantiates `Engine::new()` with full default `steel-core` stdlib (ambient filesystem/process/env access), computes fake fuel from `source.len() + input.len()` with zero runtime metering, relies on a 14-substring static blocklist trivially bypassed by anonymous `(lambda ...)` recursion or string doubling, enforces `STEEL_MAX_HOSTCALLS` only *after* execution terminates, and interpolates `callable` unescaped into S-expression source.
3. **Tautological `admit_authority` and empty-keys bypass (`src/authority/parts/mod/p001/body.rs:86-148, 189-228`):** `admit_authority` (called by `gatekeeper_resolve_live_ref`) passes `&context.subject_ref` as `requested_principal_ref` and `&context.key_refs` as `current_key_refs`; `authority_grant_currentness` skips key validation when `keys.is_empty()` and accepts contexts with stale keys via `.any()`; `cleanup_for_revocation` ignores `effective_at` and `target_kind`.
4. **Cryptographic identity adapter flaws (`src/fabric_crypto_identity/file/parts/adapter/p001..p002`, `tpm2.rs:88`):** `verify` silently overwrites a mismatched `signature.metadata.signer_public_ref` with `public_key_ref` right before `evaluate_verification`; `sign`, `sign_artifact_auth_statement`, and `load_transport_secret` suffer check-then-reload double-read TOCTOU; and `is_revoked()` checks only whether `{key_name}.revoked` is non-empty (ignoring handle/generation), causing file-key revocation after TPM2 rotation to permanently deny the new TPM2 key.
5. **Additional admission, replay, and filesystem authority gaps:**
   - `Envelope` wrappers (`ActorId`, `Capability`, `EvidenceRef` in `src/runtime/envelope/mod.rs:48-160`) derive transparent Serde `Deserialize`, bypassing `::parse`, and `validate_core` checks only list lengths.
   - `raft::membership::commit()` (`src/raft/membership.rs:35-79`) checks only `preflight.decision == "pass"` without binding `preflight` to the requested group, peer, role, or configuration.
   - Generic registered transport shell (`src/fabric_transport/shell.rs:55-94`) never marks routed requests consumed, permitting silent replay.
   - `context-profile` `merge_refs` (`src/operator/context/parts/profile/p000/body.rs:229-252`) omits conflict checks for `retention_refs` while applying override values with `decision = "pass"`.
   - `dag_sync` (`src/dag_sync/service.rs:60-94`) fetches authority/resources once before the transfer loop, missing mid-transfer revocation.
   - `spacewasm` (`src/spacewasm/admission.rs:159-237`) never binds `credential.policy_json`'s digest to `current.authority_ref` and permits `/` injection in `actor` URIs.
   - `fabric_execution::live::Adapter` (`src/fabric_execution/live.rs:58-86`, `mechanics.rs:7-41`) never hashes `stdin_bytes` against `stdin_ref`, never measures `executable_path` against `executable_identity_ref`, and blindly re-executes `Unknown`/`Terminal` operations on retry.
   - Filesystem capability races: `remove_stale_stages` (`crates/molten-node-host/src/node/state/publication.rs:73,133-150`) unlinks concurrent writers' active stage files; `DirectoryRoot::open_existing` (`authority.rs:131-137`), `protected()` (`reliability/authority/storage.rs:82-88`), and `CapabilityBundleSource::read_leaf` (`executable_extent/store.rs:23`) have symlink/mode TOCTOU windows.

## What Changes

- Remove field-copying from `request` into `authority` in `Process::execution_request` and resolve live current authority and active generation independently; fix `IntentRecordingEffectPort::route` to reject invalid outputs before committing `Terminal` and classify post-effect journal failures as `Uncertain`.
- Replace ambient `Engine::new()` in Steel execution with a strictly sandboxed engine, real runtime step/fuel and memory ceilings, synchronous pre-dispatch `STEEL_MAX_HOSTCALLS` enforcement, and safe invocation without raw string interpolation.
- Fix `admit_authority`, `authority_grant_currentness`, and `cleanup_for_revocation` to compare against independent expected principal/key sets, reject empty or partially stale key sets, and honor `effective_at` and `target_kind`.
- Fix `fabric_crypto_identity` to preserve observed `signer_public_ref` mismatches, read key material atomically from one verified handle/snapshot, and match revocation records by exact key handle/generation.
- Close all listed admission, replay, and capability-filesystem gaps across `Envelope`, `raft::membership`, `fabric_transport/shell.rs`, `context-profile`, `dag_sync`, `spacewasm`, `fabric_execution::live`, and `molten-node-host`.

## Impact

- **Files**: `src/system_extension/native_host/parts/{executor/p001,service/p002}/body.rs`, `src/harness/steel/executor.rs`, `Cargo.toml`, `src/authority/parts/mod/p001/body.rs`, `src/fabric_crypto_identity/file/parts/adapter/{p001,p002}/body.rs`, `src/fabric_crypto_identity/tpm2.rs`, `src/runtime/envelope/mod.rs`, `src/raft/membership.rs`, `src/fabric_transport/shell.rs`, `src/operator/context/parts/profile/p000/body.rs`, `src/dag_sync/service.rs`, `src/spacewasm/admission.rs`, `src/fabric_execution/{live.rs,mechanics.rs}`, `crates/molten-node-host/src/node/state/{publication.rs,authority.rs}`, `src/reliability/authority/storage.rs`, and `src/executable_extent/store.rs`.
- **Verification**: Dedicated negative regression tests for every bypass, forgery, TOCTOU, replay, and sandbox escape vector listed above.
- **Non-claims**: Closing these implementation bypasses aligns runtime shells with Aspen's authority model; it does not certify external OS kernel isolation or hardware attestation.

## Out of Scope

- World promotion / world head / world merge authority and transaction fixes (tracked in `harden-world-promotion-head-and-merge-semantics`).
- Live Raft peer transport authentication and reliability teardown replay (tracked in `repair-live-raft-liveness-safety-and-transport-auth`).
