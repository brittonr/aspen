# Source-cutover acceptance checkpoint — 2026-10-15

## Status and authority

The zero-active-Cairns objective is **not complete**. The campaign has entered attempt
**5/20**; this attempt remains blocked, not accepted. Fresh native readback reports
**51 active changes** in the dedicated worktree:
`/home/brittonr/git/aspen-worktrees/guided-goal-completion-20261015`.

This resume performed no additional archive, commit, integration, or push. The earlier
accepted authority-review archive remains separate from the incomplete behavioral work.
No incomplete package was deleted or abandoned to reduce the count.

This checkpoint and its new retained artifacts are temporarily outside the checkout,
under `/home/brittonr/.cache/molten-guided-goal-69fuhbc2/source-remediation-evidence`,
because checkout writes fail with `EDQUOT`. This is a local verification locator, not
a product dependency, release identity, self-contained ledger export, or acceptance
exception. Historical repository evidence was not rewritten.

## Exercised results

- Default workspace/all-target tests: **2,096 passed**, zero failed/ignored, across
  **18 test binaries**. Command: `nix develop -c cargo test --workspace --all-targets
  --offline --locked --no-fail-fast`. Exact status-line capture:
  `default-workspace-test-summaries.txt`.
- Compatible-feature workspace/all-target tests: **2,115 passed**, zero failed,
  **one ignored**, across **19 binaries**. Command adds
  `--features doltlite-oracle,executable-extents,world-snapshot-vm-cohort`.
  The ignored `live_doltlite_oracle_covers_normalized_positive_and_negative_cases`
  explicitly requires the Nix-built remotes-disabled DoltLite cohort; its live behavior
  was **not executed**. Capture: `compatible-feature-test-summaries.txt`.
- Workspace documentation tests: **seven compile-fail tests passed**, all in the
  nominal-reference contract; other workspace packages reported zero documentation
  tests. Command: `nix develop -c cargo test --workspace --doc --offline --locked
  --no-fail-fast`. Result was delivered by completed job `bg_70`.
- Final reached workspace and library Octet producers (`*-workspace-06` and `*-lib-06`)
  both report `clean`, zero findings/warnings/errors, pinned
  `nightly-2026-03-21`. Their exact status files are retained as
  `octet-workspace-06-status.json` and `octet-library-06-status.json`.
- The earlier `*-workspace-05` unchanged `strict-ci` gate reports pass. Its selected
  canonical receipt is retained as `prior-05-strict-ci-gate-receipt.preserves`.
  This is **prior checkpoint evidence**, not a completed final ordered source-gate
  sequence. No partial source-corpus enumeration was admitted as a complete corpus.
- Real secrets CLI fixture and report readback passed; output reported `plaintext=redacted`.
  Canonical report: `secrets-smoke-report.preserves`, report ref
  `blake3:0ac6a1347ed145e6bc57d2679f5eb9083bdd81c16907cd8fbf7f17a5732d3d79`.
- The initial transcript smoke correctly denied an unsupported expectation. The temporary
  input was corrected to `<expect-output <value "source-smoke">>`; the actual CLI run
  then reported `decision=pass`. Pass receipt ref:
  `blake3:64df04f9631140aa3c48fffb44b54f64a6502fdca73b025a8612bd344a40381c`.
  Both the original denial and corrected pass artifacts are preserved separately.
- Real `fabric-time run-fixture --profile both` and `fabric-time show` passed, reporting
  live monotonic and simulation virtual domains, 34 evidence members, and
  `live-clock=true conformance=true`. Report ref:
  `blake3:20915d1d0bc9313576a284b047e9bbe62b65a841f0fd76cdccbd3d1011ac889b`.
  Selected report/readback: `fabric-time-smoke-report.preserves` and
  `fabric-time-cli-smoke.txt`. This does not prove the missing combined production
  scheduler/capacity/effect executor or an independent simulation dispatcher.
- Strict native Cairn structural validation passed: `valid=true`, zero findings/issues,
  receipt hash `f93d2bd654ef964677ebbd5bef3125ee363ee7a64238addd1fbfc645585dd88f`.
  Exact command output: `strict-cairn-validation.json`; live backlog readback:
  `active-changes-worktree.json`. Structural validity is not behavioral acceptance.

