# Standalone node compiler graph: structural comparison only

Observed 2026-09-25 at producer local revision `f2ed5d29da03f56fe785781659d69d87497b4a63`, cwd `/home/brittonr/git/OnixResearch/molten-worktrees/node-content-service`. No Nix build, compiler, Octet gate, startup, or network action was launched for this comparison. The earlier package realization and fail-closed smoke are in `../standalone-node-nix-package/README.md`.

The checked-in `build-plan.json` has 715 package records. Its `cargoLockHash` exactly matches `sha256sum Cargo.lock`: `0b8d7a0dab775d0637067f0b8e656a3460ea6aaffa99e3f198ccde2f0c810860`. Traversing **only normal and build dependency package IDs** from `workspaceMembers."molten-node-runtime"` visits 557 records with no missing IDs: 549 crates.io, five Git, and three local (`molten-node-runtime`, `molten-node-core`, `molten-node-host`). No root `molten` or broad `molten-core` package is in that traversal. This graph records package selection, not every file the compiler read or target-specific feature selection.

The already-realized runtime derivation is `/nix/store/s8hmsz8vj7q04a4yjr29d2yh8g1gw4k7-rust_molten-node-runtime-0.1.0.drv`. Read-only `nix derivation show --option min-free 0 --option max-free 0 <drv>` reported 537 direct derivation inputs, of which 532 have Rust crate labels; the 25 planned labels absent directly include the runtime itself and transitive build-dependency crates. Read-only `nix-store --option min-free 0 --option max-free 0 --query --requisites <drv>` reported 3,567 store paths and **557 distinct Rust derivation paths with 557 distinct `rust_<name>-<version>.drv` labels**. The label sets from the lock-matched plan traversal and recursive Nix derivation references match in both directions, with no duplicate label on either side. The comparison is of labels, not a proof that individual Rust source files, cfg branches, generated outputs, or package feature bits were independently measured.

The three first-party derivations' `env.src`/`env.sourceRoot` are:

| Crate | Source | Source root |
| --- | --- | --- |
| `molten-node-runtime` | `/nix/store/1hqcb0qk0ycp6bzq5sv2wr0w9g2hj44f-source` | `source/crates/molten-node-runtime` |
| `molten-node-core` | `/nix/store/1hqcb0qk0ycp6bzq5sv2wr0w9g2hj44f-source` | `source/crates/molten-node-core` |
| `molten-node-host` | `/nix/store/5f63n7aijf0yfc25wc9mi353clwx5hhv-source` | crate root |

`nix path-info --json` returned workspace-source NAR `sha256-dVbV/FgRo6Iw/HVHB8/OkplE6y2h090Wgewf+vFaxk4=` (20,660,912 NAR bytes) and host-source NAR `sha256-M5o9RvoRULtyRF0cVHJfvVkXzvD7qZ83GzN7yf/bNfg=` (87,624 NAR bytes). The retained `nix log` for those three derivations showed Rust compiler roots `src/lib.rs` for each library plus `src/bin/molten-node.rs` for the runtime binary. `--extern` counts were 3 for core, 3 for host, 33 for runtime library (the same direct dependency counts as `build-plan.json`), and 34 for the binary (the runtime library plus its direct dependencies). None of those observed commands requested `dep-info`. The runtime log bound its source unpack and crate-specific root to the same workspace source path above.

Reproduce the dependency-label comparison without any realization:

```sh
nix-store --option min-free 0 --option max-free 0 --query --requisites \
  /nix/store/s8hmsz8vj7q04a4yjr29d2yh8g1gw4k7-rust_molten-node-runtime-0.1.0.drv |
  jq -R --slurp --slurpfile plan build-plan.json '
    def step($c): . as $ids |
      ($ids + [$ids[] | $c[.] | (.dependencies[]?,.buildDependencies[]?) | .packageId]) | unique;
    (split("\n") | map(select(length>0))) as $paths |
    ($paths | map(select(test("-rust_.*\\.drv$")) |
      split("/")[-1] | .[(index("-rust_")+1):]) | unique) as $drvs |
    ($plan[0].crates) as $c |
    ([$plan[0].workspaceMembers."molten-node-runtime"] |
      until(step($c)==.;step($c)) |
      map($c[.] | "rust_\(.crateName)-\(.version).drv") | unique) as $expected |
    {store_paths:($paths|length), rust_drv_labels:($drvs|length),
     plan_selected:($expected|length), plan_not_transitive:($expected-$drvs),
     transitive_not_plan:($drvs-$expected)}'
```

This narrows package-graph uncertainty but **does not satisfy** `molten.node-build-inputs.v1` or independent compiler/source-closure review. The compiler logs expose crate roots and `--extern` arguments, not the recursively imported Rust files or generated Rust sources. No accepted, source-file-level `build-inputs.json` or matching source inventory exists for this realized binary; hashes of a workspace snapshot and graph labels do not fill that gap. The pinned strict Octet hook still exits 2 on four host `missing_const_fn` false positives before core/runtime, and normal `run` remains denied with no receipt. No real cohort approval, consumer VM, replay, or release promotion follows from this evidence.
