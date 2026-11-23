## Tasks

- [x] [serial] Define the typed Nickel `spacewasm-core-mvp-pilot` profile, exact runtime/materialization/evidence cohort, module/ABI/resource limits, segment/yield policy, retention, promotion state, and non-claims. r[molten.spacewasm_core.profile] r[molten.spacewasm_core.materialization] r[molten.spacewasm_core.resources]
- [x] [depends:spacewasm-core-profile] Implement pure profile, bundle, Octet/Chaos evidence, module feature, import/export, ABI, authority, resource, and promotion admission with deterministic blockers. r[molten.spacewasm_core.functional_core] r[molten.spacewasm_core.admission]
- [x] [depends:spacewasm-core-profile] Define continuation, segment-plan, execution-observation, replay-comparison, and receipt DTOs with generation/artifact/profile/input/effect bindings. r[molten.spacewasm_core.resume] r[molten.spacewasm_core.receipts]
- [x] [depends:spacewasm-core-admission] Add a thin SpaceWasm shell that consumes a verified Mantle bundle, streams exact module bytes, links only declared `molten:hostcall` functions, and implements bounded `molten.wasm.abi.v1` Preserves memory transfer. r[molten.spacewasm_core.execution] r[molten.spacewasm_core.abi]
- [x] [depends:spacewasm-core-shell] Enforce separate interpreter/code/stack and guest-linear-memory/table/growth/instruction bounds with typed OOM, denial, trap, and harness outcomes. r[molten.spacewasm_core.resources]
- [x] [depends:spacewasm-core-shell] Add generation-fenced finish/trap/out-of-fuel/host-pause handling, bounded continuation retention, stale-resume denial, and explicit cleanup. r[molten.spacewasm_core.resume]
- [x] [depends:spacewasm-core-resume] Emit inspection, admission, instantiation, segment, yield/resume, hostcall, resource, terminal, replay, and denial receipts with engine-specific identities and non-claims. r[molten.spacewasm_core.receipts]
- [x] [parallel] Add positive exact-bundle, admitted MVP, Preserves ABI round-trip, declared hostcall, expected trap, out-of-fuel/resume, uninterrupted/segmented replay, and cleanup fixtures plus negative missing/stale evidence, unsupported proposal, component/WASI, undeclared import, wrong ABI, invalid pointer/length, malformed output, memory/growth/stack/instruction exhaustion, stale continuation, host trap, replay mismatch, fallback, and overclaim fixtures. r[molten.spacewasm_core.fixtures]
- [x] [depends:spacewasm-core-receipts] Add operator status/readback that keeps SpaceWasm pilot, Wasmtime core, and component evidence distinct and reports promotion blockers. r[molten.spacewasm_core.nonclaims]
- [ ] [depends:spacewasm-core-fixtures] Run focused pure-core/shell/ABI/resource/resume/replay tests, executor conformance, positive and negative fixtures, Octet and Chaos evidence checks, Cairn validation, proposal/design/tasks gates, and the smallest relevant workspace/Nix checks. r[molten.spacewasm_core.validation]

## Prerequisite status (2026-11-16)

The engine prerequisite is closed: Molten and the shared hardened runtime are both on the
patched Wasmtime 49.0.5 cohort (archived `adopt-patched-wasmtime-line`), so the SpaceWasm
differential no longer needs an advisory exception. Remaining before this change can execute
evidence-bearing acceptance:

- Mantle's independently admitted immutable revision/bundle publication binding. Its 2026-10-30 local integration verified bundle `774331f6c2defe75e30c354b2b3080838cf56438bbd99268779fe4590f8c9f39`, but `materialize-molten-spacewasm-abi` still leaves publication unchecked; a local materialization is not an authorized trust pin;
- exact-member static receipts and independently selected immutable verifier identity for Octet's archived `spacewasm-molten-abi-pilot` profile (`b3:2fd08aba3959f85f58aaac323758df3e286872cf14e7b52985a1b8d7c4647956`);
- a compatible ChaosControl full-Molten-ABI differential producer/report and independent verifier/probe pins. The inspected archived/current rail is hostless, uses `chaoscontrol.spacewasm-mvp-differential-report.v1` and Wasmtime 41.0.3; it is not the required `chaoscontrol.spacewasm-molten-abi-differential.v1` / Wasmtime 49.0.5 cohort.

