# Immutable boundary-parity continuation — 2026-10-27

## Outcome and claim boundary

The published v4 prerequisite corrects the original frozen start-work and memory-admission counterexamples without changing Molten's admitted synchronous/time-sliced profiles or WIT. The original component bytes and full comparison oracle remain: whole decisions, canonical output, typed receipt inputs, payloads, BLAKE3 identities, stage parents, successful fuel and callback ledgers.

The candidate remains `cfg(test)` only. Production synchronous execution, time-sliced callbacks and existing actor/extension paths still use the complete local runtime. No product effect world, shared-runtime consumer installation, matching positive production Mantle bundle, operated product deadline, cutover or rollback is claimed. Do not sync or archive `adopt-shared-wasm-component-runtime`.

Immediate outcome: a compatible immutable producer and executed unchanged-cohort parity for the two former blockers. Durable contribution: explicit initialization draining, checked independent memory translation and positive/negative real-component regressions. Molten's component-runtime maintainers own this adapter, product admission, receipt/ledger mapping and repeatable acceptance. Hardened-runtime maintainers own execution mechanics; Mantle maintainers own their materialization schema/verifier.

## Immutable source and completed producer evidence

- Both hardened runtime packages now resolve canonical delegate-qualified Radicle revision `cf4a1913f7f2ca7eb7f655663e87b8b72d0c506c`, contract `hwr.shell.v4`, Wasmtime 45.0.0.
- The producer's native `preserve-component-execution-boundaries` package was reviewed, synced, archived and published to canonical Radicle main. Push explicitly reported no seeds; remote replication is not claimed.
- Cargo fetched that published revision without a source-path override. Targeted Nix locking fetched the same revision; metadata-free NAR hash is `sha256-i+N+OVTdZk7t68IkVYj2EzS/8UfgN3YQ1DjFdGg3+Xg=`.
- Cargo manifest/lock, Nix input/lock, Nickel dependency policy, source hashes, both regenerated unit2nix plans and license notice select the same source. Mantle remains `c99556e5c91dfd13fcc261dc451f471359d47c38`.
- The existing stable archive-receipt identity now points at `.cairn/archive/1970-01-01-preserve-component-execution-boundaries`; it binds completed tasks, actual shell contract bytes and producer verification:

```text
d4a6554a7a61162d339bf9dc515be4e5725847ef8cb7c5b3167e1543fb8aea87  tasks.md
510b2cf96efd92c096d8d780a844e272c7811317aee3dc94fed891df89f59fa0  crates/hardened-wasmtime-shell/src/lib.rs
ad9639e51a32ab13ff4aaae009c96155cd7d11d23e0c79c3864fbe467f127e04  verification.md
```

Producer checks passed 8 core, 30 admission and 44 actual execution cases, strict Clippy, formatting, deny-all Octet, strict native Cairn and all three host Nix rails. A real public-API executable outside the test runner observed seven typed initialization yields, exactly one checkpoint with 9517 remaining fuel, then `Completed` with exact `OK` bytes and 9501 fuel matching synchronous execution. Actual module growth independently denied at `(per-memory, aggregate)=(2,1)` and `(1,2)` and succeeded at `(2,2)`. Its temporary source was removed. These are producer mechanism observations, not Molten production execution.

## Reviewed mapping and exercised behavior

`runtime/shared/sliced.rs` drains initialization through the public successful checkpoint under the same finite `total_budget_fuel / quantum_fuel + 1` poll bound as the local shell. Typed initialization yields never enter the callback scheduler or ledger. A failed start returns Inspection → Denial before exposing a callback handle. Invocation then resumes the same future/store, with no guest restart, fuel refill, quantum reset or second local engine. Terminal measured fuel retains the local final-slice convention, including initialization work.

`runtime/shared/configuration.rs` preserves the per-memory byte/page ceiling. Its explicit aggregate bound is the checked per-memory pages times the already admitted memory count. This is a redundant upper bound over the existing local policy, not multiplication of a product grant: every individual memory and the total memory count still have the unchanged product ceilings. Zero, misalignment, multiplication overflow and aggregate `u32` overflow deny construction rather than truncating or saturating.

