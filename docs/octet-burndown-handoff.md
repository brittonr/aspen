# Aspen Octet burn-down handoff

Goal: reduce the no-disabled Octet finding count in `aspen` to zero without relaxing checks.
One slice per commit, each with a fresh probe, focused validation, and a record in
`docs/octet-tigerstyle-remediation.md`.

## Current source checkpoint

Attempt 15/20 natively synced and archived
`bind-delivery-duplicate-results-to-original-claims`. Its
[completion record](../evidence/cairn-source-remediation-2026-10-15/attempt-15-delivery-result-acceptance/completion.json)
binds 26 real public-service outcomes over capability-rooted Redb storage,
independent fresh-root positive/expected-deny runs with identical canonical
results, and native receipt-backed coverage for all four F08 requirements.
Timer/status ports remain controlled observations; live worker effects and
whole-repository release coverage are not claimed.

Attempt 14/20 natively synced and archived `fix-node-shutdown-admission` on
2026-10-16. Its
[completion record](../evidence/cairn-source-remediation-2026-10-15/attempt-14-shutdown-admission-acceptance/completion.json)
binds three real CLI denial cases, fourteen unchanged protected lifecycle
artifacts, and an admitted stop. The normal suite also covers isolated missing
authority, prior shutdown-evidence preservation, adapter receipt order, and an
effect-publication error.

The unchanged product Rust source reuses the
[attempt-13 ordered acceptance](../evidence/cairn-source-remediation-2026-10-15/attempt-13-retry-arithmetic-acceptance/completion.json):
clean configured producers, the unchanged passing `strict-ci` receipt, 2,095
workspace/all-target tests, seven doctests, both explicit compatible feature
cohorts, Clippy with denied warnings, formatting, and the passing repository Nix
gate on `x86_64-linux`. This is reuse of recorded execution, not a new matrix run.

Current strict Cairn validation and the unchanged live traceability guard pass
after archival; 48 active changes remained at that checkpoint. This is local,
unintegrated evidence, not completion of those changes, whole-core Octet
warning-debt removal, arbitrary live-adapter shutdown, a release, or
authorization for a Molten remote push.

On 2026-10-19, thirteen fully checked packages were synced and archived:
`bound-lifecycle-probe-results`, `bound-offline-cluster-run-index`,
`bound-resource-deletion-facts`, `complete-owned-license-package-policy`, and
the nine caller-built input preflights (`preflight-caller-built-chunk-manifest`,
`preflight-cluster-harness-fixture-bytes`, `preflight-direct-provenance-binding`,
`preflight-performance-phase-count`, `preflight-prolly-snapshot-profile`,
`preflight-raft-placement-members`, `preflight-redb-durable-log-restore`,
`preflight-resource-lifecycle-growth`, `preflight-simulation-world-before-hosts`).
Strict Cairn validation then reported 47 active changes and zero issues
(`receipt_hash blake3:e5f3a1b6a3c35e3d925da91c9e599be7bbaa9ded7f44ca64f4c3ec17b0f78057`).
The two F09/F10 packages retain their final Nix acceptance task. On
2026-10-19 the required `nextest`, `clippy`, and `nextest-config` closures
passed on the `aspen1` builder (1,741 nextest tests, all passing) after the
local Nix daemon blocked on a root-owned `nix-store --add-root` process. The
repository flake evaluation passed with `nix flake check --no-build`; the full
flake build was attempted but stopped while building a cold NixOS VM closure.
Exact records are under
[`evidence/cairn-source-remediation-2026-10-15/attempt-21-remote-nix-acceptance/`](../evidence/cairn-source-remediation-2026-10-15/attempt-21-remote-nix-acceptance/verification.md).

### Earlier source checkpoint (attempt 5/20)


The 2026-10-15 bounded Cairn completion campaign's attempt 5/20 reached a clean
configured workspace producer and a clean library producer in the dedicated
`guided-goal-completion-20261015` worktree. This is local, unintegrated source
evidence, not a release identity or completion authority for every active change.

- `target/octet-guided-goal-remediation-20261015-workspace-07/status.json` and
  `target/octet-guided-goal-remediation-20261015-lib-07/status.json` both report
  `clean`, zero findings, zero warnings, and zero errors.
- The pinned producer remains `nightly-2026-03-21`; no lint was disabled or
  threshold relaxed. The complete object-corpus receipt covers all 1,890 Rust
  files under `src/`, including any ignored or hidden inputs.
