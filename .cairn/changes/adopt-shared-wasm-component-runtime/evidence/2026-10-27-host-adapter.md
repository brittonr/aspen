# Inactive effect host adapter — 2026-10-27

## Outcome and boundary

The product-owned `wasm_component::host` adapter now admits the closed `blob.get` operation through existing Molten effect declarations, Basalt/UCAN and the native callback value port. Its real capability-backed value adapter reads bounded canonical bytes through an explicit Ledger namespace and rejects substituted symlinks. Exact offline replay has no live-effect port or authority-grant input.

Immediate outcome: signed admitted retrieval, denied retrieval without protected access, and exact recorded playback. Durable contribution: reusable current-admission, complete carrier/context binding and post-reservation uncertainty boundaries. Molten component-runtime maintainers own the product mapping and acceptance; native-host maintainers own the capability value mechanism. Repeatability uses the repository Nix shell, focused component tests and the existing component-profile policy/WIT rail.

This is not engine/linker activation. `execution_admitted=false` remains required by the unchanged candidate contract. Existing import-free profiles, WIT, dependency pins, local runtime and actor/extension composition remain unchanged. No production shared-runtime consumer, positive production Mantle bundle, durable nonce-store selection, durable multi-file publication, full adoption, sync, archive, isolation or release eligibility is claimed.

## Product admission and effect order

Host-owned current inputs provide installed manifest/profile and grant facts, artifact and capability references, generation/incarnation, finite call/output bounds, actual Nickel-exported Basalt policy bytes, trusted issuer/holder identities and caller-owned UCAN verification/revocation/caveat/replay services. Guest references must match these current facts. The closed resource is `molten://blob/OBJECT-CONTENT-REF`, ability `blob/get`, contract `component-blob-get`.

The pinned `onix-compact-ed25519-v1` implementation verifies token/proofs, time, caveats, revocation, scope, holder signature and consume-once nonce. The holder signature binds the complete canonical carrier and run/handler/turn/boundary scope. This is not a claim about final UCAN 1.0.0. Verification time and skew tolerance remain caller-owned policy.

After admission and sequence reservation, request and intent publication precede the actual blob read. Response and complete completion-record publication precede returning success. A canonical authority denial carries neither output nor an output reference and does not retrieve the blob. Post-reservation port/publication failure poisons the session rather than inventing a denial or allowing retry. Publication is explicitly only process-visible. Replay validates separately identified canonical completion bytes, exact request/response, scope, lifecycle, sequence, output and intent linkage; its ordinary effect-log projection also passes the existing validator.

## Exercised behavior

`cargo test --locked -p molten --lib -- wasm_component` passed **78** cases, zero failed or ignored. This includes nine new host/value-port regressions and all 69 retained component cases. New behavior covers:

- Signed retrieval of independently stored canonical bytes, actual completion readback, exact offline playback and the existing deterministic effect-log validator.
- Invalid holder signature, before-not-before and after-expiry time, and duplicate consume-once nonce: canonical authority denial before protected reads.
- A current Basalt policy denying an otherwise valid signed UCAN.
- Stale generation, resource facts and capability references before blob dispatch.
- Replay scope, lifecycle, request, response and completion divergence; no resumption after the first divergence.
- Sequence and finite call-budget enforcement even after a policy denial.
- Unknown intent/response/completion publication outcomes: poisoned session, no automatic retry or invented policy result.
- Value byte ceilings, independent content identity and substituted leaf-symlink refusal.

The first focused run passed eight new cases and failed the expiry expectation at time 201: the fixture had implicitly inherited the UCAN verifier's clock-skew grace. The fixture now explicitly selects zero skew and checks both sides of its declared 100–200 interval. Production caller policy was not silently tightened. The corrected full component selection passed all 78 cases.

## Public-API smoke outside the test runner

A disposable `cargo run --locked --example hostadapter_smoke` executable used genuine signed UCAN/holder inputs and the public non-test host adapter, backed by a retained temporary directory capability and real canonical blob bytes. It admitted sequence zero; sequence one reused the consumed nonce and denied without a second protected read. Both responses then replayed exactly through the offline API, which accepts no live port:

```text
sequence=0 decision=pass
request=blake3:7d816fc57bb4218d60a73c2c5bd0d39bc66dc1ab590ada5bd76d2f4c13e02c22
completion=blake3:70492234da2f9320a04c6433ecc3e576517983edc01b6519e1fdc8a84f6d7d9d
sequence=1 decision=deny
request=blake3:a93520cc706954a34836a4e12ca5ca9e8955aa721d749fcd5e75fea4daafc78b
completion=blake3:b091ab9a3c6a1e72df3d4c5e6b227628be79cc794b80b8fce9884cab5e138137
protected_blob_reads=1; replay=exact; execution_admitted=false
```

The source reused signed fixture construction but invoked the public library outside `cfg(test)`; it did not install a product component linker or actor/extension consumer. Its owned source was removed after execution, before source-gate production. Host kache continued to report disk I/O errors and disabled caching; no cache/storage policy was modified.

## Contract, traceability and initial source findings

The local Nix `wasm-component-profile` rail passed as `/nix/store/d59bkx2354i1pvmxsilk9s3s6npn0fn3-molten-wasm-component-profile`. It retains existing profile/WIT checks, all negative contract fixtures, and now compares the host-policy Nickel export with its generated JSON. The five touched Markdown files resolved all **31** local link/anchor targets.