Actual fixtures exercised:

- Original 3000-iteration start work: equal callback boundaries, complete ledgers, canonical outputs and full receipts for Actor and SystemExtension labels.
- Original two fixed 129-page memories: equal successful output, Inspection → Instantiation → Execution, whole canonical receipts and successful fuel.
- Actual 256/257-page per-memory and 4/5-memory count boundary pairs: byte reinspection agrees with supplied facts before local/shared admission comparison. Over-bound cases deny rather than hiding behind stale-fact errors.
- Trapping start: both constructors deny before returning a callback, with the same complete Inspection → Denial records and `component-instantiation-denied` class.
- Existing retained execution, cancellation, cleanup, malformed output, pointer/transfer precedence, fuel, stale evidence, wrong world, undeclared imports, loose production bytes, fallback and receipt-overclaim cases remain in the corpus.

The first integrated focused run passed 60 cases and exposed one obsolete v3 test that expected a trapping start to return a lazy shared handle. That incidental deferral test was deleted, not re-pinned. The stronger constructor-denial regression compares both paths and complete parented records; no duplicate same-path case remains. Initial Octet warning-only evidence named two ancestor-word test names. Both names were corrected without an allowance. The unsuccessful workspace evidence remains in the source-gate directory; warning-only is not accepted evidence.

## Unchanged product inputs

These independently remeasured BLAKE3s match the preflight:

```text
5bea351924145de8946669680125e99c837fc61ac6737e114ba3b9edf7ea0b89  docs/wasm-component-runtime/generated/profile.json
48b088f86827c4fa31ffe6c2151dd32a8db53b3f38afef2b2e1b394d43d20852  docs/wasm-component-runtime/generated/time-sliced-profile.json
83be349bb27975ada30dbe60817c5404df7862babdf02ae229b399886e76d5e8  wit/molten-component-runtime/runtime.wit
```

## Executed continuation checks

Commands ran from the implementation worktree with the repository's pinned Nix development shell, executable task-owned `/dev/shm` temporary and Cargo target directories, two Cargo build jobs, and debug information/incremental compilation disabled. The canonical checkout supplied tools, not a dependency source override. Cargo resolved the published immutable producer.

| Executed surface | Observed result |
|---|---|
| `cargo test --locked -p molten --lib -- wasm_component` | 60 passed, zero failed or ignored. The candidate is test-only; this is not a production backend installation. |
| `cargo test --locked --workspace --lib --quiet` | Molten 1760 and Molten Core 629 passed; the other two library targets had zero cases. Zero failed or ignored cases. Not the integration-target or optional-feature suite. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed without allowances or policy relaxation. |
| `cargo fmt --all --check`; `nixfmt --check flake.nix` | Passed. The initial combined verification reached an unformatted new Nix assertion; canonical `nixfmt` corrected it before the final checks. |
| Pinned workspace and `-p molten -- --lib` Octet producers | Both final statuses are `clean`, exit zero, with zero findings, warnings or errors. Producer toolchain remains `nightly-2026-03-21-x86_64-unknown-linux-gnu`. |
| Real CLI artifact import, `strict-ci` gate and remediation plan | Import decision `pass`, six artifacts imported; gate `pass`; canonical remediation plan emitted. |
| Inherited Tracey guard | `pass`: 2966 requirements, 1032 referenced, 1934 unchanged baseline entries, zero dangling references. Five pending references confer identity resolution only, not accepted coverage. |
| Strict native Cairn validation with the explicit sibling-provider policy | Passed with empty findings/issues. Policy BLAKE3 `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`; lifecycle structure is not behavioral acceptance. |
| `scripts/git-source-hashes.sh --check`; Nix `release-dependency-profile` and `git-source-hash-binding` | Passed. Nix realized `/nix/store/18a5ffcm8wrz6h23s0jj5bg5saxpvar5-molten-release-dependency-profile` and `/nix/store/n0n8xr7d27ah13y4axk1pcvd1qca776n-molten-git-source-hash-binding`. Full flake checks, VM integration and release promotion were not run. |

