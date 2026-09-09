# Continuation review

## Goal and limits

Complete the remaining retry replay evidence without weakening the Octet or Nix gates.
Keep the current Molten worktree and preserve the primary checkout.
Completion requires executable replay controls, passing required gates, and normal lifecycle integration.
A test count or a changed expected source hash does not establish completion.

No subagent tool is available in this session. These local review passes are correlated.
The review permits two passes for each mechanism and three diagnostic commands for each build blocker before reassessment.
It does not permit policy bypass, manual lockfile edits, or publication with failed gates.

## Mechanisms

| Family | Mechanism | Evidence and next check | State |
|---|---|---|---|
| Retry replay | Recompute both canonical events from explicit planner inputs. Reject any difference from the recorded events. | Five new controls cover matching events, wrapped history, malformed sequences, independently changed events, and invalid inputs. The workspace suite passed. | Validated |
| Nix source binding | Check the declared Git revision and the generated unit2nix source identity. | Exact-revision prefetches and direct fetchgit builds established the source identities. The generator retains Git metadata but the builder removes it. Revision-bound native hash inputs correct the generated plans. | Validated for the recorded fetches |
| Octet diagnostic intake | Preserve source identity while admitting compiler diagnostic paths. | Both integration tests now include the same support source through its workspace-root path. All 73 integration tests passed. Octet emitted diagnostics for the included support source and completed as warning-only. | Intake repaired, strict collector blocked |

## Review details

The baseline fabric-time suite passed before the new replay boundary.
A permissive replay stub passed its positive control and failed all three initial negative controls.
The implemented boundary passed the full workspace suite across all targets and features.
One pre-existing test remained ignored.
A further negative case rejects each changed event independently, even with a valid canonical event identity.

The source-hash review used unit2nix revision `d4883180de0ce3033b7e4e2ab4216f33134863c5`.
Its `src/prefetch.rs` uses `--leave-dotGit`, while `lib/fetch-source.nix` omits `leaveDotGit`.
Fresh generation reproduced the incorrect hashes before the native hash inputs changed.
The review then used the generator's revision-qualified `crate-hashes.json` contract.
Both generated plans retain their dependency versions, URLs, and revisions.
The release plan retains its binary-only selection.
No lockfile changed.

Regeneration also exposed changed Git-metadata hashes for ChaosControl and Kamacite.
Exact-revision prefetches supplied their metadata-free identities instead of accepting the changing hashes.
The scope expanded only to those two additional changed identities.
No new dependency or local source override was introduced.

The complete Nix gate has a five-minute budget for this pass.
It passed the previous source-fetch failure positions and reached a contract-export drift failure.
That failure includes a Cairn policy export difference. The tracked policy was not weakened to pass the check.
A strict Octet pass has the same five-minute budget.
Neither budget permits a timeout to count as a passing gate.

## Terminal checks

- The final fabric-time suite passed all 23 tests (`replay-final.log.gz`).
- The final workspace Clippy pass used all targets, all features, and `-D warnings` (`clippy-final-continuation.log.gz`).
- The full workspace suite passed before the private module rename and test-support path repair (`continue-workspace.log.gz`).
- Both changed integration suites passed after the path repair: 61 CLI tests and 12 replication tests (`diagnostic-path-tests.log.gz`).
- Octet retained diagnostics from the included support source. Its non-strict run completed as warning-only, with 6,722 findings before the private module rename (`octet-continued.log.gz`).
- The strict workspace attempt failed in the Cargo worker at `package_id_spec.rs:244:40` and reported `Status: integration-failure` (`octet-strict-current.log.gz`). Its zero findings are not acceptance evidence.
- Nix progressed past the corrected source fetches. `molten-contract-export-drift-gate` failed because `cairn-policy/generated/cairn-policy.json` differs from the exported policy (`nix-continued.log.gz`).
- The broad Nix pass ended with an interruption. It is not a complete Nix result.

Pueue task history changed during this continuation. Durable command logs remain the evidence source.
Process inspection found no remaining task-owned strict Octet or bounded Nix command.
The existing policy fields remain intact. No check, policy, or lockfile was bypassed or weakened.

## Non-claims

The replay check covers the supplied retry inputs and recorded canonical events only.
It does not prove historical input authenticity, live timer execution, application retry safety, or global liveness.
The current delivery profile uses fixed delay. This correction does not prove a delivery-profile defect.
