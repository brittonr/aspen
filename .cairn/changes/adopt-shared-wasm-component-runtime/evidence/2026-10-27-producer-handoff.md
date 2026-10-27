# Real Mantle materialization handoff — 2026-10-27

## Disposition and ownership

**Blocked, not adopted.** The actual immutable Mantle pipeline succeeds and its own published-file remeasurement passes. Its published consumer verifier then reports the same bundle's directory source closure `missing`, while all **36** regular-file member observations match. Molten's independent private-snapshot intake also accepts only regular files. Consumer installation cannot satisfy the unchanged production-materialization boundary under this handoff.

Immediate outcome: replace a presumed available production handoff with an executed producer/consumer counterexample and explicit blocked metadata. Durable contribution: retained actual publication bytes, producer and consumer reports, observer provenance, and a precise owner handoff. Mantle's component pipeline/verifier maintainers own the producer repair and immutable publication; Molten's component-runtime maintainers own the later pin, capability-rooted snapshot integration and consumer acceptance.

No product source, dependency pin, profile/WIT, effect activation, source-policy allowance, debt baseline or accepted requirement changes. Existing unchecked tasks remain unchecked; the change is unsynced and unarchived.

## Immutable inputs and actual execution

- Producer, implementation core and published consumer verifier: Mantle `c99556e5c91dfd13fcc261dc451f471359d47c38`, the same revision already pinned by Molten.
- Exact producer toolchain: `/nix/store/vvxfl9d1b9a944ca7miswhnxg06cs6jl-mantle-wasm-component-toolchain-v1`.
- Executed immutable Nix compatibility check:

  ```sh
  nix build github:OnixResearch/mantle/c99556e5c91dfd13fcc261dc451f471359d47c38#checks.x86_64-linux.wasm-component-toolchain-compatibility \
    --no-link --print-out-paths --option builders '' -L
  ```

  Passed as `/nix/store/ci56zs6lfsp25wwjry24baxdyfjgmxil-mantle-wasm-component-toolchain-compatibility`.

The disposable observer included the unmodified pinned `tests/wasm_component_cli.rs` fixture constructor, called the real public `run_component_pipeline`, then called both producer remeasurement APIs and the published `ConsumerRoot::verify_bundle`. This was an executable outside the test runner, not a synthetic bundle or substituted report. It used the producer's `demo:app` fixture, **not** a matching Molten world/profile or a production-authorized actor/extension. Its purpose was to isolate producer/verifier compatibility independently of missing product admission.

The observer's small standalone dependency closure is recorded in [its generated lockfile](2026-10-27-producer-handoff/observer-Cargo.lock). Its path dependencies point at the fetched immutable source; this is an observation harness, not a product dependency override or a release-cohort claim. The [observer source](2026-10-27-producer-handoff/observer-source.txt) and [manifest](2026-10-27-producer-handoff/observer-manifest.txt) are inert provenance captures, not installed scripts. The upstream included source and generated fixture material remain attributable to the cited Mantle revision under the repositories' licenses.

The successful invocation was:

```sh
env MANTLE_WASM_COMPONENT_TOOLCHAIN=/nix/store/vvxfl9d1b9a944ca7miswhnxg06cs6jl-mantle-wasm-component-toolchain-v1 \
  TMPDIR=/tmp/aspen-mantle-boundary-witness-20261027/tmp \
  /dev/shm/aspen-mantle-boundary-target-20261027/debug/aspen-mantle-boundary-witness \
  /tmp/aspen-mantle-boundary-witness-20261027/producer-output
```

Observed, exit zero:

```text
producer_status=succeeded
producer_blockers=[]
producer_remeasurement=pass
source_closure_is_directory=true
source_closure_digest=e3c55a6434da88070b9c2338efc5b2553da1c2ceae3c469b735f08cd276f20aa
source_closure_size=4145
consumer_structural_status=Verified
consumer_byte_status=Blocked
source-closure=Missing
other_member_observations=36 Matched
stage_receipts=17
bundle_identity=93a2e552582646f7c06a299296a33318fb0fbd1913471702a8ecd2efe939d19f
```

Raw [execution report](2026-10-27-producer-handoff/execution-report.json) and [consumer report](2026-10-27-producer-handoff/consumer-report.json) are unchanged producer/API outputs. [The publication snapshot](2026-10-27-producer-handoff/producer-snapshot.zip) retains all **42** actual published files, including the complete tiny source tree, under their original root-relative logical paths. It is a transport of observation bytes, **not** an archive substituted for `source_closure` and not an admitted bundle. To replay the byte observation, extract into a fresh private root and use that directory as `ConsumerRoot`; keep the bundle locators, framed source identity and report contents unchanged. The captured observer can also reproduce the complete pipeline with a fresh same-parent output/scratch location. Relocate only its scratch manifest/include locators to a fetched checkout of the exact cited revision.

BLAKE3 measurements:

```text
4f73a5937a2edc8706b8dad694c02761f41d1267039709e7e9f3fd6d7d7cb920  execution-report.json
b8e81a5e8f47107a6bfa147c5111e24e693859627b92bb09805b13715b7ca35e  consumer-report.json
c701aa6901f614842cb2ec33d7a9b631e31f6471703bc82ee1d1ea9fce7bc7ed  producer-snapshot.zip
18cd1d7e0c92d8ed8cdcbf535037aa1c19344c1d97ea59512116b3de3638bec5  observer-Cargo.lock
```