The final required source sequence was both clean Octet producers, object-corpus receipt, actual CLI import/gate/remediation, workspace library tests, strict Clippy, and strict Cairn validation. Final emitted command/status/summary/provenance files and canonical CLI receipts are retained under [`2026-10-26-boundaries-source-gate/`](2026-10-26-boundaries-source-gate/), alongside the unsuccessful producer attempt. The full corpus command selects recursive `src` plus `src/job/dag.rs`, `src/main.rs`, `src/node/daemon.rs`, `src/node/runtime.rs`, `src/octet/gate.rs` and `src/upgrades/mod.rs`; the gate was not weakened.

Canonical CLI identities, not raw rendered-file hashes:

```text
artifact import  blake3:030bd7f098097c01cd4cccb65f8cd95bff80e440d50a9035609ace42663369fd
strict-ci gate   blake3:bfe1fc01a0e2d5acc2b90e5072167b5851231f30a8dcefd3555c8ce6a09130a8
remediation plan blake3:dd229a20c15b9bca925187813fba550fb684072a2367e4ff2a5da751ec19f723
corpus bytes     3a9e86c7a229c0866a81dfb16a61a5dd2a6129578c9f16c7ad7b7710b23621a6
```

The host kache index reported a disk I/O error and explicitly disabled caching; compilation, tests and source checks still completed. No unrelated cache or storage configuration was changed.

## Remaining adoption gates

Operated product deadlines and the new closed effect world/admitted host adapter still need product-owned evidence. Non-test ordinary actor and system-extension composition need complete matching Mantle member trees and actual production-bound admission inputs. Consumer enum labels, loose-byte fixtures and successful producer calls are not installations. Dependency, adapter, profile/WIT, consumer, materialization and receipt identities must cut over and roll back together only after those gates pass. The full local mechanism must not be removed before then.

Historical counterexamples and previous source receipts remain in the [earlier frozen record](2026-10-26-runtime-adoption-parity.md); they describe its old immutable producer, not current v4 failures. This continuation proves neither arbitrary guest correctness, host isolation, source trust, authority, release readiness nor full adoption.

## Contract-only effect-world continuation

The separate candidate `molten.wasm.component.effects.v1` now defines `molten:component-effects@1.0.0` / `actor-effects`, one `effect-request` import, one `invoke` export, and only `blob.get` / `get` under `hostcall:blob-get`. Exact Nickel equality requires `execution_admitted=false`. Request and response codecs preserve existing nested effect DTOs while binding the complete request-carrier identity independently, including generation, incarnation and sequence. Both directions are bounded to 65,536 bytes, depth 32 with root zero, and 4,096 nodes; annotation/embedded/trailing/noncanonical inputs deny. These checks grant no authority or replay admission.

Immediate outcome: a defined, checked wire/linker/operation cohort. Durable capability: reusable bounded canonical codecs, distinct outer/nested identity checks, exact WIT/export/readback and negative-contract checks. Molten's component-runtime maintainers own the contract, current-admission obligations and repeatable product acceptance; the [governing runtime contract](../../../../docs/wasm-component-runtime.md#contract-only-effect-request-world) gives wire shapes and regeneration commands. The [design decision](../design.md#add-one-explicit-effect-request-hostcall) keeps definition separate from activation.

The first focused run passed all 68 component cases, including eight new contract cases and the retained local/shared parity corpus. A disposable public-API executable compiled bindings from the actual new WIT and invoked a real Component Model core-memory/lower/lift bridge through Wasmtime 45. The host decoded the canonical request and returned an explicitly contract-only deny carrier without calling an effect port. The guest-to-host-to-guest round trip independently checked the outer carrier ref, nested effect-request ref, canonical deny and absent output:

```text
request carrier  blake3:862e924c8aac4f597ab6bceb1fac7a6422426a1b0b1097afe7c9f4eb995ca442
nested response  blake3:59890a11571e5e4afe1a914a5486514cc9601a5b405fc5087ad9c0be5dcccf08
decision=deny; no effect dispatch
```

