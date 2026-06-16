# Octet/TigerStyle remediation baseline

This file records the current Octet source-gate evidence and the remaining caveat for `octet-tigerstyle-remediation`.

Canonical plan receipt: `target/octet/remediation-plan.preserves`

Plan ref: `blake3:41dd47456010e5ae076d810acd5ff48ade13fddd06a8a6f2961724d1a4e81bfa`

Latest Octet receipt refs: artifact import `blake3:91957e379854cdc24afa45a5abf920b47dd35d5daaf3cecc056b70d0d5edf836`; strict gate pass `blake3:4ce35d68587f2235945a15a6b7c8b124db7c2f9ae08b5da60ebc27643ee45170`; remediation plan `blake3:41dd47456010e5ae076d810acd5ff48ade13fddd06a8a6f2961724d1a4e81bfa`.

## Artifact refs

| Scope | Artifact | Content/canonical ref |
|---|---|---|
| workspace | `target/octet/status.json` | `blake3:b7d8427de38c6be94349af9d01e5624e8b79682857070ba2a9c8eeb3180fb3ea` |
| workspace | `target/octet/summary.txt` | `blake3:7345c34c8139e4a835994ed958782191b719268cc80f3a04359be1b6b9cd973d` |
| workspace/focused | `target/octet/object-corpus-receipt.json` | `blake3:fcae66784b54d17faec80bb6197aa09e5a43deed91fdf9007a724268ffcfbba4` |
| lib-only | `target/octet-lib/status.json` | `blake3:014a24541d5c7c4b0fb28e2b7bcbffef6ffc2ad6573d38adb8fc607627783058` |
| lib-only | `target/octet-lib/summary.txt` | `blake3:f8346ca8bb0b0a97bc1e5525232be18f924852d5e8ea96394f9769489e71e4de` |
| workspace/import | `target/octet/artifact-ledger-receipt.preserves` | `blake3:91957e379854cdc24afa45a5abf920b47dd35d5daaf3cecc056b70d0d5edf836` |
| workspace/gate | `target/octet/gate-receipt.preserves` | `blake3:4ce35d68587f2235945a15a6b7c8b124db7c2f9ae08b5da60ebc27643ee45170` |
| workspace/plan | `target/octet/remediation-plan.preserves` | `blake3:41dd47456010e5ae076d810acd5ff48ade13fddd06a8a6f2961724d1a4e81bfa` |

Focused object corpus: object-set hash `b3:3e2e54505bd5f69db2dd62f2814361a775542d5cdab357036da8b185c1923c22`, 2870 objects, 2870 pure-cache blocked, source paths include `src/cli/core/artifact.rs`, `src/cli/core/cache.rs`, `src/cli/core/catalog.rs`, `src/cli/core/chunk.rs`, `src/cli/workflow/coordination.rs`, `src/cli/workflow/delivery.rs`, `src/cli/ops/dogfood.rs`, `src/cli/evidence/gate.rs`, `src/cli/test/harness.rs`, `src/cli/workflow/job.rs`, `src/cli/ops/ledger.rs`, `src/cli/ops/node.rs`, `src/cli/ops/nixosvm.rs`, `src/cli/ops/octet.rs`, `src/cli/ops/plugin.rs`, `src/cli/evidence/receipts.rs`, `src/cli/ops/prodsoak.rs`, `src/cli/workflow/protocol.rs`, `src/cli/workflow/provenance.rs`, `src/cli/runtime/raft.rs`, `src/cli/workflow/remote.rs`, `src/cli/test/replayfixture.rs`, `src/cli/evidence/report.rs`, `src/cli/runtime/repro.rs`, `src/cli/workflow/retention.rs`, `src/cli/runtime/rewrite.rs`, `src/cli/core/schema.rs`, `src/cli/runtime/secrets.rs`, `src/cli/runtime/service.rs`, `src/cli/core/storage.rs`, `src/cli/core/transcript.rs`, `src/cli/runtime/upgrade.rs`, `src/cli/runtime/vat.rs`, `src/prod_soak.rs`, and `src/octet_remediation.rs` plus the critical runtime, node, job, retention, secrets, protocol, catalog, and CLI paths listed in `target/octet/object-corpus-receipt.json`.

## Counts

| Scope | Status | Findings | Warnings | Errors | Autofixable |
|---|---:|---:|---:|---:|---:|
| workspace | clean | 0 | 0 | 0 | 0 |
| lib-only | clean | 0 | 0 | 0 | 0 |

Top workspace lint counts: none.

Top lib-only lint counts: none.

Critical caveat pass: the latest lib-only and workspace runs have no active Octet findings, and the strict source gate passes. The zero finding state is configuration-clean: `dylint.toml` explicitly disables `non_trait_imports`, `path_segment_repetition`, `function_length`, `excessive_file_length`, `underscore_in_module_filename`, and `module_file_count`. Nix/dogfood was not rerun for the CLI-directory disabled-lint burn-down slice. If project policy requires source-remediated zero rather than config-clean zero, those disabled families remain the follow-up burn-down.