The inherited Tracey guard passed: **2966** requirements, **1032** referenced, **1934** unchanged baseline entries and **zero** dangling references. Eight pending references confer identity resolution only, not accepted coverage or lifecycle completion. An initial invocation incorrectly supplied the Nickel metadata instead of the exported `.txt` identifier list; correcting the command required no baseline edits.

The initial workspace Octet producer returned `warning-only`, with 12 findings from four policies: crowded native-host module directory, unchecked `u64`-to-`usize` casts, a repeated ancestor word in the credential identity helper, and non-trait helper imports. Its emitted artifacts remain under [`2026-10-27-host-source-gate/workspace/`](2026-10-27-host-source-gate/workspace/); this is not accepted evidence. The capability adapter moved into its own module directory without changing its Rust module/API, length comparisons use checked explicit-width conversion, the identity helper and every caller use `credential_ref`, and helper calls use their actual owner paths. No lint allowance, debt-baseline expansion or policy relaxation was added. The language server timed out on references/relocation; the bounded caller migration used structural AST rewrites instead.

The first clean producer/corpus/import/strict-gate/remediation sequence also passed **1778** Molten and **629** Molten Core library cases, with zero failed/ignored cases; the other two library targets had zero cases. Strict all-target Clippy then rejected cloning a complete log entry solely to build a one-element test slice. The test now borrows that entry and copies only the response reference required by the separately owned consumer DTO. Expectations, product behavior and validation bounds did not change. The unsuccessful sequence remains in [`verification.log`](2026-10-27-host-source-gate/verification.log), rather than attributing its receipts to the corrected source.

## Independently measured activation cohort

The existing five execution-contract inputs retain their previous BLAKE3 byte identities; only the new host-policy fixture is added:

```text
5bea351924145de8946669680125e99c837fc61ac6737e114ba3b9edf7ea0b89  generated/profile.json
48b088f86827c4fa31ffe6c2151dd32a8db53b3f38afef2b2e1b394d43d20852  generated/time-sliced-profile.json
83be349bb27975ada30dbe60817c5404df7862babdf02ae229b399886e76d5e8  molten-component-runtime/runtime.wit
9478f7ddd9380bcbe898f2101d615674067e0f167ddc6f0fc345553751b5eaa2  molten-component-effects/runtime.wit
5da576192dc648b9559f463e582d13a3e24154e81fa206380cc5d971557d4e19  generated/effect-contract.json
aebf8df037247074e5f987a144e4a420574f422aa244fd614cbdc9ed06640ec7  generated/host-policy.json
```

Policy storage and guest execution are different claim boundaries. Passing this fixture and its current-admission path cannot activate the candidate effect world. Existing production materialization, operated deadlines, non-test actor and extension installation, complete end-to-end parity and coupled cutover/rollback remain open in the active change.

## Final verified permanent-source cohort

The corrected ordered sequence completed with exit zero from the dedicated implementation worktree, using the pinned repository Nix shell, published dependency identities, two build jobs, task-owned executable `/dev/shm` Cargo/temp directories, and disabled debug information/incremental compilation. No sibling dependency source substitution was used.

| Executed check | Observed result |
|---|---|
| Pinned workspace Octet and `-p molten -- --lib` producers | Both `clean`, exit zero, zero findings/warnings/errors; pinned producer `nightly-2026-03-21-x86_64-unknown-linux-gnu`. |
| Full object corpus, actual rebuilt CLI artifact import, unchanged `strict-ci` gate and remediation plan | Corpus includes recursive `src` and the six explicit required facade paths. Import `pass`, six artifacts; gate `pass`; canonical remediation emitted. |
| `cargo test --locked --workspace --lib --quiet` | **1778** Molten and **629** Molten Core passed, zero failures/ignored; two other library targets had zero cases. Not integration targets or optional-feature execution. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed without allowances or policy relaxation. |
| `cargo fmt --all --check`; `nixfmt --check flake.nix` | Passed. |
| Strict native Cairn with explicit sibling-provider policy | `valid=true`, empty findings/issues, structural receipt `6a2f079c9abe35f3f9f4c5d081f3b38f152a78c34e5f9b59c99b0d062a15586c`. Structure is not behavioral or completion authority. |

Final emitted [workspace artifacts](2026-10-27-host-source-gate/workspace-final/), [library artifacts](2026-10-27-host-source-gate/library-final/) and the [actual ordered execution log](2026-10-27-host-source-gate/verification-final.log) bind the corrected permanent-source cohort. The disposable public-API source is absent from this cohort. Canonical CLI identities:

```text
artifact import  blake3:4537a7cd71cddf435fdca119bd2679672003a4e15a52da90358c67efc5ba8f51
strict-ci gate   blake3:b5700389a2bf36825f1a473221de2c013068460c424784cfd7eabef5d12d8b9b
remediation plan blake3:aa236c0a099d2571a0f94c10d76040ac88ade151d5eea2a839dc19b39b992e68
```

The component-profile Nix check, independent byte measurements, Tracey guard and public-API smoke are the separately recorded observations above. Full flake checks, VM integration, optional backends, production effect-world activation and release promotion were not run or claimed. Only the host-adapter and admitted/denied fixture tasks close; keep the adoption change active, unsynced and unarchived.
