# Approved Octet pin promotion

The operator approved promoting the Molten startup Octet pin to the reviewed bare-todo fix.
The consumer pin moved from `fc38f59330b626961d166febfdf1a5aa6575460f` to `c9b06bcf565c51d4a77d210e61b69ae51db9df25`.
That revision is the published fix branch head. It carries the lint fix and its verification evidence.

## Changed consumer owners

| Owner | Change |
| --- | --- |
| `flake.nix` | `octet-toolchain.url` now pins `c9b06bcf...` over `git+ssh://git@github.com`. The verified-node-replication pilot `octetRevision` literal matches. |
| `flake.lock` | Nix rewrote only the `octet-toolchain` node. New lock NAR: `sha256-UrcFiwnUQDAq3viNaNTDrtrydUzRoW0g7ZY32l1YjFE=`. |
| `crates/molten-core/src/node_startup.rs` | `OCTET_REVISION` now `c9b06bcf...`. Policy admission rejects any other revision. |
| `tests/node_startup_evidence.rs` | Fixture policy revision updated. |
| `src/octet/startup_snapshot/tests.rs` | Fixture cohort revision updated. |
| `docs/node-startup-cohort.ncl` | Exact-string contract and default updated. |

The transport changed from `github:` to `git+ssh://git@github.com` because GitHub tarball downloads now return 404 for both revisions, including the old pinned one. The lock records this form. It needs SSH access to the private repository. No credential file was changed. The unrelated `release_dependency.rs` `OCTET_REVISION` (`4367300e...`, octet-cutover) and the `executable-extent-octet` input (`cf04e894...`) were deliberately not touched.

## New cohort identities

The build plan was audited first. It required only four derivations: engine, wrapper, runtime-path, and the UI-library alias. No compiler, driver, Verus toolchain, Mantle, Darkhttpd, or package toolchain build. The build ran offline without substitutes in 1 minute 20.9 seconds. The CLI build emitted the same 15 pre-existing dead-code warnings. `nix store verify --no-trust` passed for both outputs. This is a content check, not a signature or authority claim.

| Component | Identity |
| --- | --- |
| Octet CLI | `/nix/store/c14wxbczlzph9l1j5sfy6d4nh1ghdcxz-cargo-octet-0.1.0/bin/cargo-octet`, BLAKE3 `dbf4b36ceacdcc8d372a48498ae3cb513692a22d4bff30774db20fb4e9295f4f`, deriver `fwhajcplj...drv` |
| Octet lint library | `/nix/store/1wmd44w5qrpazl97g8rv0dfkaf3wzb9j-octet-0.1.0/lib/liboctet.so`, BLAKE3 `10919bdd10b0049c1b1113f9757dc563441c6709b91efb02323ff5d32489447b`, deriver `25rbm3gf...drv` |
| Library source binding | Derivation identical for the implementation and evidence revisions. Full-source NAR for `c9b06bcf...` is the locked `sha256-UrcFiwnUQDAq3viNaNTDrtrydUzRoW0g7ZY32l1YjFE=` |

The library derivation equals the one already built from the immutable implementation revision, so the tested bytes and the pinned bytes are the same object.

## Observed behavior of the full new cohort

The smoke used the new CLI, the new library, the existing March-21 compiler and driver, and the exact pinned `octet-deny-all` hook with workspace, all-target, and all-feature flags.

| Case | Result |
| --- | --- |
| Empty fixture | Exit 0, zero findings. |
| Bare `todo!()` | Exit 2, `no_todo` errors, at least one error-level finding. |
| Message `todo!("implement later")` | Exit 2, `no_todo` errors. |

The status JSON files in this directory record these runs. The old fc38 cohort identities (`cd4db81a...` CLI, `d5kynl0p...` library) remain historical. They are superseded for new policies, not rewritten.

## Molten validation with the new constant

All 52 focused startup tests passed: 5 core, 36 Octet gate/snapshot, 5 adapter, 4 content guards, 2 CLI. Counts match the previous baseline. `nickel typecheck` passed on `docs/node-startup-cohort.ncl`. The positive fixture exported. The invalid-toolchain fixture was denied. One first attempt used `nickel export` on the schema-only contract and failed by design; the retained log records it. The verified-node-replication pilot evaluates offline with the new input (`spmr6ncr6xyhxbgr1wia2wsgiwfbd2ls-molten-verified-node-replication-pilot.drv`). That is evaluation only. The pilot check was not built or run.

## Boundaries and non-claims

No real Molten startup gate ran on Molten source. The startup guards, content guards, and consumer VM guard are unchanged. The new cohort is a verified tool set, not an approved startup cohort; an operator must mint a cohort policy with these identities, and lifecycle admission remains unimplemented. Molten's own lint gates use the separate `cf04e894...` Octet input, which still lacks this fix upstream. Full workspace, full flake check, reproducibility, and compiler correctness remain unproved. Review was single-agent and correlated.