Additional validation after the Nix fetch fix: `nix build .#checks.x86_64-linux.nextest --no-link --print-out-paths --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` completed successfully at `/nix/store/8fcxgyj17dkigp5idpvnzb5dv78nd4nz-molten-nextest`. The flake now maps private OnixResearch git dependencies to locked local `*-src` path inputs for unit2nix git-cache population, so the Nix builder no longer needs SSH access.

Release dogfood validation for the active `octet-source-remediated-zero` first split completed with `nix build .#checks.x86_64-linux.dogfood-local-node --no-link --print-out-paths -L --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` at `/nix/store/fwssw4qm1n291lh5f919w626pi239kid-molten-dogfood-local-node`. Evidence highlights: nextest 609/609 passed; Nix release verify `blake3:83c9a66232d736c2e63bea3ae342a763d806c9d17af2cbded16e3bfdccfaf8dc`; release bundle verify `blake3:268d3489901b6b43dbd1c6596fa5eb66a1cd5a6c3b2f231e00e41487521841ab`; promotion gate `blake3:4757c7aebf542911fc9227b5fede634484b1c1fc48e4148f297d2affe62994f4`; promotion summary `blake3:a51acc6108ab576f32a68dcd42ef60310f2d43b9372278c1dcf9c0bef859685d`; export verify `blake3:74ed8add45f1a99a35a4e5578a23e3c90fc712b409a19c2db45ec91fbce21bf0`.

## Critical surfaces

| Surface | Files | Workspace findings | Critical findings |
|---|---|---:|---:|
| source-gate-and-admission | `src/octet_gate.rs`, `src/node_runtime.rs`, `src/job_dag.rs`, `src/upgrades.rs` | 0 | 0 |
| harness-and-gates | `src/harness/gate.rs`, `src/harness/schema.rs`, `src/harness/runner.rs`, `src/nixos_vm.rs` | 0 | 0 |
| node-runtime-startup | `src/node_runtime.rs`, `src/node_identity.rs`, `src/resources.rs` | 0 | 0 |
| job-execution | `src/job_dag.rs`, `src/artifacts.rs`, `src/typed_storage.rs`, `src/eval_cache.rs` | 0 | 0 |
| ledger-and-evidence | `src/ledger.rs`, `src/evidence.rs`, `src/evidence_chain.rs` | 0 | 0 |
| adapter-boundaries | `src/harness/wasm_executor.rs`, `src/harness/steel_executor.rs`, `src/effects.rs`, `src/remote_dataspace.rs` | 0 | 0 |
| redaction-and-export | `src/catalog.rs`, `src/catalog_mcp.rs`, `src/transcripts.rs`, `src/harness/repro.rs` | 0 | 0 |
| cli-artifact-output | `src/main.rs`, `src/cli/core/artifact.rs`, `src/cli/core/cache.rs`, `src/cli/core/catalog.rs`, `src/cli/core/chunk.rs`, `src/cli/workflow/coordination.rs`, `src/cli/workflow/delivery.rs`, `src/cli/ops/dogfood.rs`, `src/cli/evidence/gate.rs`, `src/cli/test/harness.rs`, `src/cli/workflow/job.rs`, `src/cli/ops/ledger.rs`, `src/cli/ops/nixosvm.rs`, `src/cli/ops/node.rs`, `src/cli/ops/octet.rs`, `src/cli/ops/plugin.rs`, `src/cli/evidence/receipts.rs`, `src/cli/ops/prodsoak.rs`, `src/cli/workflow/protocol.rs`, `src/cli/workflow/provenance.rs`, `src/cli/runtime/raft.rs`, `src/cli/workflow/remote.rs`, `src/cli/test/replayfixture.rs`, `src/cli/evidence/report.rs`, `src/cli/runtime/repro.rs`, `src/cli/workflow/retention.rs`, `src/cli/runtime/rewrite.rs`, `src/cli/core/schema.rs`, `src/cli/runtime/secrets.rs`, `src/cli/runtime/service.rs`, `src/cli/core/storage.rs`, `src/cli/core/transcript.rs`, `src/cli/runtime/upgrade.rs`, `src/cli/runtime/vat.rs` | 0 | 0 |

## Burn-down order if source-remediated zero is required

Active burn-down slice: `cairn/changes/octet-source-remediated-zero` has moved Octet, Delivery, Protocol, Provenance, Retention, Job, Secrets, Plugin, Node, Repro, Catalog, Chunk, Cache, Artifact, Storage, Schema, Upgrade, Transcript, Rewrite, Remote, Ledger, Chain, Receipts, test Receipt, Service, Vat, Coordination, Dogfood, Raft, replay-fixture, Report, Gate, and harness run/replay CLI handling out of `src/main.rs`, then relocated the CLI shell corpus under bounded `src/cli/{core,workflow,ops,runtime,evidence,test}/` groups to narrow root module-count and underscore-filename pressure while preserving command semantics. The broader state remains configuration-clean until disabled lint families are removed or narrowed and evidence is refreshed.

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
