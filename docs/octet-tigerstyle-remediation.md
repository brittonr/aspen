# Octet/TigerStyle remediation baseline

This file records the current Octet source-gate evidence and the remaining caveat for `octet-tigerstyle-remediation`.

Canonical plan receipt: `target/octet/remediation-plan.preserves`

Plan ref: `blake3:998ea20ea5935b157012911d02d1c9fca33e269e4f08cd496e77954eac2e1f68`

Latest Octet receipt refs: artifact import `blake3:e7d820b634107b9640add9b7ac7d2bdbe879eb047846dbdcc510786f2408f653`; strict gate pass `blake3:4b0adb5e1dcb2441bd958d2405e40b8e897bf26b6dfcf03f384955ea0462f5f7`; remediation plan `blake3:998ea20ea5935b157012911d02d1c9fca33e269e4f08cd496e77954eac2e1f68`.

## Artifact refs

| Scope | Artifact | Content/canonical ref |
|---|---|---|
| workspace | `target/octet/status.json` | `blake3:ec97a472d085fa845ce8164a47e108cdfa2df371bb8b7e87afa961bc69bb0772` |
| workspace | `target/octet/summary.txt` | `blake3:21a845f9bc28be78102a6700708b8aceaaa6ffde8817fc046f70073c58c2d60c` |
| workspace/focused | `target/octet/object-corpus-receipt.json` | `blake3:0c091e130c6ef23792f25d64d78d8d07275a865d2ed292c3b5ca6578381086d1` |
| lib-only | `target/octet-lib/status.json` | `blake3:e9524c30eae46b20933a45578890f9dfaec8c6ebb79f75f9f7486b9d381d4f68` |
| lib-only | `target/octet-lib/summary.txt` | `blake3:5d978afb3680d3b0bf749eb6fefbb42c063005fd51b4ac62939a76fd56e4b453` |
| workspace/import | `target/octet/artifact-ledger-receipt.preserves` | `blake3:e7d820b634107b9640add9b7ac7d2bdbe879eb047846dbdcc510786f2408f653` |
| workspace/gate | `target/octet/gate-receipt.preserves` | `blake3:4b0adb5e1dcb2441bd958d2405e40b8e897bf26b6dfcf03f384955ea0462f5f7` |
| workspace/plan | `target/octet/remediation-plan.preserves` | `blake3:998ea20ea5935b157012911d02d1c9fca33e269e4f08cd496e77954eac2e1f68` |

Focused object corpus: object-set hash `b3:fff56689b06de4e5faed535f88faad02dee0d6742868a9baf43f0868a8f174be`, 2870 objects, 2870 pure-cache blocked, source paths include `src/cli/core/artifact.rs`, `src/cli/core/cache.rs`, `src/cli/core/catalog.rs`, `src/cli/core/chunk.rs`, `src/cli/workflow/coordination.rs`, `src/cli/workflow/delivery.rs`, `src/cli/ops/dogfood.rs`, `src/cli/evidence/gate.rs`, `src/cli/test/harness.rs`, `src/cli/workflow/job.rs`, `src/cli/ops/ledger.rs`, `src/cli/ops/node.rs`, `src/cli/ops/nixosvm.rs`, `src/cli/ops/octet.rs`, `src/cli/ops/plugin.rs`, `src/cli/evidence/receipts.rs`, `src/cli/ops/prodsoak.rs`, `src/cli/workflow/protocol.rs`, `src/cli/workflow/provenance.rs`, `src/cli/runtime/raft.rs`, `src/cli/workflow/remote.rs`, `src/cli/test/replayfixture.rs`, `src/cli/evidence/report.rs`, `src/cli/runtime/repro.rs`, `src/cli/workflow/retention.rs`, `src/cli/runtime/rewrite.rs`, `src/cli/core/schema.rs`, `src/cli/runtime/secrets.rs`, `src/cli/runtime/service.rs`, `src/cli/core/storage.rs`, `src/cli/core/transcript.rs`, `src/cli/runtime/upgrade.rs`, `src/cli/runtime/vat.rs`, `src/prod/soak.rs`, and `src/octet/remediation.rs` plus the critical runtime, node, job, retention, secrets, protocol, catalog, and CLI paths listed in `target/octet/object-corpus-receipt.json`.

