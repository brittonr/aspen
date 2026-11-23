## Phase 1: Workload and semantic core

- [x] [serial] Define the typed service-registry register profile with exact key, initial value, value corpus, generator, seed, weights, clients, concurrency, operations, consistency model, recovery, and evidence bounds. r[molten.consensus.live_register_profile]
- [x] [serial] Implement the pure bounded read/write generator with deterministic operation plans and complete choice records. r[molten.consensus.live_operation_generator]
- [x] [serial] Implement pure public-operation projection and `ok`, `fail`, `info`, pending, retry, logical-ID, and attempt-ID mapping. r[molten.consensus.live_operation_history]
- [x] [parallel] Add positive write/read fixtures and negative changed-ID, malformed-response, false-definite-failure, unsupported-value, and incomplete-pair fixtures. r[molten.testing.live_reliability_validation]

The [register profile and admission](../../../src/reliability/model/profile.rs) bind one bounded key and the null initial value to the generator, adapter, outcome and register-model identities. The retained [local verification](#local-verification-2026-11-21) includes `live_reliability::profile::tests` for the deterministic seeded plan and denied cross-key and unbounded profiles.

## Phase 2: Public client and deployment adapter

- [x] [depends:chaoscontrol-semantic-history-contract] [serial] Pin the semantic-history v2 schema, register model, checker report, and evidence classes through immutable Nix inputs. r[molten.testing.live_reliability_cohort]
- [ ] [depends:onixos-live-reliability-contract] [serial] Implement the Molten product adapter for setup, write, read, recover, final-read, and teardown through public coordination endpoints. r[molten.consensus.live_public_adapter]
- [ ] [depends:onixos-molten-native-service] [serial] Package the exact multi-node Molten service cohort for disposable OnixOS clusters. r[molten.testing.live_reliability_cohort]
- [x] [parallel] Add a deliberately faulty public fixture for stale reads, lost acknowledged writes, divergence, duplicate application, and stalled recovery. r[molten.testing.live_reliability_validation]

## Phase 3: Live fault and recovery profiles

- [ ] [depends:onixos-live-reliability-contract] [serial] Add no-fault, process-restart, temporary-partition, heal, recovery, and final-read profiles. r[molten.consensus.live_fault_profile]
- [x] [serial] Require final public reads from every admitted endpoint after declared readiness and stable membership facts. r[molten.consensus.live_recovery]
- [x] [parallel] Keep clock, durability, Byzantine, queue, lock, and transaction profiles explicitly unsupported. r[molten.consensus.live_fault_profile] r[molten.testing.live_reliability_claim_boundary]

## Phase 4: Checker and evidence integration

- [ ] [serial] Invoke the pinned register checker over complete live histories and retain valid, invalid, and unknown reports. r[molten.consensus.live_linearizability]
- [x] [parallel] Add optional pinned reference-checker runs and block promotion on disagreement. r[molten.consensus.live_linearizability]
- [x] [serial] Implement a pure fail-closed importer for producer, cohort, profile, generator, history, fault, recovery, checker, witness, teardown, and non-claim facts. r[molten.testing.live_reliability_evidence]
- [x] [serial] Add a thin shell that stores a canonical external live-reliability receipt without granting other evidence or authority roles. r[molten.testing.live_reliability_evidence]
- [x] [parallel] Compare simulation, NixOS VM, ChaosControl KVM, and live profiles without merging their evidence classes. r[molten.testing.live_reliability_claim_boundary]

## Phase 5: Validation and closeout

- [x] [parallel] Add malformed history, incomplete recovery, unobserved fault, stale artifact, checker disagreement, teardown gap, and overclaim rejection fixtures. r[molten.testing.live_reliability_validation]
- [ ] [serial] Run pure adapter and importer tests, public client fixtures, disposable live campaigns, checker conformance, evidence validation, and Nix checks. r[molten.testing.live_reliability_validation]
- [x] [serial] Run Cairn proposal, design, tasks, and validation gates before sync or archive. r[molten.testing.live_reliability_validation]

## Blocker

The provider contracts are no longer missing specifications: ChaosControl's semantic-history contract is archived, and OnixOS retains archived `2026-08-30-add-live-black-box-reliability-rail` and `2026-08-30-onixos-molten-native-service` changes. Historical source, pure/VM controls and checked archive tasks do not establish current-producer admission or a live campaign.

Live behavior still requires the exact current provider integration, independently provisioned signed-output/source trust and credentials, an admitted production-shaped coordination endpoint/native service cohort, actual disposable hosts, complete histories/fault observations, checker results and teardown evidence. The retained v0.1.0 service and an experimental three-node recipe cannot satisfy those requirements. Existing generator, history, importer and public-client source must be reconciled rather than replaced; no unchecked execution criterion is completed by this prerequisite correction.

The documented `nix/live-reliability.nix` was not available from provider `onixcomputer/onix-modules` main at `f6668778464f713abc9d301c0ebedd0628bf79bb` (GitHub contents returned 404). The retained local history for that path points to superseded work `752387f0`, not a currently admitted provider package. Do not launch or qualify that historical integration in place of an exact signed cohort.

## Local verification (2026-11-21)

The root library invocation in pueue task 47 selected zero tests because its filters named source paths, not mounted module names. The retained libtest executable (BLAKE3 `333843ab160a5897195393b3b4ac86209d80f85af7fbfb2bc6dc006cde0659bb`) ran directly with the corrected filters:

```sh
"$MOLTEN_LIBTEST" 'fabric_observability::kernel::' \
  'fabric_observability::nodeadmission::observertests::' 'live_reliability::'
```

71 passed, zero failed or ignored; 61 were reliability tests. The output BLAKE3 was `0f9ec2420cd7e6e1ec2469fc22988b4a2236f7f3b79dc620b9d24f5952c33105`. Checks cover the generator, operation/attempt history, actual loopback public fixtures, experimental recipe and receipt/import controls with prevalidated checker results. Neither a provider fault nor a deployed cohort or pinned external checker ran.

The importer now requires the selected fault policy and, for a disruptive run, the independently verified provider effect reference. A standalone smoke of the exact admission functions accepted a complete partition and an explicit no-fault run, and denied late recovery, a different mechanism or effect reference, and a missing scheduled fault. This exercises pure validation only; deployed evidence and checker conformance remain unchecked.

An isolated smoke of the exact changed recovery functions (harness BLAKE3 `12570ad6f1dd3a6d4a58cf1ac7e023112b9540a83f683d3acdc93549960ccef7`) accepted a complete sampled interval. It denied a one-nanosecond final-read overrun, a short stability interval, pre-recovery samples, and an overstated sample count. Pueue task 56 passed nine importer tests from before the new timing test was selected; it is not that test's execution evidence.

Pueue task 63 selected the exact new regression and stopped at `Fault` before reaching the timing checks: the inherited in-memory fixture described only two endpoints, below the existing three-endpoint minimum. The corrected fixture keeps the original non-final workload and covers three final endpoint reads. Production admission and timing bounds are unchanged.

Pueue task 66 passed all ten tests in `cargo test --locked -p molten --lib live_reliability::evidence::tests::`, including the revised three-endpoint recovery and imported partition controls. The same selection passed `cargo fmt --check` and `cargo clippy --locked -p molten --bin molten -- -D warnings` (log BLAKE3 `74c686ed5642cf0e98e81b57f21769086fff0466fce8c94e16c2032efe79c2ae`). This verifies fail-closed final-read requirements and pure admission with in-memory observations, not execution by an independent provider or a live history.

The real `molten reliability generate` and `molten reliability import` commands then ran on the locked local binary (BLAKE3 `968c6e5d796511770037cf0df57a2c087cb2c919b2113ed9306170b56551996d`). An expected-input document omitting the selected `fault` field (`60ed1d37838672c18dd00bb3e2c3d8cdc69473f27c7506feb83db792c19d1a6f`) exited 1 with a missing-field error for `fault`. The ledger, receipt, and checker history all remained absent. This is a malformed-input CLI control, not an imported live campaign or a successful persistence result.

Task 83 separately exercised the storage shell at its real capability ledger boundary. `cargo test --offline --locked -p molten --lib live_reliability::evidence::tests::receipt_storage_refuses_substituted_identity_and_preserves_canonical_bytes -- --exact` passed one test; a substituted identity left neither artifact reachable, then a canonical fixture receipt was persisted and re-opened unchanged. Formatting and denied-warning Clippy passed (combined log BLAKE3 `6141df861331ac74846f9052e670423808328660ca661b648633cd26bd43b5d2`). Task 82 stopped at formatting before running a test. This verifies local persistence of an admitted fixture, not an observed OnixOS deployment or provider history.

## Pinned checker contract and executed shell control

The existing [`chaoscontrol-conformance-src`](../../../flake.nix) input supplies the semantic-history v2 schema, pure register model, report v1 and evidence classes. An offline Nix evaluation resolved [`flake.lock`](../../../flake.lock) revision `ec8aa799bad4735bf1028a092ca39feb08fb4bb7`, NAR identity `sha256-BE2LOEEOiKEtMbttrpR4qRRh00eFlBlGL45E7epz5VE=`. The local build inputs matched that Nix source byte-for-byte: `semantic_history.rs` BLAKE3 `0677c9fe262f5703bf0c693d7abf57040b8aa231bf8d7d1db60703805b5d7a27` and `semantic-contract.rs` BLAKE3 `3b5e9176986bf29eca25a975bedd09573e3a231cafd6c56669d7d8b7615f32cd`.

`cargo build --offline --locked -p chaoscontrol-evidence --bin semantic-history-contract` on the exact Git archive produced checker BLAKE3 `e87455e339cf5688e4ef7e92b4f1e67d16710940626213d422775a2af75f25eb`; its `selftest` passed. The previously built Aspen binary (BLAKE3 `d63c4a4987143d12312922eb943f73aa5d5a139125266471c4aa26be345e34c5`) then ran `molten reliability check` with that executable, the measured CLI source reference, a fresh work directory per case, 20,000 ms per checker operation and a 1 MiB input/output bound. Its result objects for the pinned `valid`, `invalid`, and `unknown-bound` register fixtures matched the producer's respective reports. A different expected executable digest denied with `checker-identity` before writing the work history or report.

These are source binding, checker and synthetic CLI controls. They are not a hermetic checker package, a live history, a reference-checker run, or an imported provider campaign.

## Selected reference checker regression

On `a5d10a49e` plus only the regression, Pueue task 34 ran `cargo test --offline --locked -p molten --lib live_reliability::evidence::tests::selected_reference_checker_cannot_be_omitted -- --exact`. It failed with exit 101: a complete fixture named an independently selected reference checker, but omitting both its execution and report still returned a receipt (`None` instead of `Some(Checker)`). Log BLAKE3: `6f9c5520a7f775074b0e4b16758e2422494e61e8a002c3c9d8777e7f3a8657f2`.

Pueue task 49 ran the feature-enabled library selection `live_reliability::evidence::tests:: spacewasm::` on the repaired inputs: 44 passed, none failed, and one independently provisioned SpaceWasm case remained ignored (log BLAKE3 `3df9fd1b4b01e48f6c3d4047ee7278d4e5b3beaa4613c5e39e7c248739f9cc19`). The selected-reference omission, unexecuted checker report and checker-disagreement controls all denied. These validate importer decisions over supplied observations; they do not qualify an independent reference executable or a live provider history.

## Immutable executable controls

The checker now reuses one sealed, measured executable image for `check` and `validate`, avoiding in-place changes to an already-open source inode. An initial regression run denied at Rustix's unsupported `AT_EMPTY_PATH` flag combination. The repair checks the same opened descriptor through procfs before creating the image.

An offline locked build from ChaosControl `ec8aa799bad4735bf1028a092ca39feb08fb4bb7` produced checker BLAKE3 `483b2caf9d2dc6edf060cf8ddb843a55d7642892a3f444bd99f3307d4ddcbb5c`; its `selftest` passed. The actual `molten reliability check` ran `check` and `validate` with that image over its synthetic valid, invalid and bound-exhaustion histories. The observed `valid`, `invalid` and `unknown` reports matched the producer fixtures byte for byte; a wrong executable digest denied before writing history or report. The CLI smoke log BLAKE3 is `bc6158201044b7452cf2931f1329406c7034cfb5d7da4efac3c73d24f94bca98`.

The pinned Cairn provider policy passed proposal, design, tasks and strict validation. There was no installed OnixOS history, provider fault, or independently admitted reference executable. No live checker qualification, sync or archive is claimed.

## Paired checker controls

On `012aac0e9c` plus only `duplicate_checker_cannot_count_as_a_reference`, the feature-enabled library test exited 101: the importer accepted a cloned primary report and checker reference in the reference role (`None` instead of `Some(Checker)`). Baseline log BLAKE3: `3a1662a5f13be62ecd512bad0a9a045f1bd7d34912ae082dd908abada14cf392`.

The pre-fix CLI also reached the checker under both roles when the reference selected the same executable under a different reference. The import ultimately denied the deliberately incomplete deployment binding (`evidence-denied:Identity`), but the file-operation trace showed the second run removing the primary `report.json` first. CLI and trace log BLAKE3: `98e220d0a7218b03531c9d381184b24fcbc20577f7ed4ed4fd14226a9085d9e8` and `7a398aba0ad4d7048898a6b443c16cb06b61eda50bbb5bbb368775c796debf01`.

The independent-directory smoke selects two real builds of the same pinned `ec8aa799bad4735bf1028a092ca39feb08fb4bb7` checker source solely to exercise process and file isolation. Their measured image digests are `483b2caf9d2dc6edf060cf8ddb843a55d7642892a3f444bd99f3307d4ddcbb5c` and `9637f4cbc62f7078f0bdda2e7617a1ba2ac9e3a826703a5d39f96fc6b5a793c2`; the latter passed the producer's actual self-test (build log BLAKE3 `f89d49c324f1762bc59cb6848485d3eac9bb041d9f5f90ecc74f097ba3f72e09`). These are diagnostic builds, not independently implemented checkers or an admitted live cohort.

The initial post-fix Octet pass stopped before tests on two workspace findings for an unqualified concrete report type; the library producer had zero findings (log BLAKE3 `fe81a5dd96cd4833411e4d8d13445c53da9bf9e15f85a127e6c93b6a7c6a3044`). The correction qualifies the report type at the CLI use site without changing behavior.

The repaired CLI (`blake3:50a09284a68645d49747347ab06475698b2ba6af718e8b556dd1da57db7f8a75`) denied an aliased image before either checker wrote a report. It ran both distinct diagnostic images on the same history in separate work directories and retained both reports (each BLAKE3 `b271b99bf01abe7ace9e7d771c67edf777c73094b05fc0fcbe42435594f71707`). Reuse of the reserved reference directory denied without changing either report. The incomplete deployment binding still denied; no live receipt or ledger was created.

Both Octet producers returned zero findings. The ordered run assembled 3,169 source inputs, passed `strict-ci` (`blake3:fb24889beb06ba56a2c206d1ff864fa016bc02d2a3259adf71c95f24fc35ff63`), emitted the remediation plan (`blake3:966edd6d8a135b7f5d048f4ad3f7fc8920b15fe7af878d10e5b53c21af980dda`), and passed all 66 `live_reliability::` library tests, including omission, aliasing and disagreement. Formatting, denied-warning Clippy and strict Cairn validation passed. Ordered log BLAKE3: `e200818f74bc86f6cc55f6a129225bd1b54ec28e5b0233d40ec857a242a8555a`.

This completes the optional checker execution and disagreement gate in Aspen. It does not qualify a live history, an independently implemented reference checker, or a provider campaign.
