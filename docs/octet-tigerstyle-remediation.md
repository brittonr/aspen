# Octet/TigerStyle remediation baseline

This file records the current Octet source-gate evidence and the remaining caveat for `octet-tigerstyle-remediation`.

Canonical plan receipt: `target/octet/remediation-plan.preserves`

Plan ref: `blake3:c3dc783d41b42af53315d3377d89b290aebac5df3960a2d06651d7e006971c2d`

Latest Octet receipt refs: artifact import `blake3:0835a0d9c68349328e3c362ae009fedffcaeef04dae7f427f97a445cf16585c9`; strict gate pass `blake3:0227372220d11b7e9dcb512c428255a2e746b8ed7e650ee5eaaadc6458776f98`; remediation plan `blake3:c3dc783d41b42af53315d3377d89b290aebac5df3960a2d06651d7e006971c2d`.

## Artifact refs

| Scope | Artifact | Content/canonical ref |
|---|---|---|
| workspace | `target/octet/status.json` | `blake3:b20c6b5f05268d388af8f4b474e79a217e9d1b31dd89066431fa15687f405434` |
| workspace | `target/octet/summary.txt` | `blake3:1674a4493e906173010ad80e1d270db8b17af9437c4c86f5c79468f1e474482c` |
| workspace/focused | `target/octet/object-corpus-receipt.json` | `blake3:6ec4e5bd27ba7495756ae2e5f0d33e190d2c35e2f022368cfe24fd2f006f629e` |
| lib-only | `target/octet-lib/status.json` | `blake3:08320c635b64492fa88009a6ee42e3025db91b04032191b5b32fb0f3d201ef1b` |
| lib-only | `target/octet-lib/summary.txt` | `blake3:1674a4493e906173010ad80e1d270db8b17af9437c4c86f5c79468f1e474482c` |
| workspace/import | `target/octet/artifact-ledger-receipt.preserves` | `blake3:0835a0d9c68349328e3c362ae009fedffcaeef04dae7f427f97a445cf16585c9` |
| workspace/gate | `target/octet/gate-receipt.preserves` | `blake3:0227372220d11b7e9dcb512c428255a2e746b8ed7e650ee5eaaadc6458776f98` |
| workspace/plan | `target/octet/remediation-plan.preserves` | `blake3:c3dc783d41b42af53315d3377d89b290aebac5df3960a2d06651d7e006971c2d` |

Focused object corpus: object-set hash `b3:404e3e371dd93ea478497fce557095f4ab06343005bc9821d70dc1972cc6c5ea`, 2732 objects, 2732 pure-cache blocked, source paths include `src/cli_delivery.rs`, `src/cli_job.rs`, `src/cli_node.rs`, `src/cli_octet.rs`, `src/cli_plugin.rs`, `src/cli_protocol.rs`, `src/cli_provenance.rs`, `src/cli_repro.rs`, `src/cli_retention.rs`, `src/cli_secrets.rs`, and `src/octet_remediation.rs` plus the critical runtime, node, job, retention, secrets, protocol, catalog, and CLI paths listed in `target/octet/object-corpus-receipt.json`.

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
| cli-artifact-output | `src/main.rs`, `src/cli_delivery.rs`, `src/cli_job.rs`, `src/cli_node.rs`, `src/cli_octet.rs`, `src/cli_plugin.rs`, `src/cli_protocol.rs`, `src/cli_provenance.rs`, `src/cli_repro.rs`, `src/cli_retention.rs`, `src/cli_secrets.rs` | 0 | 0 |

## Burn-down order if source-remediated zero is required

Active burn-down slice: `cairn/changes/octet-source-remediated-zero` has moved Octet, Delivery, Protocol, Provenance, Retention, Job, Secrets, Plugin, Node, and Repro CLI command parsing out of `src/main.rs` into `src/cli_octet.rs`, `src/cli_delivery.rs`, `src/cli_protocol.rs`, `src/cli_provenance.rs`, `src/cli_retention.rs`, `src/cli_job.rs`, `src/cli_secrets.rs`, `src/cli_plugin.rs`, `src/cli_node.rs`, and `src/cli_repro.rs` while preserving command semantics. The broader state remains configuration-clean until disabled lint families are removed or narrowed and evidence is refreshed.

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
