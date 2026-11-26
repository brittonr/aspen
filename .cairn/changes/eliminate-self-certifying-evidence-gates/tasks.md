# Tasks

- [ ] [serial] Update `src/operator/parts/dogfood/p004/body.rs` so `release-promote` validates canonical Octet and Cairn receipts and denies placeholder strings, with positive and negative unit/CLI tests. r[molten.evidence_gates.release_promote_requires_canonical_receipts]
- [ ] [serial] Remove `--claimed-decision` pass minting from `molten test nixos-vm shard-run`, move synthetic shard fixture checks out of `checks.nixos-vm-*`, and add negative tests rejecting unverified synthetic inputs. r[molten.evidence_gates.vm_shard_derived_decision_only]
- [ ] [serial] Rename unsigned startup denial VM check in `flake.nix` accurately, wire a real multi-node VM execution check, and update `AGENTS.md` and `docs/distributed-testing.md`. r[molten.evidence_gates.flake_checks_canonical_non_circular]
- [ ] [serial] Replace self-copy `deterministic-drift-gate`, disposable `requirement-traceability-gate`, and stdout/`journalctl` `grep` gating in `flake.nix` with canonical receipt verification over real Aspen outputs. r[molten.evidence_gates.flake_checks_canonical_non_circular]
- [x] [serial] Clean up `tests/evidence-matrix.ncl` to eliminate sequential placeholder hashes, remove 10-row aliasing of one `--ignored` test, and align all requirements mapped in `config/capability-report/capabilities.json`. r[molten.testing.evidence_matrix_native_producers]
- [x] [parallel] Implement native execution producers for the locally runnable product capabilities in `config/capability-report/capabilities.json` (durable state recovery, effect uncertainty, cross-process transport, world replay/promotion/restore, control-plane failover) with positive/negative tamper controls. r[molten.testing.evidence_matrix_native_producers]
- [ ] [serial] Run focused unit/CLI suites, workspace Octet, Clippy, and strict Cairn validation after all gate repairs. r[molten.evidence_gates.release_promote_requires_canonical_receipts] r[molten.evidence_gates.vm_shard_derived_decision_only] r[molten.evidence_gates.flake_checks_canonical_non_circular] r[molten.testing.evidence_matrix_native_producers]

## Verification (2026-11-26, tasks 5–6: evidence matrix and native capability producers)

- Matrix before: 18 rows, every one with a sequential placeholder digest. Twelve were `tests/collaborationnative` rows; ten of those aliased one `--ignored` test across positive and negative rows of five requirements.
- Matrix after: 25 rows, each with one unique command and refs computed from real artifacts:
  - 2 hardening rows: each test asserts that its own row matches the manifest it computes.
  - 1 `cliharness` positive `cli_receipt_first.normative_artifacts` row: the test asserts its input content refs and its canonical `ci-test-run` receipt ref.
  - 22 `molten evidence qualify --scenario <id>` rows: plan ref plus runtime-artifact ref. Each scenario's own test runs it once and must reproduce the checked-in ref, so equality across separate runs is also the determinism check.
- Not claimed, recorded as tracked debt in `docs/proof-workflow.md#evidence-matrix-tracked-debt` and in the `tests/evidence-matrix.ncl` header (no longer only in this file):
  - `native_effect_{installation,authority,completion,lifetime,consumer}`: no canonical artifact without a signed cohort.
  - `cli_receipt_first.negative_fail_closed`: the test emits no receipt by design.
  - `native_effect_uncertainty` rows are marked PARTIAL: they use the producer-owned `EffectRuntime` port, not the native supervisor.
- Unqualified (no producer): `post-effect-uncertainty-same-process`, every `*-nixos-vm` and `*-cross-host` family, and `cross-host-transport`.
- Placeholder refs (including exemption `evidence_ref`) and one command reused across rows now fail closed in three places: the Nickel contract, `build_evidence_matrix_manifest` (`placeholder-ref:` / `aliased-command:`) and `tools/capability-report.rs` (`validate_matrix`).
- Producers live in `src/capability_qualification/products/`: `durable`, `effect`, `transport`, `world`, `control_plane`, plus `peer`, which runs distinct-process peers via the hidden `molten evidence qualify-peer`.
- Every (requirement, family, polarity) in the producer families has exactly one scenario. A passing receipt must be identical to the admitted binary's re-execution.
- Tamper controls cover: flipped checks, relabelled scenarios, swapped plans, environment promotion, unobserved scenarios, an effect with no started child or an untyped failure, a withheld acknowledgement versus no frame, and an untampered durable snapshot control.
- Effect children prove they started by creating a start marker. Native-work uncertainty must have the typed `UncertainExternalOutcome` class.
- `aremote 'cargo nextest run -p molten --profile ci -E "test(/capability_qualification|hardening_core::tests::evidence_matrix|cli_ci_run_receipt|cli_evidence_qualify_peer/)"'` (debug): 48 passed in 136 s. 6 were slow; the world tests ran about 77–81 s each, inside the reviewed `.config/nextest.toml` budget (60 s × 4).
- The same filter under the default profile: 48 passed in 67 s; world tests about 65 s each.
- `aremote 'cargo clippy -p molten --lib --tests --bins -- -D warnings'`: clean.
- `nickel export tests/evidence-matrix.ncl`: ok. Placeholder, repeated-nibble and aliased-command mutants are each rejected.
- `docs/atlas.ncl` regenerated with `scripts/generate-atlas.py --check`: ok.
- Capability-report tool `--self-test`: ok, run through a temporary crate that carries the root `[patch]` sections. Plain `cargo -Zscript tools/capability-report.rs` cannot fetch the ssh-patched `onix-artifact` dependency on leviathan; that problem predates this change.
- `cairn validate --strict`: valid.