The initial probe attempted direct re-export of an imported component function and failed component compilation. The corrected lowered/lifted bridge passed; its owned temporary source was removed. This is carrier/binding execution, not an admitted product host adapter or actor/extension installation.

The existing synchronous profile, time-sliced profile and production WIT retain the three hashes recorded above. The new effects WIT is `9478f7ddd9380bcbe898f2101d615674067e0f167ddc6f0fc345553751b5eaa2`; the generated effect contract is `5da576192dc648b9559f463e582d13a3e24154e81fa206380cc5d971557d4e19`. Dependency revisions and product execution defaults are unchanged. The inherited Tracey guard passed with 2966 requirements, 1032 referenced, 1934 unchanged baseline entries, zero dangling refs and six pending identity-only resolutions; the debt baseline was not edited.

The initial Nix component-profile check hit the configured build farm's overlong lock filename. The local retry (`--option builders ''`) passed both WIT hashes, all existing profile checks, candidate export/readback and the five new WASI/extra-operation/activation/depth/node negative fixtures. Independent review then required tying the exported contract's WIT source ref directly to the checked WIT hash; the rail now enforces that comparison rather than trusting independent literals.

Independent carrier review found an upstream atom-length arithmetic hazard in the initial preflight; its concrete regression and corrected source-gate results are recorded below. The initial configured workspace Octet producer reported `warning-only` for recursive traversal, crowded test placement and path-word repetition. Those artifacts remain under `2026-10-27-effects-source-gate/workspace/`; warning-only is not accepted evidence. No allowances or policy/baseline relaxation are authorized.

The production host adapter, current Basalt/UCAN/resource/replay admission, admitted/denied dispatched-effect fixtures, operated product deadlines, matching production Mantle bundles, real consumers and coupled cutover/rollback remain unproved. Keep the active change unsynced and unarchived, and keep the complete local runtime as the production default.

### Review corrections and regression proof

Both read-only review slices completed after the specialist review backend returned rate-limit errors. Policy review found that independent WIT-hash literals did not prove equality with the exported contract's source reference. The corrected rail compares `nickel export ... --field wit.source_ref --format text` to the exact measured WIT hash, then runs the existing checksum. Its local Nix realization passed as `/nix/store/ydrj6zrsizmvpshca60ap9qdpfvv64ym-molten-wasm-component-profile`.

Carrier review supplied an 11-byte atom with a valid `u64::MAX` length. The added regression failed before correction at Preserves 5.0.0-rc.7 `source.rs:196` with `attempt to add with overflow`, proving that the original `skip_atom` delegated unchecked length arithmetic to the dependency. The correction inspects packed framing iteratively with scalar depth/node counters and slice-checked atom bodies before recursive canonical decoding; it performs no allocation on the admitted preflight path and never calls that upstream skip path. Three varint bytes suffice for any atom inside the fixed byte ceiling; oversized/noncanonical longer lengths deny. Existing typed/canonical decoding still owns actual value validity.

After correction, `cargo test --locked -p molten --lib -- wasm_component` passed **69** cases, zero failed/ignored, including maximum declared lengths for float/integer/string/bytes/symbol atoms and all retained canonical, identity, resource and parity cases. Physical modules were renamed to intake/reply/declaration and contract tests moved under their owner; no duplicate implementation, forwarding shim, recursion allowance or source-policy relaxation was added.

A second disposable executable exercised the corrected public codecs outside the test runner. It accepted the canonical carrier above, independently measured nested request `blake3:92102bd8c4e884a22308e30853d0fe9ec37faba78326998e8921c71b29802cea`, and returned typed `InvalidPreservesPayload` denials for all five `u64::MAX` atom lengths without a panic or dispatch. Its source was removed after execution. The subsequent intermediate workspace producer retained an enum catch-all finding and a filename finding on that disposable source; its `warning-only` evidence remains under `workspace-final/`. The permanent framing match is now exhaustive and the disposable file is absent; only a later clean permanent-source producer is eligible for acceptance.

