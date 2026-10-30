# Peer-query Raft recovery and voting admission: verification

## Outcome and ownership

The deferred voting-readiness wiring in the 2026-10-22 verification is complete. Existing live replica services now execute bounded canonical peer queries, durable repairs, and a negotiated term barrier before a suspect replica can rejoin. Local storage reconciliation alone does not grant voting authority.

The durable capability is restart-safe recovery admission, consumed directly by Molten's live Raft shell and `RedbReplicaPort`. Molten's fabric-consistency maintainers own the implementation, regression corpus, and architecture documentation. No new external dependency or adoption shim was introduced.

## Final ordered checks

Executed from the dedicated recovery worktree inside its Nix development shell:

1. Configured workspace `cargo octet check`, then `cargo octet check -p molten -- --lib`: both **clean**, zero findings, warnings, or errors, using the pinned `nightly-2026-03-21` producer.
2. Complete root-source object-corpus receipt: **15,302 objects**, supplied with the enumerated 2,061 `src/**/*.rs` inputs.
3. Artifact import: **pass**, six imported artifacts; strict-CI source gate: **pass**; remediation plan emitted.
4. `cargo test --locked -p molten-core --lib -- consensus_recovery`: **56 passed**.
5. `cargo test --locked -p molten --lib -- fabric_consistency`: **passed**.
6. `cargo test --locked --workspace`: **passed**, including integration and documentation tests.
7. `cargo clippy --locked --workspace --all-targets -- -D warnings`: **passed**.
8. `cargo fmt --all --check`: **passed**.

The final ordered shell command exited zero after all eight stages. The final producer artifacts, complete corpus receipt, and canonical import/gate/remediation receipts are retained under [2026-10-29-peer-recovery](2026-10-29-peer-recovery/).

Canonical receipt references:

- Artifact import: `blake3:bfe24b42fa4f5c7987d0d2897de22d12626d2dd23c72324404f503987f93221d`.
- Strict-CI gate: `blake3:d467bff8a0f25b1800d4e9d2455524d410a94ead624de5d995a81ba80ff1ea28`.
- Remediation plan: `blake3:f9825eb5ce549ae54d3d84ac2f190c5642d6a3e4ded9612b082eaa62177ca8d3`.

## Exercised behavior

The existing `three_elects_commits_reads_and_catches_up` harness ran three actual worker processes over the live transport and durable ports. It exercises election, commitment, reads, lagging-replica catch-up, and all-participant crash/restart. Recover-mode startup asserts `Recovering`; workers complete peer-query reconciliation before the designated candidate campaigns, and final shutdown asserts `Voter`. This is process-level execution, not a new VM or deployed-cluster campaign.

Core and shell regressions cover:

- missing, faulty, conflicting, duplicate, gapped, oversized, and substituted peer observations;
- committed-history protection, unknown commitment, peer-held uncommitted evidence, and peer-only quorum intersection;
- complete-corpus pagination, changed-corpus restart, stale-term invalidation, and the higher-term barrier;
- positive repair from one available committed source without voting or truncation admission, including continued cached multi-batch repair on timers;
- lost hard state, complete peer-term fencing, and refusal of higher-term votes while recovering;
- real Redb/FaultDisk synchronization failure after acknowledged durable intent, followed by storage reconciliation without Raft startup effects and two genuinely clean opens that must still recover;
- durable intent visible from power-loss-image records before data/application effects, and exact retained term/vote after successful completion;
- snapshot selection, repeated snapshot restart, explicit retain/discard disposition, unresolved suffix retention, and committed-suffix protection;
- failed-send availability, bounded UTF-8 diagnostics, health degradation, and fail-closed durability/application errors.

Power-loss-image assertions replay actual durable records after explicit storage reconciliation, independently of the transient repair flag. The separate clean-open assertions exercise live startup admission; they cannot pass merely because Redb observed repair.

## Cohort and repeatability

- Root replica dependency: Redb **4.1.0**, selected by `Cargo.toml` and `Cargo.lock`.
- Observed host kernel: **Linux 6.18.50**.
- Test temporary/build filesystem: `datapool/cargo-target`, **ZFS**, `rw,noatime,xattr,posixacl,casesensitive`.
- Normal build target: `/home/brittonr/.cargo-target/aspen-peer-recovery-20261029`.
- Pinned producer target: `/home/brittonr/.cargo-target/aspen-peer-recovery-octet-20261029`.
- Temporary root: `/home/brittonr/.cargo-target/aspen-peer-recovery-tmp-20261029`.
- Builds used two Cargo jobs, disabled incremental compilation, and disabled dev/test debug information. These are operator build settings, not product path dependencies.

Earlier attempts are not final acceptance evidence: the first fault-disk fixtures attempted recovery on unreconciled repaired images, then incorrectly assumed the first post-error open would be clean. The corrected scenario explicitly reconciles once without executing Raft startup, closes, and proves the durable marker on subsequent clean opens. Strict Clippy also found nested guards, a needless borrow, a modulo predicate, and a unit-struct default call; these were corrected without lint suppression. Final producers and the complete ordered checks were regenerated afterward.

Kache reported an unusable shared cache and disabled caching. Nix/Cargo cache files also reported exhausted storage during an earlier producer run; Nix's operator cache was relocated to the Cargo filesystem. No unrelated cache or historical evidence was deleted. These diagnostics are not durability evidence for Molten.

## Claim boundary

The live backend remains three-voter and non-production-admitted. Larger-membership pure-core regressions do not prove a larger live backend. Recovery progress requires a stable observation window; arbitrary churn, malformed durable histories, arbitrary media loss, and whole-cluster correctness are not established.

The retained [2026-10-28 ChaosControl coverage](2026-10-28-chaoscontrol-campaign-coverage.md) remains model-only evidence. No new ChaosControl guest, VMM fault campaign, or hardware durability claim is made by this closeout. Native Cairn gates and archive receipts establish lifecycle conformance, not behavioral correctness by themselves.
