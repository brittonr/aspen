# F04 executed verification — 2026-10-20

## Subject, outcome, and owner

Implementation is in the dedicated `aspen-f02-startup-evidence-20261020` worktree. The audit described source `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`; the pre-edit public counterexample was run at the worktree's unchanged `7e2fe0075` baseline. This record does not identify the edited tree as a committed or released candidate.

Immediate outcome: current absence or corruption cannot be satisfied by an old successful transfer. Durable capability: deterministic, bounded fresh attempts plus separately admitted exact-operation replay. The content-replication maintainers own the core, shell, adapters, and repeatable checks.

F04 owns fresh availability and replay. F05 owns cleanup selection; F07 owns deficit projection and receipt completion. Their combined checks do not merge these requirement packages.

## Baseline and changed behavior

The retained pre-edit baseline passed 10 root-library, 9 core, and 12 integration tests. The public audit smoke then failed its invariants: F04 printed `fresh_repair=false verified=2 unresolved=0`; F05 selected `peer-c`, losing the required unique domain; the F07 action-free deficit disappeared. These are planner/status observations, not executed production deletion or durability claims.

The post-fix public example was executed with:

```text
nix develop -c cargo run --example replication_audit_smoke
```

Observed output:

```text
F04 fresh_repair=true verified=1 unresolved=1
F05 cleanup_targets=["peer-b"] residual_domains_preserved=true
F07 decision=partial verified=1 unresolved=1
SHELL DeterministicSimulation fresh_attempt=2 transfers=2 no_target=partial deficit=1 receipt=blake3:906aeea4c91d4298b2cfe5fbf798e81ffa920d0b7447eaea6ad0f99e367465db
SHELL IrohLiveLoopback fresh_attempt=2 transfers=2 no_target=partial deficit=1 receipt=blake3:906aeea4c91d4298b2cfe5fbf798e81ffa920d0b7447eaea6ad0f99e367465db
```

The shell smoke used the public activation/reconciliation APIs, production fabric transport adapters, and `SimulatedDurableReplicationAdapter`, with controlled observation/content ports. It preserved the first durable operation, observed a distinct attempt 2, and then published an action-free partial receipt without a third transfer. The Iroh observation covers live loopback transport, not live distributed membership, real replica durability, or arbitrary deployments. The temporary example was removed after execution.

## Permanent regression coverage

- `crates/molten-core/src/content_replication/tests/repair.rs`: same-epoch loss, absent/corrupt targets, admitted current inventory, exhausted attempts, history ordering, conflicting identities, malformed/duplicate history, stale generations/epochs, and historical saved-plan readback.
- `src/content_replication/tests/deficits.rs`: fresh successful repair, unavailable repair, verification/envelope/planning rejection preserving prior durable state, exhaustion, and unchanged historical canonical operation/receipt bytes.
- `src/content_replication/tests/replay.rs`: exact saved terminal outcomes cause no duplicate effects; conflicting, malformed, duplicate, or unfinished saved operations reject; recorded cleanup success cannot bypass new retention admission.
- F05/F07 controls exercise safe cleanup and historical success with unresolved demand.

Initial post-fix focused runs, before policy-only file/name/import refactors:

```text
nix develop -c cargo test --manifest-path checks/content-replication-octet/Cargo.toml --all-targets --all-features
# 33 passed, 0 failed, 0 ignored
nix develop -c cargo test --lib content_replication -- --nocapture
# 28 passed, 0 failed, 0 ignored; includes controlled shells, world-distribution bridge,
# deterministic durability/faults, and live-Iroh/local-content parity
```

The focused workspace has only its own compatible feature set; this is not an unqualified all-features Molten baseline.

## Compatibility decision

