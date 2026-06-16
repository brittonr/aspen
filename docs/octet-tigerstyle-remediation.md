# Octet/TigerStyle remediation baseline

This file records the current Octet source-gate evidence and the remaining caveat for `octet-tigerstyle-remediation`.

Canonical plan receipt: `target/octet/remediation-plan.preserves`

Plan ref: `blake3:252232dd954e07a924b55146e7a97efce1df1f147fc4835c2fe468392f9c2b7a`

Latest Octet receipt refs: artifact import `blake3:19a0239d4fb8598921159669568acbcbe486ade41cb4708ea21c7f05bbce0cf0`; strict gate pass `blake3:8e6c5f9f51e220893dbbae355e1091ea0fb8e59b2c6d9983c00d8812575e5404`; remediation plan `blake3:252232dd954e07a924b55146e7a97efce1df1f147fc4835c2fe468392f9c2b7a`.

## Artifact refs

| Scope | Artifact | Content/canonical ref |
|---|---|---|
| workspace | `target/octet/status.json` | `blake3:ec97a472d085fa845ce8164a47e108cdfa2df371bb8b7e87afa961bc69bb0772` |
| workspace | `target/octet/summary.txt` | `blake3:21a845f9bc28be78102a6700708b8aceaaa6ffde8817fc046f70073c58c2d60c` |
| workspace/focused | `target/octet/object-corpus-receipt.json` | `blake3:96a237d919494ba0fcc1242c9cd33115440bba93deb70ced810e7d1312734d15` |
| lib-only | `target/octet-lib/status.json` | `blake3:e9524c30eae46b20933a45578890f9dfaec8c6ebb79f75f9f7486b9d381d4f68` |
| lib-only | `target/octet-lib/summary.txt` | `blake3:5d978afb3680d3b0bf749eb6fefbb42c063005fd51b4ac62939a76fd56e4b453` |
| workspace/import | `target/octet/artifact-ledger-receipt.preserves` | `blake3:19a0239d4fb8598921159669568acbcbe486ade41cb4708ea21c7f05bbce0cf0` |
| workspace/gate | `target/octet/gate-receipt.preserves` | `blake3:8e6c5f9f51e220893dbbae355e1091ea0fb8e59b2c6d9983c00d8812575e5404` |
| workspace/plan | `target/octet/remediation-plan.preserves` | `blake3:252232dd954e07a924b55146e7a97efce1df1f147fc4835c2fe468392f9c2b7a` |

Focused object corpus: object-set hash `b3:4e05f6ae5d0f3acc36b5cf8a2c0b8f30202ace843165b3e35c7b0ff12b2d5c8a`, 2870 objects, 2870 pure-cache blocked, source paths include `src/cli/core/artifact.rs`, `src/cli/core/cache.rs`, `src/cli/core/catalog.rs`, `src/cli/core/chunk.rs`, `src/cli/workflow/coordination.rs`, `src/cli/workflow/delivery.rs`, `src/cli/ops/dogfood.rs`, `src/cli/evidence/gate.rs`, `src/cli/test/harness.rs`, `src/cli/workflow/job.rs`, `src/cli/ops/ledger.rs`, `src/cli/ops/node.rs`, `src/cli/ops/nixosvm.rs`, `src/cli/ops/octet.rs`, `src/cli/ops/plugin.rs`, `src/cli/evidence/receipts.rs`, `src/cli/ops/prodsoak.rs`, `src/cli/workflow/protocol.rs`, `src/cli/workflow/provenance.rs`, `src/cli/runtime/raft.rs`, `src/cli/workflow/remote.rs`, `src/cli/test/replayfixture.rs`, `src/cli/evidence/report.rs`, `src/cli/runtime/repro.rs`, `src/cli/workflow/retention.rs`, `src/cli/runtime/rewrite.rs`, `src/cli/core/schema.rs`, `src/cli/runtime/secrets.rs`, `src/cli/runtime/service.rs`, `src/cli/core/storage.rs`, `src/cli/core/transcript.rs`, `src/cli/runtime/upgrade.rs`, `src/cli/runtime/vat.rs`, `src/prod/soak.rs`, and `src/octet/remediation.rs` plus the critical runtime, node, job, retention, secrets, protocol, catalog, and CLI paths listed in `target/octet/object-corpus-receipt.json`.

## Counts

| Scope | Status | Findings | Warnings | Errors | Autofixable |
|---|---:|---:|---:|---:|---:|
| workspace | clean | 0 | 0 | 0 | 0 |
| lib-only | clean | 0 | 0 | 0 | 0 |

Top workspace lint counts: none.

Top lib-only lint counts: none.

Latest no-disabled-lints probe (`target/octet-probe-relocated-source`) is still `warning-only` with 8271 warnings: `non_trait_imports` 4635, `path_segment_repetition` 3040, `function_length` 436, `excessive_file_length` 137, `module_file_count` 23, and no `underscore_in_module_filename` findings.

Critical caveat pass: the latest lib-only and workspace runs have no active Octet findings, and the strict source gate passes. The zero finding state is configuration-clean: `dylint.toml` explicitly disables `non_trait_imports`, `path_segment_repetition`, `function_length`, `excessive_file_length`, and `module_file_count`. `underscore_in_module_filename` is no longer disabled after the source-layout relocation probe reported zero underscore-filename findings. Nix/dogfood was not rerun for this partial disabled-lint burn-down slice. If project policy requires source-remediated zero rather than config-clean zero, the remaining disabled families remain the follow-up burn-down.