## Local verification (2026-11-21)

Pueue task 46 ran `cargo test --locked -p molten --features spacewasm-pilot --lib spacewasm::`: 31 passed, one independently provisioned case ignored. The pinned Nickel CLI also exported the checked authoring profile after the contract-export shadowing repair; `production` and `release` remained `false`. Neither check provides the missing independent publication, exact-verifier, or full-ABI differential evidence above.

The November 21 prerequisite readback found no admitted bundle: Mantle revision `aa81952cce99ac7bb5e2bff95a97bcb4b785b8ca` left immutable publication and consumer execution unchecked, Octet revision `3ddd21ef671f206537d42ad2289467b530b96255` retained fixture-only static receipts, and the checked ChaosControl differential still declared `chaoscontrol.spacewasm-mvp-differential-report.v1`.

## Checked profile ceilings

Nickel 1.17.0 exported the default profile and accepted the declared ceiling, then denied ceiling plus one, independently for `input_bytes`, `output_bytes`, `continuation_count`, `total_fuel`, `retention_turns`, and `continuation_bytes`. Production and release remained false.

On `a5d10a49e` plus only the regression, Pueue task 39 ran `cargo test --offline --locked -p molten --features spacewasm-pilot --lib spacewasm::profile::tests::checked_profile_limits_reject_out_of_envelope_requests -- --exact`. It failed with exit 101 at `input_bytes above the bound accepted` (log BLAKE3 `be04b05649b7f71558794783c5c5f21f07b45cb2890846d25def73546d4f5119`). This reproduced the deviation at Rust admission without any guest allocation or external execution.

Pueue task 49 ran `cargo test --offline --locked -p molten --features spacewasm-pilot --lib -- live_reliability::evidence::tests:: spacewasm::` with all six Rust ceilings enforced. The repaired regression passed at each ceiling and denied ceiling plus one. The combined selection passed 44 tests with one independently provisioned replay case ignored (log BLAKE3 `3df9fd1b4b01e48f6c3d4047ee7278d4e5b3beaa4613c5e39e7c248739f9cc19`). This closes the Nickel/runtime envelope discrepancy, not the external materialization or differential evidence gates.

## Operator status control

The feature-enabled `molten spacewasm show` read a canonical diagnostic admission-denial receipt (`blake3:bc29685ba897d573e68872115fac24132f6ebf73f97b1a137428c64ee21e4f7e`). The passing expected-denial fixture reported its pilot profile, no production evidence, and the promotion blocker. Reading those same bytes as an execution receipt denied. The smoke log BLAKE3 is `bc6158201044b7452cf2931f1329406c7034cfb5d7da4efac3c73d24f94bca98`. The pinned Nickel pilot profile exported with `production` and `release` false. This verifies readback, not an admitted Mantle execution, differential result, or promoted receipt.

## Segment accounting control

The engine records one `spacewasm-event-v2` segment row per real runner call, with the logical turn, segment/total ceilings, grant, and cumulative work before and after. It does not invent a grant for lifecycle rows or a segment for a pre-execution denial. The feature-enabled selection passed all 71 local tests, including early finish, truncated final grant, wrong binding or resource accounting, and normalized replay comparisons. The externally provisioned replay test remained ignored (log BLAKE3 `b0267f6eef7b83963deb4f3d87e34f85083948ff7bc86d23bc9211b819b65951`). This verifies the DTO and consumer accounting, not execution on the exact external fixture bundle.

## Continuation turn control

On `e682822a9e` plus the two local regressions, `retained_turn_rejects_clock_regression_and_expiry` failed with exit 101: the actual interpreter returned `Finished` on a regressed logical turn that was still within its creation-based retention interval, instead of `Denied("stale continuation")`. The failing selection's log BLAKE3 is `280e494c8739e61c2bbf688f8e58c925c647189f0fd7cf9280be498ea874fef8`. The hostless test uses a local fixture admission and cannot establish independently admitted Mantle, Octet or ChaosControl evidence.

