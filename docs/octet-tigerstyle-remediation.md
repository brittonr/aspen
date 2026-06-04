# Octet/TigerStyle remediation baseline

This file records the current Octet source-gate evidence and the remaining caveat for `octet-tigerstyle-remediation`.

Canonical plan receipt: `target/octet/remediation-plan.preserves`

Plan ref: `blake3:88109c97b000db6325392cb834cfeea475a90fd7e6e64c608b7b865082cc9157`

Latest Octet receipt refs: artifact import `blake3:1d9a63acab05c45b7de3ee960f7b05c1ec78ced919b3f33ee16f63e381cf466b`; strict gate pass `blake3:1e345244fbba421d7aee3ff4aeb8391aea6c16fa34a8f0e8bd952c00b0d8e0c9`; remediation plan `blake3:88109c97b000db6325392cb834cfeea475a90fd7e6e64c608b7b865082cc9157`.

## Artifact refs

| Scope | Artifact | Content/canonical ref |
|---|---|---|
| workspace | `target/octet/status.json` | `blake3:4a0e50c1d618632fa99e0799890c69cffb292a10cc5d4c4d6a336717d166db8f` |
| workspace | `target/octet/summary.txt` | `blake3:390cf1201df74cc8c138476002fbe3a0d2cd6332f3f04cfb09cef48931f54eac` |
| workspace/focused | `target/octet/object-corpus-receipt.json` | `blake3:c820c5a496005f8c568d289016082fb8be20316932777fe79a9af2ec466130df` |
| lib-only | `target/octet-lib/status.json` | `blake3:09166845bf27b17ed19b30a49b8a54ad9c8738d24c8110eb950f3dae35164c36` |
| lib-only | `target/octet-lib/summary.txt` | `blake3:cf9e24bf22e8af6374620ac714f1dfca701f3c95d7d5107e7656b83c17a1560f` |
| workspace/import | `target/octet/artifact-ledger-receipt.preserves` | `blake3:1d9a63acab05c45b7de3ee960f7b05c1ec78ced919b3f33ee16f63e381cf466b` |
| workspace/gate | `target/octet/gate-receipt.preserves` | `blake3:1e345244fbba421d7aee3ff4aeb8391aea6c16fa34a8f0e8bd952c00b0d8e0c9` |
| workspace/plan | `target/octet/remediation-plan.preserves` | `blake3:88109c97b000db6325392cb834cfeea475a90fd7e6e64c608b7b865082cc9157` |

Focused object corpus: object-set hash `b3:280a9b948271535f2d11930b090011d50013d276b63918d0592547ce286455a9`, 312 objects, 312 pure-cache blocked, source paths `src/job_dag.rs`, `src/main.rs`, `src/node_runtime.rs`.

## Counts

| Scope | Status | Findings | Warnings | Errors | Autofixable |
|---|---:|---:|---:|---:|---:|
| workspace | clean | 0 | 0 | 0 | 0 |
| lib-only | clean | 0 | 0 | 0 | 0 |

Top workspace lint counts: none.

Top lib-only lint counts: none.

Critical caveat pass: the latest lib-only and workspace runs have no active Octet findings, and the strict source gate passes. The zero finding state is configuration-clean: `dylint.toml` explicitly disables `non_trait_imports`, `path_segment_repetition`, `function_length`, `excessive_file_length`, `underscore_in_module_filename`, and `module_file_count`. If project policy requires source-remediated zero rather than config-clean zero, those disabled families remain the follow-up burn-down.

Additional validation after the Nix fetch fix: `nix build .#checks.x86_64-linux.nextest --no-link --print-out-paths --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` completed successfully at `/nix/store/8fcxgyj17dkigp5idpvnzb5dv78nd4nz-molten-nextest`. The flake now maps private OnixResearch git dependencies to locked local `*-src` path inputs for unit2nix git-cache population, so the Nix builder no longer needs SSH access.

## Critical surfaces

| Surface | Files | Workspace findings | Critical findings |
|---|---|---:|---:|
| source-gate-and-admission | `src/octet_gate.rs`, `src/node_runtime.rs`, `src/job_dag.rs`, `src/upgrades.rs` | 0 | 0 |
| harness-and-gates | `src/harness/gate.rs`, `src/harness/schema.rs`, `src/harness/runner.rs` | 0 | 0 |
| node-runtime-startup | `src/node_runtime.rs`, `src/node_identity.rs`, `src/resources.rs` | 0 | 0 |
| job-execution | `src/job_dag.rs`, `src/artifacts.rs`, `src/typed_storage.rs`, `src/eval_cache.rs` | 0 | 0 |
| ledger-and-evidence | `src/ledger.rs`, `src/evidence.rs`, `src/evidence_chain.rs` | 0 | 0 |
| adapter-boundaries | `src/harness/wasm_executor.rs`, `src/harness/steel_executor.rs`, `src/effects.rs`, `src/remote_dataspace.rs` | 0 | 0 |
| redaction-and-export | `src/catalog.rs`, `src/catalog_mcp.rs`, `src/transcripts.rs`, `src/harness/repro.rs` | 0 | 0 |
| cli-artifact-output | `src/main.rs` | 0 | 0 |

## Burn-down order if source-remediated zero is required

1. Split or reshape long files/functions instead of relying on `function_length` and `excessive_file_length` disables.
2. Normalize imports and repeated path segments instead of relying on `non_trait_imports` and `path_segment_repetition` disables.
3. Decide whether `underscore_in_module_filename` should be fixed by renaming modules or retained as an explicit style exception.
4. Resolve `module_file_count`, including external registry/rustlib paths, through Octet/config/tooling support rather than repo-only edits if needed.

## No-suppression policy

- Hidden suppressions are denied.
- Every retained active warning must have scheduled remediation or an explicit reviewed quarantine receipt.
- Strict gate keeps treating `warning-only` as deny.
- Quarantine is only for explicit, expiring, reviewed critical findings during burn-down.
- Current disabled lint families are explicit in `dylint.toml`; treat them as a documented configuration caveat, not hidden source suppressions.
