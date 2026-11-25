# Tasks

- [ ] [serial] Update `.config/nextest.toml` so `profile.ci` and `profile.deterministic` include `molten-core`, `molten-node-host`, and `molten-release-policy` alongside `molten`. r[molten.testing.ci_workspace_member_coverage]
- [ ] [serial] Wire `crates/molten-core` and `crates/molten-release-policy` unit tests into `flake.nix` (`checks.nextest` and member check outputs) and verify failure propagation on a deliberate negative regression. r[molten.testing.ci_workspace_member_coverage]
- [ ] [serial] Expand `[workspace.metadata.octet]` in `Cargo.toml` to include `molten-core` and `molten-release-policy`, wire hermetic workspace Octet into `flake.nix`, and resolve orphaned `checks/content-replication-octet` and `checks/dag-sync-octet` directories. r[molten.evidence_gates.ci_octet_and_cairn_hermetic]
- [ ] [serial] Wire hermetic pinned `cairn validate --strict` into `flake.nix` and align `.pre-commit-config.yaml` to avoid unpinned `../cairn` drift. r[molten.evidence_gates.ci_octet_and_cairn_hermetic]
- [ ] [parallel] Add CI feature-matrix compilation and unit test checks for self-contained optional Cargo features and audit/classify all 49 `#[ignore]` tests. r[molten.testing.feature_and_ignore_ci_accounting]
- [ ] [parallel] Add `hegeltest` to `crates/molten-core` and author deterministic `#[hegel::test]` properties for core identity, merge, reservation, and Prolly invariants. r[molten.testing.molten_core_hegel_properties]
- [ ] [serial] Update `AGENTS.md` and `docs/distributed-testing.md` to document exact CI workspace scope and run strict Cairn validation plus Nix check verification. r[molten.testing.ci_workspace_member_coverage] r[molten.evidence_gates.ci_octet_and_cairn_hermetic]