The repaired engine admits another segment at the same turn through the original retention limit. It rejects both regressed and expired resumes or cancellations before counting more work and releases guest resources and the local allocation guard. The actual-engine regression passed in the [ordered source run](../add-linux-ebpf-observability-adapter/tasks.md#fabric-queue-budget-control). This closes the continuation implementation task; external fixture admission, execution and replay remain open.

## Full-ABI reference runner

ChaosControl [`a2b85741`](https://github.com/brittonr/chaoscontrol/commit/a2b85741c341863798850fa3c82d19a22e4912b3) supplies the opt-in hostcall model and Wasmtime 49.0.5 reference runner. Its [producer evidence](https://github.com/brittonr/chaoscontrol/blob/a2b85741c341863798850fa3c82d19a22e4912b3/.cairn/changes/add-spacewasm-full-abi-differential-rail/tasks.md) records 49 library and 13 binary tests and explicitly limits them to synthetic reference-runner fixtures. The SpaceWasm runner, release-authority publication, exact-member scenarios, two-engine report and independently selected verifier inputs are still required. No Aspen admission, replay or promotion task is completed by this checkpoint.

## Checked operator authoring

[`OperatorConfiguration`](../../../config/spacewasm/contracts.ncl) checks one explicit pilot profile, three immutable verifier selections, published-identity fields, capability-relative artifact paths, and the fixture/scenario pairing. It does not supply or admit a publication pin.

The offline `checks.x86_64-linux.spacewasm-operator-profile` derivation realized `bv2ksrjyk43npalhby6cqw8vwlgbf6wq-molten-spacewasm-operator-profile`. Six scenario shapes exported byte-identically with local Nickel (`44cb1bbec8aa2129a582deca627807a9caff7c8cc01f23067e276055cc4657ae`); ten controls denied missing cohort or probe pins, malformed verifier or Octet digests, ambient verifier paths, traversal, unknown fields, engine substitution, promotion, and a misbound host scenario (`7faabbb1aa7ddf62084fca12e5a3dd037f2e518b5a84418126c070a0f6d7bb46`).

The retained feature-enabled `molten` binary (`50a09284a68645d49747347ab06475698b2ba6af718e8b556dd1da57db7f8a75`) parsed each export via `spacewasm run-fixture`, denied the unavailable bundle in each case, and denied an independently corrupted probe pin before opening the bundle. No status receipt or differential report was written (combined Nix and command log BLAKE3 `edcf94caa43c44de16f92efeb6f240c2b3b11ec57a7c741b4d215e8497297b1a`). The exact independently published cohort remains unselected. These checks establish the authoring and fail-closed CLI boundaries, not a positive external execution or replay.

## Cohort binding and admission closeout (2026-11-23)

The pilot profile now binds one fixed cohort instead of leaving the evidence identities restated in checks: `wasm1` module features, the `mantle-spacewasm-reference-bundle-v1` materialization profile `nasa-spacewasm-e24cf093-reference-v1`, the Octet `octet-evidence-rail-receipt/v1` profile `spacewasm-molten-abi-pilot` with runtime cohort `b3:7c537eef…`, and the `chaoscontrol.spacewasm-molten-abi-differential.v1` reference run on `wasmtime` `49.0.5`. The cohort is hashed into `molten-spacewasm-profile-v1`, and `evidence.rs` reads those identities from the profile constants, so a receipt naming one profile cannot be replayed under another cohort.

`checked_profile_cohort_rejects_every_substitution` mutates each of the nine cohort entries and requires both admission denial and a changed profile identity. Exporting the authoring profile with any one entry drifted was denied by the Nickel `Profile` contract in all nine cases.

`cargo test --locked --offline -p molten --features spacewasm-pilot --lib 'spacewasm::'` passed 37 tests with the independently provisioned `configured_replay_smoke` ignored (log BLAKE3 `76d5158b3f552c7aaa8274f6dd5f220169fe8e2185f035e72a884390f1e72a23`). That selection covers profile and engine blockers, bundle/manifest and Octet identity substitution, stale static artifacts and error findings, hostless members, wrong or post-MVP ABI, ambient and undeclared hostcalls, growth and allocation bounds, terminal and replay receipt agreement, promotion overclaims, and ambient path or oversized receipt readback. The formatter-clean source re-ran the same selection and passed 37 tests (log BLAKE3 `4f38896a31f4280b369ab521d430f8027a0c906f2b0f0f7f09c29badef8b6a74`); denied-warning library Clippy passed (log BLAKE3 `9ad41e424e64a010dca64508d32b3d03607aa722305cdb36bc001bf69f88f678`). The fixtures are local; no Mantle publication pin, Octet receipt, or ChaosControl differential was executed, and task 12 stays open for that conformance.