## Counts

| Scope | Status | Findings | Warnings | Errors | Autofixable |
|---|---:|---:|---:|---:|---:|
| workspace | clean | 0 | 0 | 0 | 0 |
| lib-only | clean | 0 | 0 | 0 | 0 |

Top workspace lint counts: none.

Top lib-only lint counts: none.

Latest no-disabled-lints probe (`target/octet-probe-module-layout-fixed`) is `warning-only` with 8265 warnings: `non_trait_imports` 4635, `path_segment_repetition` 3040, `function_length` 436, `excessive_file_length` 137, and `module_file_count` 17. The remaining `module_file_count` entries are external registry/rustlib paths, with no Molten source `module_file_count` findings; `underscore_in_module_filename` remains zero.

Critical caveat pass: the latest lib-only and workspace runs have no active Octet findings, and the strict source gate passes. The zero finding state is configuration-clean: `dylint.toml` explicitly disables `non_trait_imports`, `path_segment_repetition`, `function_length`, `excessive_file_length`, and `module_file_count`. `underscore_in_module_filename` is no longer disabled after the source-layout relocation probe reported zero underscore-filename findings, and Molten source is now also clear for `module_file_count` after the module-directory relocation. Nix/dogfood was not rerun for this partial disabled-lint burn-down slice. If project policy requires source-remediated zero rather than config-clean zero, the remaining disabled families remain the follow-up burn-down.

Additional validation after the Nix fetch fix: `nix build .#checks.x86_64-linux.nextest --no-link --print-out-paths --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` completed successfully at `/nix/store/8fcxgyj17dkigp5idpvnzb5dv78nd4nz-molten-nextest`. The flake now maps private OnixResearch git dependencies to locked local `*-src` path inputs for unit2nix git-cache population, so the Nix builder no longer needs SSH access.

Release dogfood validation for the active `octet-source-remediated-zero` first split completed with `nix build .#checks.x86_64-linux.dogfood-local-node --no-link --print-out-paths -L --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` at `/nix/store/fwssw4qm1n291lh5f919w626pi239kid-molten-dogfood-local-node`. Evidence highlights: nextest 609/609 passed; Nix release verify `blake3:83c9a66232d736c2e63bea3ae342a763d806c9d17af2cbded16e3bfdccfaf8dc`; release bundle verify `blake3:268d3489901b6b43dbd1c6596fa5eb66a1cd5a6c3b2f231e00e41487521841ab`; promotion gate `blake3:4757c7aebf542911fc9227b5fede634484b1c1fc48e4148f297d2affe62994f4`; promotion summary `blake3:a51acc6108ab576f32a68dcd42ef60310f2d43b9372278c1dcf9c0bef859685d`; export verify `blake3:74ed8add45f1a99a35a4e5578a23e3c90fc712b409a19c2db45ec91fbce21bf0`.

## Critical surfaces

