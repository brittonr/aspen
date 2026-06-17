# Octet/TigerStyle remediation baseline

This file records the current Octet source-gate evidence and the remaining caveat for `octet-tigerstyle-remediation`.

Canonical plan receipt: `target/octet/remediation-plan.preserves`

Plan ref: `blake3:87f6ce33381572749100d7a3fde54deece90ad9e6b0e26e7be00136b246111a4`

Latest Octet receipt refs: artifact import `blake3:6c85cdc1aa20923ced247aa2c39e71de6d59899b56400a9488b6d7c20e5f086b`; strict gate pass `blake3:6eb5b9165a6c90a05f72fe72cea7f6ed3d95d30bb3a914d85e215dd878a35c70`; remediation plan `blake3:87f6ce33381572749100d7a3fde54deece90ad9e6b0e26e7be00136b246111a4`.

## Artifact refs

| Scope | Artifact | Content/canonical ref |
|---|---|---|
| workspace | `target/octet/status.json` | `blake3:a8c5804a1621d1398174f1a360c86f597cc7077bf1787fc21852201708a875b3` |
| workspace | `target/octet/summary.txt` | `blake3:1674a4493e906173010ad80e1d270db8b17af9437c4c86f5c79468f1e474482c` |
| workspace/focused | `target/octet/object-corpus-receipt.json` | `blake3:b2e969bcb4436c38e4d8ba98b4de45e280a4bdabdc824e6f9743d8310ca5c96f` |
| lib-only | `target/octet-lib/status.json` | `blake3:969a253485bd6f3fcc6994a191de87ba4a323f970db82394ff621b59d23f95d6` |
| lib-only | `target/octet-lib/summary.txt` | `blake3:1674a4493e906173010ad80e1d270db8b17af9437c4c86f5c79468f1e474482c` |
| workspace/import | `target/octet/artifact-ledger-receipt.preserves` | `blake3:6c85cdc1aa20923ced247aa2c39e71de6d59899b56400a9488b6d7c20e5f086b` |
| workspace/gate | `target/octet/gate-receipt.preserves` | `blake3:6eb5b9165a6c90a05f72fe72cea7f6ed3d95d30bb3a914d85e215dd878a35c70` |
| workspace/plan | `target/octet/remediation-plan.preserves` | `blake3:87f6ce33381572749100d7a3fde54deece90ad9e6b0e26e7be00136b246111a4` |

Focused object corpus: object-set hash `b3:88b7ab5e6a82962c0e7b18dfbf4e6c8493873aaae67eb23aa66a0e552d30e080`, 2905 objects, 2905 pure-cache blocked, source paths include `src/cli/core/artifact.rs`, `src/cli/core/cache.rs`, `src/cli/core/catalog.rs`, `src/cli/core/chunk.rs`, `src/cli/workflow/coordination.rs`, `src/cli/workflow/coordination/bounded.rs`, `src/cli/workflow/delivery.rs`, `src/cli/ops/dogfood.rs`, `src/cli/evidence/gate.rs`, `src/cli/test/harness.rs`, `src/cli/workflow/job.rs`, `src/cli/ops/ledger.rs`, `src/cli/ops/node.rs`, `src/cli/ops/nixosvm.rs`, `src/cli/ops/octet.rs`, `src/cli/ops/plugin.rs`, `src/cli/evidence/receipts.rs`, `src/cli/ops/prodsoak.rs`, `src/cli/workflow/protocol.rs`, `src/cli/workflow/provenance.rs`, `src/cli/runtime/raft.rs`, `src/cli/workflow/remote.rs`, `src/cli/test/replayfixture.rs`, `src/cli/evidence/report.rs`, `src/cli/runtime/repro.rs`, `src/cli/workflow/retention.rs`, `src/cli/runtime/rewrite.rs`, `src/cli/runtime/rewrite/input.rs`, `src/cli/core/schema.rs`, `src/cli/runtime/secrets.rs`, `src/cli/runtime/service.rs`, `src/cli/core/storage.rs`, `src/cli/core/transcript.rs`, `src/cli/runtime/upgrade.rs`, `src/cli/runtime/vat.rs`, `src/prod/soak.rs`, `src/runtime/envelope/mod.rs`, `src/runtime/envelope/tests.rs`, `src/nixos/tests.rs`, and `src/octet/remediation.rs` plus the critical runtime, node, job, retention, secrets, protocol, catalog, and CLI paths listed in `target/octet/object-corpus-receipt.json`.

## Counts