- The freshly Nix-built Molten CLI passed artifact import, the unchanged
  `strict-ci` gate, and remediation-plan generation. Gate receipt:
  `blake3:a4a5be2eca77151988f3448f9a8b0ae6dfc70635a0d456eac5170bcefa4f8f41`.
- Current `cargo fmt --all --check` and the exported Nix workspace Clippy
  check passed. Final compatible-feature Cargo acceptance is not complete.
  The node-state, capability-store, materialization, and optional VM-cohort
  deny-all Octet Nix checks passed.

The [post-recovery checkpoint](../evidence/cairn-source-remediation-2026-10-15/source-gate-after-recovery.json)
records the retained receipts, 1,694 passing hermetic tests, and the remaining
Tracey/lifecycle and unavailable nominal-domain-provider blockers. At that
attempt-5 checkpoint, native Cairn readback reported 51 active changes. A Cargo CLI retry timed out waiting
for the shared package-cache lock; the Nix-built CLI is real execution evidence,
not a claim that the complete ordered Cargo source-gate sequence passed.

Repeat the complete ordered source-gate sequence in the [development guide](../README.md#development),
not an ambient `cargo-octet` or a producer exit code alone. Molten source-policy
and affected subsystem maintainers own the clean naming/layout conventions,
explicit environment/allocation inputs, and subsequent acceptance evidence.
Package-specific runtime, feature, Nix, review, sync, and archive obligations
remain separate. Historical counts and failed-attempt evidence below are not
current pass claims.

### Traceability lifecycle closure (attempt 8/20)

The [attempt-8 completion receipt](../evidence/cairn-source-remediation-2026-10-15/attempt-08-traceability-closure/completion.json)
records successful native sync and archive of `support-active-delta-traceability-before-sync`
in the dedicated worktree. Strict Cairn validation has zero findings. The actual
post-sync guard reports 2,799 accepted requirements, 875 referenced, exactly
1,924 inherited gaps, 23 pending references, and zero dangling identities.
Preview and execution preserve native input/policy/profile and planned merge
identities; both mode-dependent envelope hashes and exact mutation manifests
are retained. All nine archived files preserve their contents. Molten source-policy
and traceability maintainers own this boundary and its reproducible guard command.
The worktree now has 52 active changes; canonical integration, immutable Octet
adoption, other change acceptance, and any Molten remote push are not claimed.

## Historical burn-down snapshot

- Repository: `/home/brittonr/git/OnixResearch/aspen`, branch `molten`, HEAD `de53ab3ba`.
- Gate config: `dylint.toml` is `[octet] disabled_lints = []`, so every lint is active.
- Latest probe: `target/octet-burndown/module-layout-b-0/summary.txt`, status `warning-only`,
  3619 warnings, 0 errors, 0 autofixable.
- Session start probe for comparison: `target/octet-baseline/summary.txt`, 3963 warnings.
- Pre-remediation baseline recorded in the doc: 6754 warnings.

Probe command:

```sh
CARGO_INCREMENTAL=0 cargo octet check --artifact-dir target/octet-burndown/<slice-name>-0
```

Read counts from `<artifact-dir>/status.json` and per-lint counts from the `Index:` block of
`summary.txt`:

```sh
sed -n '/^Index:/,$p' target/octet-burndown/<slice>-0/summary.txt \
  | awk 'NF>=4 && $1 ~ /^F[0-9]+$/ {print $2}' | sort | uniq -c | sort -rn
```

## Findings by family (probe `module-layout-b-0`)

| Lint | Findings | Notes |
|---|---:|---|
| `path_segment_repetition` | 1877 | item name repeats an ancestor module word; needs ~1030 renames |
| `excessive_file_length` | 552 | 307 distinct files over the 300-line limit |
| `borrowed_argument_types` | 368 | mostly `&mut Vec<T>` sink parameters |
| `unbounded_collection_growth` | 216 | unbounded `push`/`insert`/`collect` |
| `function_length` | 204 | long function bodies |
| `too_many_parameters` | 143 | wide signatures; the input-struct repair is proven |
| `no_unwrap` | 80 | see the caveat below |
| `non_trait_imports` | 34 | repair tool exists; see the refused list below |
| `usize_in_public_api` | 32 | platform-sized ints in public signatures |
| `explicit_defaults` | 31 | see the caveat below |
| `underscore_in_module_filename` | 21 | layout move; 21 of 56 already done |
| `module_file_count` | 18 | module directory has too many files |
| `ambient_clock` | 17 | needs a clock port |
| `ambient_env` | 14 | needs an env port |
| `unbounded_channel` | 9 | production port uses an unbounded mpsc |
| `no_recursion` | 2 | the structural Preserves scan |
| `no_panic` | 1 | `tests/content/replication.rs` |

Cleared this session: `catch_all_on_enum` (was 8). Seventeen of the 72 lints in the Octet catalog still
report findings; the rest report zero.

## Slice recipe that works

1. Read `.agent/napkin.md` and `docs/octet-tigerstyle-remediation.md` first.
2. Pick exactly one lint family or one file.
3. Write the smallest change that removes the finding without changing public behavior.
4. Run, in this order:
   ```sh
   cargo fmt --all
   cargo check --workspace --all-targets
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test -p molten --lib          # 1458 tests, ~40-60 s
   # plus a focused filter for the touched area
   ```
5. Probe with the command above into a new artifact directory.
6. Compare per-lint counts against the previous probe and confirm no other family moved.
7. Update `docs/octet-tigerstyle-remediation.md`: the "Current state" probe path, commit, total,
   the `## Counts` table, the "Top workspace lint counts" line, and one new slice paragraph at the
   end of the slice log (before `## No-suppression policy`).
8. Append one line to `.agent/napkin.md`.
9. Commit only the touched paths (see the pitfall below).

Long commands go through pueue. A probe takes about 2 minutes, clippy 1-3 minutes, and the full
library suite about 1 minute after the first build.

## Hard rules

- Never add to `dylint.toml` `disabled_lints`. Never loosen a threshold. Never add an `#[allow]`
  without a specific reason.
- Do not push. Do not open a pull request.
- Do not edit `flake.lock`.
- `git commit` in this repository must name paths explicitly:
  ```sh
  git commit -F /tmp/msg.txt -- src docs/octet-tigerstyle-remediation.md .agent/napkin.md
  ```
  Two files are permanently staged and must not enter a commit:
  `aspen-orchestration-batch-implementation.zip` and `docs/audits/logical-bugs-2026-09-08.md`.
  A bare `git commit` sweeps them. If that happens, `git reset --soft HEAD~1`, then re-commit with
  explicit paths; both files return to the staged state.
- Preserve unrelated work. The working tree normally shows only those two staged files.
- Commit message style: imperative subject, body explains why, no conventional-commit prefixes.

## Tooling built this session

`scripts/octet-qualify-imports.rs` repairs `non_trait_imports` from a probe summary. It now:

- skips the identifier after `fn`, so a method definition name survives;
- removes each flagged private import and qualifies the references that import owns;
- follows references into descendant modules;
- scopes a nested `use` inside a function body or a nested module body to that block;
- refuses a nested import that no reference can qualify, a public import, a shared use group, an
  attributed import, an inline format capture, a second binding, a field shorthand, a
  `#[path]` module, and an `include!` part-body splice.

Run `cargo -q -Zscript scripts/octet-qualify-imports.rs --self-test` before and after changing it.
`scripts/octet-predicate-names.rs` resolves `bool_naming`, which now reports zero.

## Next slices, cheapest first

Close a small family completely before starting a large one.

1. `no_panic` (1) — `tests/content/replication.rs`. That file also holds 9 `no_unwrap` and 3
   `non_trait_imports` findings and a `#[path]` module the tool refuses, so treat it as one slice.
2. `no_recursion` (2) — `src/preserves/parts/rail/p001/body.rs:1810`
   (`visit_structural_value`). It carries a path stack, a first-match result, and per-node and
   per-depth bounds, so the conversion needs an explicit work stack that reproduces preorder and
   the first bound failure. Do not change the scan semantics.
3. `underscore_in_module_filename` (21) — 10 remaining `src` files plus three `tests/*.rs` targets.
   Follow the `x_y.rs` to `x/y.rs` pattern and keep the module name with
   `#[path = "x/y.rs"]`. Files with real submodules (`src/fabric_crypto_identity/artifact_auth/mod.rs`,
   `src/cli/runtime/fabric_time/mod.rs`, `src/cli/runtime/fabric_simulation/mod.rs`,
   `src/cli/runtime/system_extension/mod.rs`, `src/audit/ast/grep.rs`) need their inner paths adjusted
   too. The three relocated integration sources now have explicit `Cargo.toml` targets preserving their binary names.
   After any move, search for path consumers:
   ```sh
   git grep -n -F "<old/path>.rs"
   ```
   `flake.nix` runs `test -f "$counterexample_path"` on
   `evidence/tracey/runtime-spine-content-refs-direct-repairs.ncl`, and the checked-in `.json` must
   be regenerated with `nickel export <ncl> --format json`.
4. `ambient_env` (14) and `ambient_clock` (17) — inject a port instead of reading the environment or
   the clock. This is a design change; keep the core pure and put the effect in the shell.
5. `unbounded_channel` (9) — the Raft time port uses `tokio::sync::mpsc::unbounded_channel`. A
   bounded channel changes backpressure behavior in `TokioReplicaPort`, so this needs a design
   decision plus tests for the denial path.
6. `usize_in_public_api` (32) — change the public signature to `u64` and carry the conversion at the
   boundary. Sites are mostly single-file.
7. `explicit_defaults` (31) — see the caveat below before starting.
8. `non_trait_imports` (34) — the repair tool refuses the remaining files for these reasons:
   - `src/fabric/mod.rs` (16): a descendant re-imports `MoltenError` under a file-level allow, so
     the tool stops at the descendant check. A safe tool change is to skip that name in that
     descendant instead of refusing the parent repair.
   - `tests/nativesystemextension.rs` (4) and `tests/content/replication.rs` (3): `#[path]` modules.
   - `src/main/root/command.rs` (3): `use clap::Parser;` exists only for method resolution on
     `Cli::try_parse_from`, so no reference can be qualified.
   - `src/plugin/parts/host/p000/body.rs` (2): shared use group.
   - `src/cli/runtime/fabric_time/mod.rs` (2) and `src/cli/runtime/fabric_simulation/mod.rs` (2): a
     `pub(crate) use` re-export feeds a glob re-export in `src/main.rs`, so it needs a module
     re-export instead.
   - `src/retention/parts/mod/tests/m000/p000/body.rs` (1): a part body shares one module scope.
   - `src/chunk/parts/store/p000/body.rs` (1): an attributed import.
9. `borrowed_argument_types` (368) — nearly all are `&mut Vec<T>` sink parameters, one function at a
   time, 184 functions across about 100 files. The repository's proven repair is a bounded
   diagnostic sink with a fallible push, as in `src/testing/distributed/parts/p003/body.rs` and
   `src/node/parts/iroh/p000/body.rs`. The largest single file is `src/testing/hardening.rs`
   (44 findings, 22 `&mut Vec<String>` parameters, 54 push sites, 6 helpers that currently return
   `()` and would become `Result<()>`). Changing a parameter from `&mut Vec<String>` to
   `&mut impl <trait>` leaves call sites unchanged because `Vec<String>` implements the trait, but
   the push becomes fallible, which is why those six helpers need `Result`. A shared public sink
   type is better than a copy per module; `crate::bounded` already holds `VecSink`, `push`,
   and a `pub(crate)` `DiagnosticSink` struct. Watch `path_segment_repetition` when naming anything
   inside `src/bounded`: a name containing the word `bounded` adds a finding.
10. `unbounded_collection_growth` (216), `function_length` (204), `too_many_parameters` (143),
    `excessive_file_length` (552), `path_segment_repetition` (1877) — large campaigns. The doc's own
    next-step list names file splitting first. `path_segment_repetition` needs about 1030 item
    renames and is the only family that changes the shape of the work; treat it as its own plan.

## Lint limits that need an Octet-side decision

Both were found while scoping families, and both block product-source repairs:

- `no_unwrap` reports `.expect()` in `tests/*.rs`. The test-context check accepts `#[cfg(test)]` on
  an enclosing item, but an integration-test `#[test]` function is not detected, so 70 of 80
  findings are integration test code that is not production code.
- `explicit_defaults` fires on `#[serde(default)]` field attributes, which points at
  attribute-generated code rather than a call site.

Do not add `#[allow]` to work around either. Record the decision instead, or change the rule in the
`octet` sibling repository with its own validation.

## Environment notes

- `cargo octet` resolves to `cargo-octet 0.1.0` with toolchain
  `nightly-2026-03-21-x86_64-unknown-linux-gnu`; the probe reports warning-only, so exit code alone
  is not acceptance evidence. Read `status.json`.
- `cargo test -p molten --lib` has one known flake under full parallel runs:
  `fabric_execution::tests::live::live_adapter_preserves_rejected_exit_and_descendant_teardown`
  fails on process spawn, passes alone and on the next full run. Do not treat it as a regression,
  and do not weaken the test.
- A full `nix flake check` was not run this session. It is slow and needs
  `--option substituters https://cache.nixos.org/ --option builders ""` in this environment. Some
  checks read `evidence/tracey/*` and `flake.nix` fixed paths, so keep evidence paths truthful when
  moving files.
- Pueue is the right runner for probes, clippy, and Nix work. Report task id and log path.

## Caller-built chunk manifest admission

The source-owned parser caps manifest chunks at `MAX_CHUNK_STORE_CHUNKS =
65,536`, but public `ChunkManifest` fields permit an external Rust caller to
bypass parsing. Both `verify_remote_range_readback` and
`plan_chunk_traversal_sync` now invoke the same count preflight in
`validate_chunk_manifest_identity`, before allocating range refs or traversal
plans. A 65,537-chunk value is denied by the parser's existing count
diagnostic; exactly 65,536 chunks remain eligible for the previous
manifest-ref and content-ref checks, strategy validation, and verified
readback. This is not a full authenticity proof for a caller-built manifest,
nor a waiver of other full-catalog Octet warnings. The change-local Cairn is
`preflight-caller-built-chunk-manifest`; the isolated owner starts at clean
canonical HEAD and the detached 64-package composition is a separate
integration target.

The same remote readback helper now preflights its separate caller-supplied
`chunk_bytes` tuple count at 65,536 before building a lookup. Its
`BTreeMap<&str, &[u8]>` borrows the caller's refs and byte slices instead of
cloning every supplied byte buffer, including irrelevant extras; lookup keeps
last-duplicate-wins behavior. Exactly 65,536 tuples with one four-byte
required chunk remain eligible; 65,537 are denied before byte copying, and
an admitted overflowing range still fails its existing range check. Required
byte buffers and aggregated verified output use the existing 1 GiB
`MAX_CHUNK_STORE_OBJECT_BYTES` boundary before hashing or appending, so
caller-supplied chunk size and declared range length cannot create an
unbounded output allocation; no new count or byte policy was added.

## Additional caller-built input preflights

The same F09/F10 review found public producers that skipped an existing
source-owned count or byte envelope. These now preflight the existing bound
before cloning, formatting, or traversing caller-built values:

- `consensus_placement_report` denies more than `MAX_RAFT_MEMBERS = 32`
  admitted members before diagnostics or cloning (`preflight-raft-placement-members`).
- `build_benchmark_run` and `validate_benchmark_run` deny more than the
  reviewed three benchmark phase groups before normalization, diagnostics, or
  hashing (`preflight-performance-phase-count`).
- `verify_remote_range_readback` and `plan_chunk_traversal_sync` apply the
  parser's 65,536-chunk manifest bound to caller-built manifests and the
  65,536 supplied-chunk tuple bound (`preflight-caller-built-chunk-manifest`).
- `RedbDurableStateAdapter::open` preflights persisted record count and
  borrowed per-record byte budgets against the current profile before owning
  legacy log values (`preflight-redb-durable-log-restore`).
- `evaluate_placement` and `validate_reconcile_completion` deny one-past the
  existing 64-constraint and 128-effect-intent bounds before diagnostics
  (`preflight-resource-lifecycle-growth`).
- `prepare_world_for_manifest` admits the simulated world before constructing
  or activating reference extension hosts (`preflight-simulation-world-before-hosts`).
- `validate_probe_results` denies more than `MAX_REFS = 1,024` probes before
  cloning status refs (`bound-lifecycle-probe-results`).
- `load_prolly_snapshot` validates the supplied profile before any block read
  or traversal allocation (`preflight-prolly-snapshot-profile`).
- `evaluate_build_verification_binding` applies the existing 64-ref envelope to
  direct receipts and forged build-record refs before binding
  (`preflight-direct-provenance-binding`).
- `evaluate_deletion_gate` applies the existing owner, finalizer, evidence-ref,
  and fact-byte envelopes before deciding cleanup
  (`bound-resource-deletion-facts`).
- `parse_manifest`, direct `plan`, and the cluster CLI manifest read enforce
  `MAX_CLUSTER_MANIFEST_BYTES = 1,048,576`, the 512-node lifecycle bound, and
  the one-megabyte fixture read before harness output-root preparation
  (`preflight-cluster-harness-fixture-bytes`).

Exact-limit admitted shapes keep their prior behavior and error order; each
change package carries its own positive, negative, and smoke evidence. These
checks bound local intake and allocation only. They are not authenticity,
authorization, profile-correctness, or release evidence for caller-built
values.

## Authority limits

The session that wrote this had authority to edit source, run focused checks, and commit locally on
`molten`. It did not have authority to push, to open a pull request, to relax the gate, to rewrite
history, or to change `dylint.toml` policy. Keep those limits unless the operator grants more.