Additional validation after the Nix fetch fix: `nix build .#checks.x86_64-linux.nextest --no-link --print-out-paths --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` completed successfully at `/nix/store/8fcxgyj17dkigp5idpvnzb5dv78nd4nz-molten-nextest`. The flake now maps private OnixResearch git dependencies to locked local `*-src` path inputs for unit2nix git-cache population, so the Nix builder no longer needs SSH access.

Release dogfood validation for the active `octet-source-remediated-zero` first split completed with `nix build .#checks.x86_64-linux.dogfood-local-node --no-link --print-out-paths -L --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` at `/nix/store/fwssw4qm1n291lh5f919w626pi239kid-molten-dogfood-local-node`. Evidence highlights: nextest 609/609 passed; Nix release verify `blake3:83c9a66232d736c2e63bea3ae342a763d806c9d17af2cbded16e3bfdccfaf8dc`; release bundle verify `blake3:268d3489901b6b43dbd1c6596fa5eb66a1cd5a6c3b2f231e00e41487521841ab`; promotion gate `blake3:4757c7aebf542911fc9227b5fede634484b1c1fc48e4148f297d2affe62994f4`; promotion summary `blake3:a51acc6108ab576f32a68dcd42ef60310f2d43b9372278c1dcf9c0bef859685d`; export verify `blake3:74ed8add45f1a99a35a4e5578a23e3c90fc712b409a19c2db45ec91fbce21bf0`.

## Critical surfaces

| Surface | Files | Workspace findings | Critical findings |
|---|---|---:|---:|
| source-gate-and-admission | `src/octet/gate.rs`, `src/node/runtime.rs`, `src/job/dag.rs`, `src/upgrades.rs` | 0 | 0 |
| harness-and-gates | `src/harness/gate.rs`, `src/harness/schema.rs`, `src/harness/runner.rs`, `src/nixos/vm.rs` | 0 | 0 |
| node-runtime-startup | `src/node/runtime.rs`, `src/node/identity.rs`, `src/resources.rs` | 0 | 0 |
| job-execution | `src/job/dag.rs`, `src/artifacts.rs`, `src/typed/storage.rs`, `src/eval/cache.rs` | 0 | 0 |
| ledger-and-evidence | `src/ledger.rs`, `src/evidence.rs`, `src/evidence/chain.rs` | 0 | 0 |
| adapter-boundaries | `src/harness/wasm/executor.rs`, `src/harness/steel/executor.rs`, `src/effects.rs`, `src/remote/dataspace.rs` | 0 | 0 |
| redaction-and-export | `src/catalog.rs`, `src/catalog/mcp.rs`, `src/transcripts.rs`, `src/harness/repro.rs` | 0 | 0 |
| cli-artifact-output | `src/main.rs`, `src/cli/core/artifact.rs`, `src/cli/core/cache.rs`, `src/cli/core/catalog.rs`, `src/cli/core/chunk.rs`, `src/cli/workflow/coordination.rs`, `src/cli/workflow/delivery.rs`, `src/cli/ops/dogfood.rs`, `src/cli/evidence/gate.rs`, `src/cli/test/harness.rs`, `src/cli/workflow/job.rs`, `src/cli/ops/ledger.rs`, `src/cli/ops/nixosvm.rs`, `src/cli/ops/node.rs`, `src/cli/ops/octet.rs`, `src/cli/ops/plugin.rs`, `src/cli/evidence/receipts.rs`, `src/cli/ops/prodsoak.rs`, `src/cli/workflow/protocol.rs`, `src/cli/workflow/provenance.rs`, `src/cli/runtime/raft.rs`, `src/cli/workflow/remote.rs`, `src/cli/test/replayfixture.rs`, `src/cli/evidence/report.rs`, `src/cli/runtime/repro.rs`, `src/cli/workflow/retention.rs`, `src/cli/runtime/rewrite.rs`, `src/cli/core/schema.rs`, `src/cli/runtime/secrets.rs`, `src/cli/runtime/service.rs`, `src/cli/core/storage.rs`, `src/cli/core/transcript.rs`, `src/cli/runtime/upgrade.rs`, `src/cli/runtime/vat.rs` | 0 | 0 |

## Burn-down order if source-remediated zero is required

Active burn-down slice: `cairn/changes/octet-source-remediated-zero` has moved Octet, Delivery, Protocol, Provenance, Retention, Job, Secrets, Plugin, Node, Repro, Catalog, Chunk, Cache, Artifact, Storage, Schema, Upgrade, Transcript, Rewrite, Remote, Ledger, Chain, Receipts, test Receipt, Service, Vat, Coordination, Dogfood, Raft, replay-fixture, Report, Gate, and harness run/replay CLI handling out of `src/main.rs`, then relocated the CLI shell corpus under bounded `src/cli/{core,workflow,ops,runtime,evidence,test}/` groups and moved the remaining underscore-named source files to directory/file layouts such as `src/node/runtime.rs`, `src/octet/gate.rs`, and `src/job/dag.rs` to narrow root module-count and underscore-filename pressure while preserving command semantics. The broader state remains configuration-clean until disabled lint families are removed or narrowed and evidence is refreshed.

1. Split or reshape long files/functions instead of relying on `function_length` and `excessive_file_length` disables.
2. Normalize imports and repeated path segments instead of relying on `non_trait_imports` and `path_segment_repetition` disables.
3. Resolve `module_file_count`, including external registry/rustlib paths, through Octet/config/tooling support rather than repo-only edits if needed.

## No-suppression policy

- Hidden suppressions are denied.
- Every retained active warning must have scheduled remediation or an explicit reviewed quarantine receipt.
- Strict gate keeps treating `warning-only` as deny.
- Quarantine is only for explicit, expiring, reviewed critical findings during burn-down.
- Current remaining disabled lint families are explicit in `dylint.toml`; treat them as a documented configuration caveat, not hidden source suppressions.
