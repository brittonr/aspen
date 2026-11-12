# Open issue triage

Reviewed at `c952c28c4`. This page records why the remaining public issues are
not silently claimed as implemented. It is a review aid, not a second
specification or a release checklist.

## Closed by the current batch

The following issues have concrete changes on `molten`:

- **#23** public HTTPS dependency sources, regenerated locks, hashes, and plans.
- **#16** sender-root source-trust provisioning in the signed VM script.
- **#24** storage-denial receipt downgraded to explicit unavailable; observed-denial child-evidence rule.
- **#14** provider-throttle class and put-acknowledgement uncertainty.
- **#13** simulated put faults for throttling, torn put, and multipart abort.
- **#7** inbound admission rate-limit finding and loopback-only deferral.
- **#30** security and contribution entry points.
- **#29** Prolly documentation reconciliation and source-sensitive page map.
- **#27** third-party notice revision binding and checker rule.
- **#25** Molten identity, status, and an executed first workflow.
- **#4** Nickel-sourced system atlas with Markdown and static HTML twins plus a freshness check.
- **#22** Read-only capability qualification report joining the evidence matrix with validated receipts.
- **#28** Resolved-closure dependency advisory, source, and license audit with reviewed exceptions.
- **#11** Structured release-dependency rule IDs and supporting facts; keep-Rust/stop-Datalog decision recorded.
- **#8 (partial)** Distinct `ContentAdapterClass::ObjectStorage`, a Nickel profile stub, and a bounded SigV4 S3 put/get/head client with mock-endpoint tests; a live S3-compatible endpoint and verified NVMe tier remain unimplemented.
- **#5** Bounded TLA+ fast-path model with a TLC flake check and a pinned TLA-ts 0.0.10 parse check over the same model; the parse path was exercised locally, while the flake check remains unexecuted because Nix evaluation is blocked.
- **#31 (tracker)** The [external pilot tracker](external-pilot-tracker.md) records the selected-workflow dependencies and distribution checklist; it does not qualify a pilot.
- **#17** Public `promote`, `retry-plan`, `deny`, and `abandon`; signed authority/closure, observed head, atomic store, canonical receipts, and a real CLI fixture for positive plus stale/revoked/incomplete promotion. Effect dispatch remains a separate admitted step.
- **#26 (partial)** The opt-in VM run booted, started, restarted, stopped, and denied an absent trust file; the final assertion was corrected for the module's restart policy, but the corrected full run was blocked by a host Nix auto-GC lock.
- **#18 (partial)** A local logical file-tree adapter now verifies bundle component identities, signed current admission, handles, restore ordering, activation, and receipts through `world-snapshot restore`; typed node-store restoration and opaque/VM runtime admission remain unavailable.

No-go decisions are recorded for **#2**, **#3**, **#6**, **#9**, **#10**, and **#12** in
the [deferred decision record](deferred-decisions.md). They remain open until
their reopening triggers are met.

## Remaining issues and blockers

| Issue | Current blocker | Next honest action |
| --- | --- | --- |
| #2 | No-go decision recorded: prerequisite query and baseline evidence are absent. | Reopen with a measured query requirement. |
| #3 | No-go decision recorded: published bbtree versions pin an incompatible `vstd` cohort. | Reopen with a compatible bbtree source or an isolated benchmark workspace. |
| #6 | No-go decision recorded: no profile output justifies the split. | Reopen after runtime reuse and a measured threshold. |
| #8 | Class, profile stub, feature-gated SigV4 S3 client with a direct-dependency allowlist and mock-endpoint signing tests, plus a bounded local tier composed with the client: local hits make no remote request, remote bytes are verified before caching, and corruption is never cached. Needs a live S3-compatible endpoint, profile conformance, latency receipts, and live/mock parity evidence. A `minio/minio` pull was denied by the registry in this environment. | Obtain a reachable S3-compatible endpoint and implement the bounded NVMe tier. |
| #9 | No-go decision recorded: `ManagedSecret` has no loader; TPM2 needs a swtpm/hardware PCR test. | Implement the sealed backend with a fail-closed denial and rotation fixture. |
| #10 | No-go decision recorded: threat model and disclosure rule are not agreed. | Reopen after the discovery threat model is agreed. |
| #12 | No-go decision recorded: no bounded root-blocker consumer. | Reopen with a world-workflow query need. |
| #15 | Requires two real hosts, actual authenticated peer binding, and a Kiln consumer cohort. | Qualify one direct-LAN stream profile and retain loopback evidence separately. |
| #18 | Local logical file-tree adapter and CLI restore are implemented with restart/readback and idempotent reactivation tests; it restores declared bundle trees, not Molten's typed node stores or a live runtime. Opaque/VM restore still deny. | Bind the adapter to the typed node-store roots and observe the actual destination cohort. |
| #19 | Needs a public-node composition and a non-simulated actor effect port. | Reuse the archived actor profile; do not add a second mailbox or ledger. |
| #20 | Depends on #15 and #19 plus separate state roots and an independent effect observer. | Qualify ownership fencing after the cross-host and actor paths exist. |
| #21 | Needs a pinned Golem deployment and matched durability/recovery semantics. | Define the common manifest before any latency comparison. |
| #26 | The opt-in VM run reached startup, restart, shutdown, and absent-trust denial; the final assertion expected `failed` while `Restart=on-failure` leaves the unit restarting. The corrected full run is blocked by a host Nix auto-GC lock; repeated `nix eval` probes time out, including a 30-minute uncached/ cached attempt. | Rerun the corrected opt-in VM app when the Nix store is idle and retain the exact package/profile result. |
| #31 | Tracker added at `docs/external-pilot-tracker.md`; depends on #15, #16, #17–#19, #22, #26, #27–#30. | Keep it as the release-readiness checklist, not an implementation task. |

## Boundaries

No live hardware, VM, cross-host, Golem, or advisory-database run was executed
for this triage. A green source check, fixture, or archived receipt is not
substituted for those observations. The completed batch is scoped to the
changes and checks reported in the adjacent commits.
