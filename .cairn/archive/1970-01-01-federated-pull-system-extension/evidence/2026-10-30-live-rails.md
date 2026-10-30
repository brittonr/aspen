# Federated pull live rails — 2026-10-30

Executed in the integration worktree `cairn/finish-open-changes-20261029` (base
`d7e435e711f7f58167900ce7419121390b0842d3`, uncommitted integration) with the worktree's own
`target/debug/molten` build. These rails exercise no node startup source gate.

## DAG conformance input and its authority

`--dag-conformance-ref blake3:56234be682d161db5f7c35d66ac420a419562a56428efa3fc472794c0ebc4249`
identifies `../dag-sync-fabric-admission-binding/evidence/2026-10-30-dag-owner-conformance-run.txt`,
the executed transcript of the DAG owner suite (`cargo test -p molten --lib dag_sync::`, 10 passed).
The integration owner reviewed that transcript. **It is not independent DAG owner approval.** The
runs below therefore prove the federation mechanism under an explicitly experimental conformance
selection, not production admission or owner-approved conformance.

## Positive runs

| Command | Result |
|---|---|
| `molten federation live-loopback --run-root …/loopback --dag-conformance-ref $REF` | exit 0, `federation-live-proof-v1 "live-loopback"`, artifact `blake3:c3e5e221…faa7`, 10 checks |
| `molten federation distinct-process --run-root …/process --process-binary $MOLTEN --dag-conformance-ref $REF` | exit 0, `federation-distinct-process-proof-v1` over three distinct OS processes, same artifact `blake3:c3e5e221…faa7` |

Both proofs carry the non-claims `not production readiness`, `not permanent convergence` and
`not remote correctness`. The live receiver run also executes the installed simulation-world
campaign and compares its plan with the live session (`simulationworld::run` from
`liveproof/receiver/mod.rs`).

## Malicious-wire negatives

Each fault ran on a fresh owned root for both rails: `corrupt-signature`, `wrong-key`,
`wrong-domain`, `corrupt-content`, `unsolicited-push`. All ten runs exited 0 with the canonical
outcome `"no imported artifact after actual receiver boundary rejection"`; no receiver registry
artifact was materialized.

## Ignored live tests, run explicitly

`MOLTEN_DAG_CONFORMANCE_REF=$REF MOLTEN_FIXTURE_BINARY=$MOLTEN cargo test -p molten --lib mesh::liveproof::tests:: -- --ignored --test-threads=1`:

- `live_loopback_commits_real_receiver_registry_and_detached_evidence_denies_tampering` — ok
  (offline detached verification from a public-only root; tampered bundles and a wrong context deny)
- `actual_malicious_responses_cannot_materialize_receiver_registry_artifacts` — ok
- `distinct_os_processes_commit_signed_admitted_receiver_artifact` — ok
- `distinct_os_processes_reject_actual_malicious_signed_responses_before_registry_mutation` — ok
  (the child's `system-extension bound fabric-port routing failed` line is the expected denial)

`test result: ok. 4 passed; 0 failed`.

## Non-claims

No network beyond same-host loopback/process exchange, no production signer or DAG owner
approval, no permanent convergence, and no remote correctness is claimed.