The first observer compilation required explicit conversion of the producer's `ShellError`, which does not implement `std::error::Error`. The first runtime attempt supplied different scratch/output parents and was refused before publication; the corrected call uses one atomic publication parent. The next attempt placed scratch/source under `/dev/shm`, hidden by the pipeline's bubblewrap `/dev` setup; it produced the retained [blocked package-resolution report](2026-10-27-producer-handoff/dev-shm-blocked-execution-report.json). Moving only task-owned runtime scratch/publication to `/tmp` allowed the real pipeline to complete. These observer/host attempts are not consumer acceptance or producer fixes. Host kache reported disk I/O errors and disabled caching; cache policy was not changed.

## Mechanism mismatch

At the exact Mantle revision:

1. [`publish_bundle_inputs`](https://github.com/OnixResearch/mantle/blob/c99556e5c91dfd13fcc261dc451f471359d47c38/crates/crunch-wasm-component/src/materialization.rs#L118-L145) publishes `inputs/source` as a directory. [`copy_source_tree`](https://github.com/OnixResearch/mantle/blob/c99556e5c91dfd13fcc261dc451f471359d47c38/crates/crunch-wasm-component/src/files.rs#L27-L80) hashes bounded, sorted framed directory/file records and counts member content bytes. This is not BLAKE3 over a serialized archive or one regular file.
2. The producer's [`remeasure_object`](https://github.com/OnixResearch/mantle/blob/c99556e5c91dfd13fcc261dc451f471359d47c38/crates/crunch-wasm-component/src/materialization.rs#L294-L320) explicitly distinguishes directories and files and reuses that tree mechanism.
3. The published consumer's [`measure_path`](https://github.com/OnixResearch/mantle/blob/c99556e5c91dfd13fcc261dc451f471359d47c38/crates/mantle-wasm-consumer-verifier/src/shell.rs#L180-L210) reports every non-regular file `Missing`. The structurally accepted directory therefore prevents a verified byte layer despite its presence and correct producer identity.
4. Molten's [independent measurement](../../../../src/wasm/component/evidence/materialization/published/measurement.rs) also requires `metadata.is_file()` and `metadata.len() == size`; [complete-report admission](../../../../src/wasm/component/evidence/materialization/published.rs) requires agreement with the actual published Structural+Bytes report. Fixing only Molten cannot turn this pinned consumer report into `Verified`.

No workaround may flatten the directory, omit the source role, rewrite locators/digests, manufacture a positive bundle or upgrade Structural into Bytes. The repair must preserve the producer's tree identity, finite bounds, no-follow capability-rooted observation, private snapshot and complete layer/report agreement.

## Product composition and unchanged baseline

Source inspection follows the actor harness's [admitted dispatch](../../../../src/harness/parts/runner/p001/body.rs) into the [legacy module executor](../../../../src/harness/wasm/parts/executor/p000/body.rs): `wasmtime::Module`, core-module exports and the existing hostcall/Preserves ABI. The extension fixture's [WasmProbe and EchoExecutor](../../../../src/system_extension/parts/fixture/p000/body.rs) instantiate a core `(module ...)`, not a Component Model instance. Neither path is a shared-runtime Component Model installation. Public component execution remains local; the shared adapter remains `cfg(test)`.

Executed from the dedicated Molten worktree in the pinned Nix shell:

```sh
cargo test --locked -p molten --lib -- \
  wasm_component wasm_executor wasm_preserves_abi system_extension::tests
```

**94 passed, zero failed or ignored**. This overlapping selection covers the unchanged component/parity, host-adapter, legacy actor-ABI and extension fixtures; it does not establish the missing consumer handoff.

The actual previously built CLI was also exercised outside the test runner:

- `molten test run examples/wasm-hostcall.preserves --report-out ...` and `molten test report validate ...` passed. Report/replay ref `blake3:34eabb754734d48dfcf9cf1c216a8a5baf8c24901686b513108c584e6a056a3d`; final state `blake3:9d673b966d375f5f7aa7c45241cbc2c9366adca900087f623ce7f47ed5bc5c32`.
- `molten system-extension run-fixture --profile sandboxed-component --out ...` and `molten system-extension show --status ...` passed. 68 evidence records; final status `blake3:97caee679618dc6368102301c9ef8914417bd73b43e37eb4464e52e0f3a9ddfb`, phase `stopped`, 13 invocations, one restart.
- Executable byte identity `262f25349cb23f0bcc1f74ccb5c36c3732d553efbbd99fbac16d8be09e07c458`. These are observations of the existing binary, not a new binary's source-gate or shared-consumer claim.

## Resume condition

Mantle must complete and publish a producer/consumer contract that verifies genuine directory source closures with exact framing, bounds, role/substitution denials and symlink/drift refusals. Molten then pins that immutable publication, implements its descriptor-relative bounded tree snapshot integration, and reruns the real producer-to-consumer handoff before actor/extension wiring. Keep profile/WIT and effect activation independent: `execution_admitted=false` remains required. Operated deadlines, complete product parity, real consumer installation and coupled cutover/rollback remain open.

This record claims no Wasmtime, compiler, component, actor, extension or effect correctness; no host isolation, source trust, authority, release eligibility or completed adoption. No permanent tests or runtime implementation were added for this investigation. Full workspace/source-gate, full flake, VM and optional-backend checks were not rerun or claimed; evidence-only document/lifecycle checks and required publication hooks are recorded separately when completed.

## Evidence-only validation

The four touched Markdown documents resolved all **34** local link/anchor targets. The retained ZIP passed CRC checking and exact byte comparison for all 42 original publication files; no empty source directories were omitted. Strict native Cairn validation with the explicit sibling-provider policy passed with empty findings/issues and policy BLAKE3 `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`. This validates the blocked lifecycle disposition's structure, not product adoption or the truth of arbitrary external evidence. Publication hooks remain separate observations, not completion authority.
