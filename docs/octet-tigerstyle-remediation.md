# Octet/TigerStyle remediation baseline

This file records the current Octet source-gate evidence and the remaining caveat for `octet-tigerstyle-remediation`.

Canonical plan receipt: `target/octet/remediation-plan.preserves`

Plan ref: `blake3:ec3ef46fdeb07dcbe70a688e66296f718f83d3b83f333a6cda1b80cff772f1d4`

Latest Octet receipt refs: artifact import `blake3:7c10fc70f2558af30ddc854478deb3dc2a1894c43b3cc99d0f592d391e9d64b4`; strict gate pass `blake3:e1a239203b4885a6ac61c6b8eadcd657e95a4e02bdc0f556976deb1e1c7fc282`; remediation plan `blake3:ec3ef46fdeb07dcbe70a688e66296f718f83d3b83f333a6cda1b80cff772f1d4`.

## Artifact refs

| Scope | Artifact | Content/canonical ref |
|---|---|---|
| workspace | `target/octet/status.json` | `blake3:22df34469748c613baf9546600f608569344f9c1b1c3a403ae27e9f814e41d72` |
| workspace | `target/octet/summary.txt` | `blake3:950623ea396e4d89b32d1e2f41f621f5d58b78a441de16cbd31ccf7a01044fca` |
| workspace/focused | `target/octet/object-corpus-receipt.json` | `blake3:465e223715f93a53b734d661cdae2e32edb5b0c728c402605fe7df1e4fc490b3` |
| lib-only | `target/octet-lib/status.json` | `blake3:fa51f427bf8e5716e3f910e25e2f7877e46577dea09a960c07f33d786cb30086` |
| lib-only | `target/octet-lib/summary.txt` | `blake3:3240ff47b4e2515d42fccb52d810beed032ccacdc07f7bc652d37cd3c3c5cdae` |
| workspace/import | `target/octet/artifact-ledger-receipt.preserves` | `blake3:7c10fc70f2558af30ddc854478deb3dc2a1894c43b3cc99d0f592d391e9d64b4` |
| workspace/gate | `target/octet/gate-receipt.preserves` | `blake3:e1a239203b4885a6ac61c6b8eadcd657e95a4e02bdc0f556976deb1e1c7fc282` |
| workspace/plan | `target/octet/remediation-plan.preserves` | `blake3:ec3ef46fdeb07dcbe70a688e66296f718f83d3b83f333a6cda1b80cff772f1d4` |

Focused object corpus: object-set hash `b3:7d3abf25466d3418f74d07b44781ad9cbbcf47ff3011b927c92eaa130e4bbb73`, 2665 objects, 2665 pure-cache blocked, source paths include `src/cli_delivery.rs`, `src/cli_octet.rs`, `src/cli_provenance.rs`, and `src/cli_retention.rs` plus the critical runtime, node, job, retention, secrets, protocol, catalog, and CLI paths listed in `target/octet/object-corpus-receipt.json`.

## Counts

| Scope | Status | Findings | Warnings | Errors | Autofixable |
|---|---:|---:|---:|---:|---:|
| workspace | clean | 0 | 0 | 0 | 0 |
| lib-only | clean | 0 | 0 | 0 | 0 |

Top workspace lint counts: none.

Top lib-only lint counts: none.

Critical caveat pass: the latest lib-only and workspace runs have no active Octet findings, and the strict source gate passes. The zero finding state is configuration-clean: `dylint.toml` explicitly disables `non_trait_imports`, `path_segment_repetition`, `function_length`, `excessive_file_length`, `underscore_in_module_filename`, and `module_file_count`. If project policy requires source-remediated zero rather than config-clean zero, those disabled families remain the follow-up burn-down.

Additional validation after the Nix fetch fix: `nix build .#checks.x86_64-linux.nextest --no-link --print-out-paths --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` completed successfully at `/nix/store/8fcxgyj17dkigp5idpvnzb5dv78nd4nz-molten-nextest`. The flake now maps private OnixResearch git dependencies to locked local `*-src` path inputs for unit2nix git-cache population, so the Nix builder no longer needs SSH access.

Release dogfood validation for the active `octet-source-remediated-zero` first split completed with `nix build .#checks.x86_64-linux.dogfood-local-node --no-link --print-out-paths -L --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` at `/nix/store/fwssw4qm1n291lh5f919w626pi239kid-molten-dogfood-local-node`. Evidence highlights: nextest 609/609 passed; Nix release verify `blake3:83c9a66232d736c2e63bea3ae342a763d806c9d17af2cbded16e3bfdccfaf8dc`; release bundle verify `blake3:268d3489901b6b43dbd1c6596fa5eb66a1cd5a6c3b2f231e00e41487521841ab`; promotion gate `blake3:4757c7aebf542911fc9227b5fede634484b1c1fc48e4148f297d2affe62994f4`; promotion summary `blake3:a51acc6108ab576f32a68dcd42ef60310f2d43b9372278c1dcf9c0bef859685d`; export verify `blake3:74ed8add45f1a99a35a4e5578a23e3c90fc712b409a19c2db45ec91fbce21bf0`.

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
| cli-artifact-output | `src/main.rs`, `src/cli_delivery.rs`, `src/cli_octet.rs`, `src/cli_provenance.rs`, `src/cli_retention.rs` | 0 | 0 |

## Burn-down order if source-remediated zero is required

Active burn-down slice: `cairn/changes/octet-source-remediated-zero` has moved Octet, Delivery, Provenance, and Retention CLI command parsing out of `src/main.rs` into `src/cli_octet.rs`, `src/cli_delivery.rs`, `src/cli_provenance.rs`, and `src/cli_retention.rs` while preserving command semantics. The broader state remains configuration-clean until disabled lint families are removed or narrowed and evidence is refreshed.

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
