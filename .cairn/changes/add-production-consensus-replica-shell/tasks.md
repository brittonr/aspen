## Phase 1: Configuration and shell

- [ ] [serial] Define the bounded canonical replica configuration and listener-key input, with exact group/membership/endpoint/durability/application/profile/bound admission and typed denials. r[molten.fabric_consistency.replica_shell.config]
- [ ] [serial] Implement fresh and recover lifecycle around the existing `ScopedLiveReplica`, admitted ports, readiness publication, bounded stop, and terminal receipt. r[molten.fabric_consistency.replica_shell.lifecycle]
- [ ] [serial] Reuse the existing ingress pump with explicit capacity, delivery, accept, and poll bounds; close the owned listener and fence further admission on stop. r[molten.fabric_consistency.replica_shell.ingress]
- [ ] [parallel] Bind the collaboration-scope application manifest/handler and deny unknown, test-only, or mismatched handlers before readiness. r[molten.fabric_consistency.replica_shell.application]

## Phase 2: Command and evidence

- [ ] [serial] Add `molten cluster replica` with explicit config/key paths, fresh/recover selection, status readback, and deterministic exit codes. r[molten.fabric_consistency.replica_shell.lifecycle] r[molten.fabric_consistency.replica_shell.evidence]
- [ ] [parallel] Emit canonical startup/terminal receipts and non-claims that cannot claim guest, KVM, machine-loss, deployment, or release authority. r[molten.fabric_consistency.replica_shell.evidence] r[molten.fabric_consistency.replica_shell.nonclaims]

## Phase 3: Validation and handoff

- [ ] [serial] Add negative admission tests for test refs, relative paths, duplicate/missing peers, ambient keys, wrong mode, unsupported handler, and unbounded limits. r[molten.fabric_consistency.replica_shell.validation]
- [ ] [serial] Add the native three-process production-command test covering election, commit, read, snapshot catch-up, restart, stale fencing, and clean teardown. r[molten.fabric_consistency.replica_shell.validation]
- [ ] [parallel] Document the exact ChaosControl guest handoff inputs and keep guest image, device, campaign, and KVM claims outside this change. r[molten.fabric_consistency.replica_shell.nonclaims]
- [ ] [serial] Run focused tests, Clippy, Octet, Cairn validation, and the proposal/design/tasks gates before sync or archive. r[molten.fabric_consistency.replica_shell.validation]