Operation/action v1 framing and operation/status/receipt v1 shapes are preserved. Fresh reconciliation increments the maximum current-generation/epoch receiver attempt across historical outcomes and kinds. Active `Reuse` actions are removed. New canonical plans use the explicit v2 carrier/identity context and bind per-content missing counts. Public reconciliation always replans; historical plans and receipts do not authorize current availability or deletion. The [design decision](../design.md#implementation-compatibility-decision-2026-10-20) and [operator contract](../../../../docs/content-replication.md#canonical-compatibility) specify the cutover.

## Executed policy and lifecycle observations

- `nix build path:.#checks.x86_64-linux.world-distribution-octet-deny-all --no-link -L`: **passed**, `Status: clean`, zero findings/warnings/errors, pinned `nightly-2026-03-21`. Output: `/nix/store/sv89m9zz4ysilz8fm4s176sf0gixahhf-octet-consumer-check`. This compiles the real core and its tests alongside DAG sync/world distribution with warnings denied.
- `nix develop -c cargo fmt --check`: **passed** after removing the temporary smoke example.
- Final configured workspace and library Octet producers both passed **clean**, zero findings/warnings/errors, after the replay fixture imports were qualified and cleanup rejection cases were split into a child module. The workspace config/profile hashes are `b3:f6d90d147080ec9c5ff594bf443aed65b8f60517ec0eecdda92502b1f6964268` / `b3:bdaab55d417efab9becf009dbf8e655a084da641a854331fc72bf76fab13e482`; the library config/profile hashes are `b3:7e5ddc24bd10c119cbb858ca382e7f0f4cbd629a2fe1075ec8402936da19d71a` / `b3:2aad1f1f1a78bc9277fdb6abe27b39a261cacbd0e2d280f548667b61f852dbbd`. The pinned producer cohort remains `cargo-octet 0.1.0`, `nightly-2026-03-21-x86_64-unknown-linux-gnu`.
- The final object-corpus receipt was generated from all **1,916** sorted `src/**/*.rs` members. Bounded directory enumeration used 94 queries with no truncated leaf scopes. `cargo octet object corpus receipt --output target/octet/object-corpus-receipt.json <sorted members>` completed successfully.
- `nix develop -c cargo run -- test octet artifacts import --artifacts target/octet --ledger target/octet-ledger --receipt-out target/octet/artifact-ledger-receipt.preserves`: **pass**, six artifacts imported; receipt `blake3:0c65262b4fef29891f781bddb38e98fcadd73f56bb2e3e1c5ffd62a61f9ebc68`.
- `nix develop -c cargo run -- test octet gate --artifacts target/octet --profile strict-ci --receipt-out target/octet/gate-receipt.preserves`: **pass**, receipt `blake3:95bb870be3023b80efbaee24ddbf638a461ae411a833f516b8b2a4e2aaf283d9`.
- `nix develop -c cargo run -- test octet remediation plan --artifacts target/octet --lib-artifacts target/octet-lib --focused-object-corpus target/octet/object-corpus-receipt.json --receipt-out target/octet/remediation-plan.preserves`: **completed**, receipt `blake3:4f06c9b800e46a1351cfaad6505433f00ee4d44eb074ea7ceab9791ab2990da2`.
- Final-tree `nix develop -c cargo test --workspace`: **passed**, 2,219 passed, zero failed or ignored, including normal unit/integration tests and seven doctests. The moved cleanup rejection tests and qualified replay fixtures executed successfully in the normal library suite.
- Final-tree `nix develop -c cargo test --workspace --all-targets --features doltlite-oracle,executable-extents,world-snapshot-vm-cohort`: **passed**, 2,231 passed, zero failed, one ignored. The ignored live DoltLite oracle requires its Nix-built remotes-disabled external cohort. These are the compatible non-profiling runtime features, not an unqualified `--all-features` baseline; this run does not establish VM execution.
- Final-tree `nix develop -c cargo clippy --workspace --all-targets --features doltlite-oracle,executable-extents,world-snapshot-vm-cohort -- -D warnings`: **passed** without warning allowances.
- Inherited traceability guard: **passed** with 2,845 accepted requirements, 912 referenced, 1,933 uncovered matching all 1,933 baseline entries, zero dangling references, and 17 pending references. Pending means identity resolution only, not accepted coverage or behavioral completion; the baseline was not expanded.
- After the implementation/validation task boxes were updated, all nine separate proposal/design/tasks gates passed again with zero issues under the explicit owner policy. These are structural planning gates, not acceptance or review authority. Task-gate receipts: F04 `c1ed5cd170efa1143a9957e1e7bddc6f891eada02a52964827c2ad333edd94d9`; F05 `a6113ff92065e3cb7aa2c6e28ff7c57abb68405f58ed8b7743ecc48f45093a8c`; F07 `6f95207d08f61c64f5b7940fa0d761b27b40e49ee9a93da287dfbe1cf4bd032f`.
- Installed `cairn validate --root . --policy /home/brittonr/git/OnixResearch/cairn/cairn-policy/generated/cairn-policy.json --strict`: **passed** before and after the final source/test/task edits, 42 active changes and no issues/findings. The initial receipt was `20c6877f59e3547356476beed15094665de2ead3b64d0ef3d62dccaf9e606228`; the post-task-readback receipt, measured before appending this result, is `574933ba27c710867ab4b0e456928edba5c43be92c766528a33dc336f1517f47`. Explicit policy BLAKE3 remains `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`.
- The inherited traceability guard also passed after the final source/test/task edits with the same counts above. Its task-owned temporary executable was removed. At this pre-publication verification stage, no sync, archive, commit, push, or release operation had been performed.

## Retained failed-attempt evidence

- The first hermetic policy attempt found module-file-count, concrete-import, test filename, and repeated-path naming errors. Source layout/imports/names were corrected without new lint allowances; the retry passed above.
- A configured workspace producer overlapped the structural file move and observed the temporary missing `status` module. Module mounts were corrected before the final source-gate run. A first library producer returned `warning-only` for the new replay file's concrete `BTreeMap` import; it was qualified instead of allowing the warning.
- The next configured workspace producer reported ten concrete/renamed-import findings in the new replay fixture. Those imports were removed or qualified using the existing glob-import convention; the final configured workspace/library runs above were clean. No warning baseline, lint allowance, or policy relaxation was introduced.
- Direct focused Octet invocation from `checks/content-replication-octet` failed with `invalid Cargo diagnostic path: component escapes the workspace root` while processing its existing out-of-workspace `#[path]` source mount. It produced no passing focused lint receipt. This failure is distinct from the successful hermetic deny-all scope above.
- The expanded temporary shell smoke initially lacked its borrowed fixtures' `crate::error` binding. The harness import was corrected; the actual public shell execution then produced the recorded output. No product fallback was added.
- Launching the sibling Cairn through `nix run path:/home/brittonr/git/OnixResearch/cairn#cairn` failed during path ingestion with `file '/socket' has an unsupported type`. The installed Cairn executed the same strict validation with the same explicit owner policy; no policy or strictness was reduced.

## Non-claims

No source-to-running-binary binding was invented for F02. No production deletion, permanent durability, arbitrary network correctness, VM execution, lifecycle completion, archive, commit, push, or release approval follows from these observations.

## Authorized lifecycle closeout — 2026-10-20

After the implementation report, the operator explicitly requested commit, push, and continued work. Native Cairn completeness review, sync preview/execution, and archive preview/execution were then run separately for F04, F05, and F07 under the same explicit owner policy. All previews were unblocked and all executions reported mutation. Completeness is structural coverage evidence, not an independent human semantic review or release approval.

| Change | Completeness receipt | Sync receipt | Archive receipt |
|---|---|---|---|
| F04 | `392f85aa14e1087db1347e6e35489f358f707a650b44acd19603354fec36943a` | `ee7d7a53c54f0334c3415ec6358f80125e1da02183ce3e9b1c0e72ae4f2d165a` | `0cd573d508c547e2644173f5b499f5718a10b5fe4d47ddb6f6c097896a9d0199` |
| F05 | `44afccc822fc58379ccbf4a43e82e53cfef10be254ed18f91e554e8a509dc637` | `3251df1a3858d8ff586502e3bce4f01151782bfbcba24a16f6bee5cf8c8f77c0` | `077d820e49c7082922f1353b52c6969ba9e5537abd5b7a20b0b463f54d485bc6` |
| F07 | `3bdc101d0df299a286398c1d529acc598712871a762a56fdd222e2026ed6812d` | `62d5b0aba65b4e8dc426b60ae7fc1414d4a0c4dad931b6bed40eaaab05efc0cf` | `5b42fa19c035a2cf7efdefb294c73559213d8dfe4f3c5e059df36708d9213807` |

The native tool created the repository-conventional `1970-01-01-` archive names; the executed observation date remains 2026-10-20. Audit navigation and the new archives' cross-links were migrated without rewriting historical captured stdout.

Post-sync/archive inherited-debt validation passed: 2,857 accepted requirements, 924 referenced, 1,933 uncovered matching the unchanged 1,933-entry baseline, five pending references, and zero dangling references. Strict Cairn validation passed with 39 remaining active changes and no findings/issues; receipt `8dd556c5c37c2877f3b2f57e5c699a625c70a1073400e12182a1435a280b1588`.

These native lifecycle observations do not establish production durability, arbitrary network correctness, VM behavior, or release readiness. Git commit and remote publication results are reported separately.

