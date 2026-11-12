# Open issue triage

Reviewed at `dae9d492e`. This page records why the remaining public issues are
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
- **#8 (partial)** Distinct `ContentAdapterClass::ObjectStorage` and a Nickel profile stub with explicit capabilities; live S3 client and NVMe tier remain unimplemented.

No-go decisions are recorded for **#2**, **#3**, **#6**, **#10**, and **#12** in
the [deferred decision record](deferred-decisions.md). They remain open until
their reopening triggers are met.

## Remaining issues and blockers

| Issue | Current blocker | Next honest action |
| --- | --- | --- |
| #2 | No-go decision recorded: prerequisite query and baseline evidence are absent. | Reopen with a measured query requirement. |
| #3 | No-go decision recorded: published bbtree versions pin an incompatible `vstd` cohort. | Reopen with a compatible bbtree source or an isolated benchmark workspace. |
| #5 | No pinned TLC/TLA-ts tooling in the flake; external TLA+ results do not prove Molten behavior. | Pin a TLA+ toolchain and add a bounded model plus CI exploration. |
| #6 | No-go decision recorded: no profile output justifies the split. | Reopen after runtime reuse and a measured threshold. |
| #8 | Class and profile stub implemented; needs the injectable S3-compatible client, verified NVMe tier, and live/mock parity evidence. | Start with the mock transport from #13 and the new object-storage profile. |
| #9 | Requires TPM2 hardware and a sealed-key backend; the current host has no qualified TPM path. | Implement the pure key-backend contract first, then a hardware rail. |
| #10 | No-go decision recorded: threat model and disclosure rule are not agreed. | Reopen after the discovery threat model is agreed. |
| #12 | No-go decision recorded: no bounded root-blocker consumer. | Reopen with a world-workflow query need. |
| #15 | Requires two real hosts, actual authenticated peer binding, and a Kiln consumer cohort. | Qualify one direct-LAN stream profile and retain loopback evidence separately. |
| #17 | Needs current authority/policy/head/intent composition; the public command still denies by design. | Compose the existing promotion service over concrete current-facts adapters. |
| #18 | Needs an admitted logical runtime adapter; the current denial is deliberate. | Implement one concrete restore adapter and keep opaque/VM restore denied. |
| #19 | Needs a public-node composition and a non-simulated actor effect port. | Reuse the archived actor profile; do not add a second mailbox or ledger. |
| #20 | Depends on #15 and #19 plus separate state roots and an independent effect observer. | Qualify ownership fencing after the cross-host and actor paths exist. |
| #21 | Needs a pinned Golem deployment and matched durability/recovery semantics. | Define the common manifest before any latency comparison. |
| #26 | `nixosModules.molten` and a module-eval check are implemented for the bounded-loop profile; VM boot, restart, shutdown, and absent/wrong trust denial are not run. | Run the module through the NixOS VM rail and retain the exact package/profile result. |
| #31 | Tracker depends on #15, #16, #17–#19, #22, #26, #27–#30. | Keep it as the release-readiness checklist, not an implementation task. |

## Boundaries

No live hardware, VM, cross-host, Golem, or advisory-database run was executed
for this triage. A green source check, fixture, or archived receipt is not
substituted for those observations. The completed batch is scoped to the
changes and checks reported in the adjacent commits.