| Scope | Status | Findings | Warnings | Errors | Autofixable |
|---|---:|---:|---:|---:|---:|
| workspace | clean | 0 | 0 | 0 | 0 |
| lib-only | clean | 0 | 0 | 0 | 0 |

Top workspace lint counts: none.

Top lib-only lint counts: none.

Latest no-disabled-lints probe (`target/octet-probe-small-file-splits-final`) is `warning-only` with 8245 warnings: `non_trait_imports` 4623, `path_segment_repetition` 3040, `function_length` 436, `excessive_file_length` 129, and `module_file_count` 17. The remaining `module_file_count` entries are external registry/rustlib paths, with no Molten source `module_file_count` findings; `underscore_in_module_filename` remains zero.

Critical caveat pass: the latest lib-only and workspace runs have no active Octet findings, and the strict source gate passes. The zero finding state is configuration-clean: `dylint.toml` explicitly disables `non_trait_imports`, `path_segment_repetition`, `function_length`, `excessive_file_length`, and `module_file_count`. `underscore_in_module_filename` is no longer disabled after the source-layout relocation probe reported zero underscore-filename findings, Molten source is now also clear for `module_file_count` after the module-directory relocation, and the latest file-length micro-split lowered `excessive_file_length` from 137 to 129. Nix/dogfood was not rerun for this partial disabled-lint burn-down slice. If project policy requires source-remediated zero rather than config-clean zero, the remaining disabled families remain the follow-up burn-down.

Additional validation after the Nix fetch fix: `nix build .#checks.x86_64-linux.nextest --no-link --print-out-paths --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` completed successfully at `/nix/store/8fcxgyj17dkigp5idpvnzb5dv78nd4nz-molten-nextest`. The flake now maps private OnixResearch git dependencies to locked local `*-src` path inputs for unit2nix git-cache population, so the Nix builder no longer needs SSH access.

Release dogfood validation for the active `octet-source-remediated-zero` first split completed with `nix build .#checks.x86_64-linux.dogfood-local-node --no-link --print-out-paths -L --option eval-cache false --option substituters https://cache.nixos.org/ --option builders "" --option auto-optimise-store false --option min-free 0 --option max-free 0` at `/nix/store/fwssw4qm1n291lh5f919w626pi239kid-molten-dogfood-local-node`. Evidence highlights: nextest 609/609 passed; Nix release verify `blake3:83c9a66232d736c2e63bea3ae342a763d806c9d17af2cbded16e3bfdccfaf8dc`; release bundle verify `blake3:268d3489901b6b43dbd1c6596fa5eb66a1cd5a6c3b2f231e00e41487521841ab`; promotion gate `blake3:4757c7aebf542911fc9227b5fede634484b1c1fc48e4148f297d2affe62994f4`; promotion summary `blake3:a51acc6108ab576f32a68dcd42ef60310f2d43b9372278c1dcf9c0bef859685d`; export verify `blake3:74ed8add45f1a99a35a4e5578a23e3c90fc712b409a19c2db45ec91fbce21bf0`.

## Critical surfaces

| Surface | Files | Workspace findings | Critical findings |
|---|---|---:|---:|
| source-gate-and-admission | `src/octet/gate.rs`, `src/node/runtime.rs`, `src/job/dag.rs`, `src/upgrades/mod.rs` | 0 | 0 |
| harness-and-gates | `src/harness/gate.rs`, `src/harness/schema.rs`, `src/harness/runner.rs`, `src/nixos/vm.rs`, `src/nixos/tests.rs` | 0 | 0 |
| node-runtime-startup | `src/node/runtime.rs`, `src/node/identity.rs`, `src/resources/mod.rs` | 0 | 0 |
| job-execution | `src/job/dag.rs`, `src/artifacts/mod.rs`, `src/typed/storage.rs`, `src/eval/cache.rs` | 0 | 0 |
| ledger-and-evidence | `src/ledger/mod.rs`, `src/evidence/mod.rs`, `src/evidence/chain.rs` | 0 | 0 |
| adapter-boundaries | `src/harness/wasm/executor.rs`, `src/harness/steel/executor.rs`, `src/effects/mod.rs`, `src/remote/dataspace.rs`, `src/runtime/envelope/mod.rs`, `src/runtime/envelope/tests.rs` | 0 | 0 |
| redaction-and-export | `src/catalog/mod.rs`, `src/catalog/mcp.rs`, `src/transcripts/mod.rs`, `src/harness/repro.rs` | 0 | 0 |
| cli-artifact-output | `src/main.rs`, `src/cli/core/artifact.rs`, `src/cli/core/cache.rs`, `src/cli/core/catalog.rs`, `src/cli/core/chunk.rs`, `src/cli/workflow/coordination.rs`, `src/cli/workflow/coordination/bounded.rs`, `src/cli/workflow/delivery.rs`, `src/cli/ops/dogfood.rs`, `src/cli/evidence/gate.rs`, `src/cli/test/harness.rs`, `src/cli/workflow/job.rs`, `src/cli/ops/ledger.rs`, `src/cli/ops/nixosvm.rs`, `src/cli/ops/node.rs`, `src/cli/ops/octet.rs`, `src/cli/ops/plugin.rs`, `src/cli/evidence/receipts.rs`, `src/cli/ops/prodsoak.rs`, `src/cli/workflow/protocol.rs`, `src/cli/workflow/provenance.rs`, `src/cli/runtime/raft.rs`, `src/cli/workflow/remote.rs`, `src/cli/test/replayfixture.rs`, `src/cli/evidence/report.rs`, `src/cli/runtime/repro.rs`, `src/cli/workflow/retention.rs`, `src/cli/runtime/rewrite.rs`, `src/cli/runtime/rewrite/input.rs`, `src/cli/core/schema.rs`, `src/cli/runtime/secrets.rs`, `src/cli/runtime/service.rs`, `src/cli/core/storage.rs`, `src/cli/core/transcript.rs`, `src/cli/runtime/upgrade.rs`, `src/cli/runtime/vat.rs` | 0 | 0 |