### Accepted permanent-source gate

The final configured workspace and `-p molten -- --lib` Octet producers both reported **clean**, exit zero, zero findings/warnings/errors, using the pinned `nightly-2026-03-21-x86_64-unknown-linux-gnu` producer. Their emitted command/status/summary/provenance artifacts are retained under [`2026-10-27-effects-source-gate/workspace-accepted/`](2026-10-27-effects-source-gate/workspace-accepted/) and [`library-accepted/`](2026-10-27-effects-source-gate/library-accepted/). Both disposable executables were absent from this accepted source cohort.

The full object-corpus receipt includes recursive `src` and the six explicit required facade paths, as in the recorded strict sequence. The actual rebuilt Molten CLI imported six artifacts with decision `pass`, passed unchanged `strict-ci`, and emitted the canonical combined remediation plan:

```text
artifact import  blake3:bdd66406b84cf7e43fbba74dc3441a0098bf4ad6721ab419127f6df81646955a
strict-ci gate   blake3:b8ea18fc40708cb97ef0ea0a7a40aee85c2b50611badc8f659cd9903a01c1cd8
remediation plan blake3:eaa01bb9d3705bd35315429f3944fe394dfd5cd82ffefa3018c1d41f8e535a28
corpus bytes     86bc8e9577738bd99a0286a84c4c40c7ce9679db59c5cbcd935954e297093c5f
```

Final Rust formatting and Nix formatting checks passed. The five touched Markdown files resolved all 32 local link targets. These source, link and contract checks do not prove authority, product installation, arbitrary component correctness, host isolation or release readiness. No sync or archive is authorized.

The strict sequence then ran `cargo test --locked --workspace --lib --quiet`: Molten **1769** and Molten Core **629** cases passed; the two other library targets had zero cases. Zero failed or ignored cases. This includes the retained actor, extension, effect, replay and receipt library behavior, not the integration-target, optional-feature, full flake or VM suite.

The first strict Clippy run found two needless generic-argument borrows and one redundant closure in contract test helpers. Those three test-only expressions were simplified without changing validation, expectations, limits or production logic; strict workspace/all-target Clippy then passed. Because source-token identities changed, the ordered producers/corpus/import/gate/remediation/library-tests/Clippy/Cairn sequence was repeated under a distinct `workspace-verified/` and `library-verified/` cohort rather than attributing the earlier receipt identities to different source bytes.

### Final verified checkpoint cohort

The repeated ordered sequence completed with exit zero: formatting; both clean pinned producers; complete corpus; actual CLI import, unchanged strict gate and remediation; **1769 + 629** passing workspace library cases; strict workspace/all-target Clippy; and strict native Cairn validation. The final [workspace](2026-10-27-effects-source-gate/workspace-verified/) and [library](2026-10-27-effects-source-gate/library-verified/) artifacts bind the corrected test-helper bytes. The emitted [complete execution log](2026-10-27-effects-source-gate/final-verification.log) retains the actual commands and output, not recreated producer assertions.

```text
artifact import  blake3:ce3134951a046ed709a1d81f0c3734829de9e0e4f4b324eb8c05006c50203d5d
strict-ci gate   blake3:cae81b2a3b18fd7976447d8298d6bc1838b526095758c6b9344c4593e91fff6a
remediation plan blake3:33feb53f9e02901a491529f8ed1dbdf8fe6d458ceacba60ddd42cf3f13c00be0
corpus bytes     559ba8605b68d903561a82922f003c75b231d0b0523c191a8da394bde6c2352a
```

Strict Cairn returned `valid=true` with empty findings/issues under explicit provider policy `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`; structural receipt hash was `eaebd7c5f2159f209659c641a1f2462774a257745a935fd9a3143f594e2a2e06`. The final inherited Tracey guard again passed unchanged debt and zero dangling references. The host kache index still reported disk I/O errors and disabled caching; no cache or storage configuration was changed. These bounded observations close only the contract-definition task. No production profile activation, effect dispatch, full adoption, sync, archive, VM execution or release eligibility is claimed.