| Surface | Files | Workspace findings | Critical findings |
|---|---|---:|---:|
| source-gate-and-admission | `src/octet/gate.rs`, `src/node/runtime.rs`, `src/job/dag.rs`, `src/upgrades/mod.rs` | 0 | 0 |
| harness-and-gates | `src/harness/gate.rs`, `src/harness/schema.rs`, `src/harness/runner.rs`, `src/nixos/vm.rs` | 0 | 0 |
| node-runtime-startup | `src/node/runtime.rs`, `src/node/identity.rs`, `src/resources/mod.rs` | 0 | 0 |
| job-execution | `src/job/dag.rs`, `src/artifacts/mod.rs`, `src/typed/storage.rs`, `src/eval/cache.rs` | 0 | 0 |
| ledger-and-evidence | `src/ledger/mod.rs`, `src/evidence/mod.rs`, `src/evidence/chain.rs` | 0 | 0 |
| adapter-boundaries | `src/harness/wasm/executor.rs`, `src/harness/steel/executor.rs`, `src/effects/mod.rs`, `src/remote/dataspace.rs` | 0 | 0 |
| redaction-and-export | `src/catalog/mod.rs`, `src/catalog/mcp.rs`, `src/transcripts/mod.rs`, `src/harness/repro.rs` | 0 | 0 |
| cli-artifact-output | `src/main.rs`, `src/cli/core/artifact.rs`, `src/cli/core/cache.rs`, `src/cli/core/catalog.rs`, `src/cli/core/chunk.rs`, `src/cli/workflow/coordination.rs`, `src/cli/workflow/delivery.rs`, `src/cli/ops/dogfood.rs`, `src/cli/evidence/gate.rs`, `src/cli/test/harness.rs`, `src/cli/workflow/job.rs`, `src/cli/ops/ledger.rs`, `src/cli/ops/nixosvm.rs`, `src/cli/ops/node.rs`, `src/cli/ops/octet.rs`, `src/cli/ops/plugin.rs`, `src/cli/evidence/receipts.rs`, `src/cli/ops/prodsoak.rs`, `src/cli/workflow/protocol.rs`, `src/cli/workflow/provenance.rs`, `src/cli/runtime/raft.rs`, `src/cli/workflow/remote.rs`, `src/cli/test/replayfixture.rs`, `src/cli/evidence/report.rs`, `src/cli/runtime/repro.rs`, `src/cli/workflow/retention.rs`, `src/cli/runtime/rewrite.rs`, `src/cli/core/schema.rs`, `src/cli/runtime/secrets.rs`, `src/cli/runtime/service.rs`, `src/cli/core/storage.rs`, `src/cli/core/transcript.rs`, `src/cli/runtime/upgrade.rs`, `src/cli/runtime/vat.rs` | 0 | 0 |

## Burn-down order if source-remediated zero is required

Active burn-down slice: `cairn/changes/octet-source-remediated-zero` has moved Octet, Delivery, Protocol, Provenance, Retention, Job, Secrets, Plugin, Node, Repro, Catalog, Chunk, Cache, Artifact, Storage, Schema, Upgrade, Transcript, Rewrite, Remote, Ledger, Chain, Receipts, test Receipt, Service, Vat, Coordination, Dogfood, Raft, replay-fixture, Report, Gate, and harness run/replay CLI handling out of `src/main.rs`, then relocated the CLI shell corpus under bounded `src/cli/{core,workflow,ops,runtime,evidence,test}/` groups, moved the remaining underscore-named source files to directory/file layouts such as `src/node/runtime.rs`, `src/octet/gate.rs`, and `src/job/dag.rs`, and converted the remaining broad flat modules to directory `mod.rs` layouts such as `src/artifacts/mod.rs`, `src/ledger/mod.rs`, `src/resources/mod.rs`, and `src/runtime/envelope/mod.rs`. Molten source now has no `module_file_count` findings in the no-disabled probe; the residual `module_file_count` findings are external registry/rustlib paths. The broader state remains configuration-clean until disabled lint families are removed or narrowed and evidence is refreshed.

Module-file-count source slice validation: `cargo fmt --check`, `cargo check`, `cargo test octet_remediation`, `cargo test cli_`, `cargo test --test cliharness octet`, `cargo clippy --all-targets -- -D warnings`, refreshed workspace/lib Octet, object corpus, artifact import, strict gate, remediation plan, and Cairn strict validate/proposal/design/tasks gates. Cairn gate receipts: proposal `8725c7396eda896b7d334510b1005256a4152acd29449203bcbbb97ccd0fc9bb`, design `bf0f6408f5ddea4a63edb85ad61ceedcf0a6f4826db363ab735d5a79463e3735`, tasks `b6c8ed194ba78cd454de64297607da4ee477054df9da32de056693270a746d2e`. Nix/dogfood was not rerun for this partial source-layout lint slice.

1. Split or reshape long files/functions instead of relying on `function_length` and `excessive_file_length` disables.
2. Normalize imports and repeated path segments instead of relying on `non_trait_imports` and `path_segment_repetition` disables.
3. Resolve `module_file_count`, including external registry/rustlib paths, through Octet/config/tooling support rather than repo-only edits if needed.

## No-suppression policy

- Hidden suppressions are denied.
- Every retained active warning must have scheduled remediation or an explicit reviewed quarantine receipt.
- Strict gate keeps treating `warning-only` as deny.
- Quarantine is only for explicit, expiring, reviewed critical findings during burn-down.
- Current remaining disabled lint families are explicit in `dylint.toml`; treat them as a documented configuration caveat, not hidden source suppressions.