## Burn-down order if source-remediated zero is required

Active burn-down slice: `cairn/changes/octet-source-remediated-zero` has moved Octet, Delivery, Protocol, Provenance, Retention, Job, Secrets, Plugin, Node, Repro, Catalog, Chunk, Cache, Artifact, Storage, Schema, Upgrade, Transcript, Rewrite, Remote, Ledger, Chain, Receipts, test Receipt, Service, Vat, Coordination, Dogfood, Raft, replay-fixture, Report, Gate, and harness run/replay CLI handling out of `src/main.rs`, then relocated the CLI shell corpus under bounded `src/cli/{core,workflow,ops,runtime,evidence,test}/` groups, moved the remaining underscore-named source files to directory/file layouts such as `src/node/runtime.rs`, `src/octet/gate.rs`, and `src/job/dag.rs`, converted the remaining broad flat modules to directory `mod.rs` layouts such as `src/artifacts/mod.rs`, `src/ledger/mod.rs`, `src/resources/mod.rs`, and `src/runtime/envelope/mod.rs`, and split low-risk file-length hotspots into child files (`src/runtime/envelope/tests.rs`, `src/nixos/tests.rs`, `src/cli/workflow/coordination/bounded.rs`, and `src/cli/runtime/rewrite/input.rs`). Molten source now has no `module_file_count` findings in the no-disabled probe, and `excessive_file_length` is down to 129; the residual `module_file_count` findings are external registry/rustlib paths. The broader state remains configuration-clean until disabled lint families are removed or narrowed and evidence is refreshed.

File-length micro-split validation: `cargo fmt --check`, `cargo check`, focused `cargo test envelope_`, `cargo test nixos_vm`, `cargo test cli_coordination_commands_work`, `cargo test cli_rewrite_commands_work`, `cargo test octet_remediation`, `cargo clippy --all-targets -- -D warnings`, refreshed workspace/lib Octet, object corpus, artifact import, strict gate, and remediation plan. Cairn strict validate passed (4 changes / 38 specs), plus proposal/design/tasks gates with receipts `4fbf59efcadc1cf3eca8a5988010ec9f03b9949c5214d5ab8bd4e6d0f88af1e6`, `4dc5aacff8511e22dbc29859e413ad3f11f8fe2d984f83014aceda5dc0b3ee3e`, and `3385a0515e4e3b34e327c26341ebbbd5303a3b8d7967e3496ad198346036b484`. Nix/dogfood was not rerun for this partial disabled-lint burn-down slice.

1. Split or reshape long files/functions instead of relying on `function_length` and `excessive_file_length` disables.
2. Normalize imports and repeated path segments instead of relying on `non_trait_imports` and `path_segment_repetition` disables.
3. Resolve `module_file_count`, including external registry/rustlib paths, through Octet/config/tooling support rather than repo-only edits if needed.

## No-suppression policy

- Hidden suppressions are denied.
- Every retained active warning must have scheduled remediation or an explicit reviewed quarantine receipt.
- Strict gate keeps treating `warning-only` as deny.
- Quarantine is only for explicit, expiring, reviewed critical findings during burn-down.
- Current remaining disabled lint families are explicit in `dylint.toml`; treat them as a documented configuration caveat, not hidden source suppressions.
