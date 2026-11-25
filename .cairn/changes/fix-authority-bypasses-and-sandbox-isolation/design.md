# Design

## Native Extension Execution & Effect Router

- In `src/system_extension/native_host/parts/executor/p001/body.rs`: pass the actual independent current `authority` and live `active_generation` (from the supervisor/current-authority port, never overwritten from `request` fields) into `fabric_execution::admit_request`.
- In `src/system_extension/native_host/parts/service/p002/body.rs`: evaluate `admit_materialized_effect_output` *before* writing `NativeOperationState::Terminal` to journal or memory (recording invalid output as a typed rejection/uncertain failure rather than `Terminal`), and map any journal persistence failure after `self.delegate.route` to `FabricPortError::uncertain`.

## Sandboxed Steel Executor

- Restrict `steel-core` features in `Cargo.toml` to disable ambient filesystem/process/network/env modules where possible and construct the VM via `Engine::new_sandboxed()` in `src/harness/steel/executor.rs`.
- Enforce bounded execution via actual VM step/fuel limits (or bounded execution watchdog + memory ceiling) rather than static token counts and 14-item substring checks.
- Check `hostcall_counter` *before* permitting a hostcall inside the registered `molten-hostcall` closure so excess calls deny immediately rather than post-hoc.
- Validate `callable` strictly as a closed identifier (rejecting whitespace, parentheses, quotes, comments) or invoke the compiled Steel function handle directly instead of interpolating into source text.

## Authority Currentness & Cryptographic Identity Adapter

- In `src/authority/parts/mod/p001/body.rs`: accept explicit `expected_principal_ref`, `minimum_epoch`, and `current_key_refs` inputs in `admit_authority` / `gatekeeper_resolve_live_ref` instead of echoing `context`'s own fields; reject empty `context_refs.keys` and require every declared signing key to be current; check `revocation.effective_at <= logical_time` and `revocation.target_kind` in `cleanup_for_revocation`.
- In `src/fabric_crypto_identity/file/parts/adapter/`:
  - Remove the silent `observed.signer_public_ref = public_key_ref` overwrite (`p001/body.rs:228-231`) so `evaluate_verification` observes and denies metadata mismatches.
  - Perform revocation check, metadata resolution, and secret material loading from a single consistent opened file/snapshot (`p001`/`p002`) so concurrent rotation/revocation cannot mix old metadata with new key bytes.
  - Parse `{key_name}.revoked` and check whether the *specific* active key handle/generation is revoked rather than blocking all keys for that purpose (unblocking file-to-TPM2 rotation in `tpm2.rs`).

## Closing Remaining Admission, Replay, and Capability Filesystem Gaps

- `src/runtime/envelope/mod.rs`: implement checked custom `Deserialize` delegating to `::parse` for `ActorId`, `Capability`, and `EvidenceRef`, and make `validate_core` revalidate envelope version and all bounded identifiers.
- `src/raft/membership.rs`: bind `preflight` receipt fields (group, target peer, requested role, configuration ref, valid BLAKE3 syntax, and receipt digest) to `input` inside `commit()`.
- `src/fabric_transport/shell.rs`: track consumed request refs in `routed_requests` (or remove from `self.requests`) inside generic `execute_effect`, closing silent replay.
- `src/operator/context/parts/profile/p000/body.rs`: add conflict diagnostics for `retention_refs` overrides matching `policy_refs`/`authority_refs`/`resource_refs`.
- `src/dag_sync/service.rs`: refresh authority and resource admission per object fetch inside the transfer loop (matching `docs/dag-sync.md`).
- `src/spacewasm/admission.rs`: verify `blake3(credential.policy_json)` against `current.authority_ref` and restrict `actor` to nominal segments without `/` or path separators.
- `src/fabric_execution/live.rs` & `mechanics.rs`: hash `stdin_bytes` against `stdin_ref`, measure `executable_path` (via capability/opened FD) against `executable_identity_ref`, and check `self.operations` to refuse re-executing `Unknown`/`Terminal` `(operation_ref, generation)` pairs.
- `crates/molten-node-host/src/node/state/{publication.rs,authority.rs}`, `src/reliability/authority/storage.rs`, `src/executable_extent/store.rs`: scope stage cleanup to owned/dead-PID stages, open directories/files with `O_NOFOLLOW`/single-handle `observe_file` metadata checks, and forbid symlink following on `read_leaf`.