Commands ran from the dedicated repository, with task-owned `TMPDIR` supplied.
Native validation used the documented sibling Cairn provider and its generated policy:
`nix run --no-update-lock-file path:/home/brittonr/git/OnixResearch/cairn#cairn --
validate --root . --policy /home/brittonr/git/OnixResearch/cairn/cairn-policy/generated/cairn-policy.json --strict`.

## Unresolved acceptance failures

1. Compatible-feature Clippy with `-D warnings` failed on two `needless_borrow`
   diagnostics at `src/octet/parts/gate/p007/body.rs:91-92`. Each passes `&manifest_path`
   where `manifest_path` is already `&Path`. Smallest repair: pass `manifest_path`.
   Capture: `compatible-feature-clippy-before-repair.txt`. The chained
   `cargo fmt --check` did **not run** after this failure; an earlier formatting pass
   is not a fresh final-format claim.
2. `nix flake check --no-update-lock-file -L` failed at
   `checks.x86_64-linux.inherited-tracey-debt`: **22 dangling references**. The guard
   reads accepted `.cairn/specs`, while these F01/F08/F09/F10/F12 and two ChaosControl
   definitions currently occur in active change deltas. It reported 2,795 requirements,
   866 referenced, 1,929 uncovered, and a 1,924-entry reviewed baseline, then rejected
   dangling references before the baseline comparison. Retained bounded output pages:
   `repository-nix-gate-before-traceability-repair.txt` (retrieval footers included).
   Do not widen the baseline, suppress references, relax the guard, or sync incomplete
   packages merely to make the gate pass. Required lifecycle/direct-evidence resolution
   remains open; the full repository Nix rail has **not passed**.
3. Independent static review found a **P2 Nix test-discovery regression**. The pinned
   unit2nix/buildRustCrate discovery does not honor the three explicitly nested Cargo
   test paths at `Cargo.toml:49-58`; the hermetic nextest path can omit
   `content_replication`, `fabric_simulation_boundary`, and feature-enabled
   `executable_extent`. Preserve real discoverable entrypoints and existing feature
   gating, then exercise the actual Nix/nextest discovery path. Review record:
   `source-cutover-static-review.json`. The allocation/environment review returned no
   findings, but was static only, not runtime or cross-platform proof.

No source repair or remaining acceptance task above is marked complete.

## Storage prerequisite

`datapool/git` reached its **820 GiB quota**, with zero available space and no snapshot
usage. A new repository evidence write failed with `EDQUOT`; no partial evidence file
was left at that attempted path. Task cache, temporary builds, Nix store, and shared
Cargo target are on other datasets, so deleting them does not restore this quota.
No unrelated checkout data was deleted.

The user authorized **only an 8 GiB increase, to 828 GiB**. The pool-capacity check
reported 413,201,563,648 free bytes before that attempted change. Both available
execution routes failed: `sudo -n zfs set quota=828G datapool/git` requires a password;
direct `zfs set quota=828G datapool/git` is permission denied. No SSH execution hosts
are configured. The quota was **not changed** by this session.

A privileged operator must run:

```sh
sudo zfs set quota=828G datapool/git
```

After capacity is restored, apply the concrete source/discovery repairs, resolve
traceability without weaker policy or premature lifecycle completion, and execute
all still-required acceptance. Admit these new campaign artifacts to the repository
without rewriting historical receipts. Keep the 20-attempt cap and all sibling,
pin, frozen-fixture, history, and push boundaries unchanged.

## Capability contribution and ownership

Immediate observed outcome: clean pinned source producers and passing default,
compatible-feature, nominal-contract, and real CLI smoke paths; final acceptance is
still blocked. Durable contribution is the existing source naming/layout cutover,
explicit environment/allocation boundaries, migrated consumers, and retained
reproducible positive/negative evidence—not an archive count or completion claim.
Molten source-policy/subsystem maintainers own those boundaries; Nix integration
maintainers own complete test discovery; lifecycle/traceability maintainers own the
accepted-versus-pending requirement resolution. Reproduction inputs and precise
limits are recorded above; no production, optional-oracle, whole-system, release,
or universal-correctness claim is added.
