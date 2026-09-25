# Standalone node selected-source gate: denied

The producer was an uncommitted worktree rooted at `93a761f19` when this check ran. The source tree was not frozen or independently inventoried and changed again afterward. This record is a failed diagnostic gate run, not a source/build/executable cohort or a startup authorization.

Working directory: `/home/brittonr/git/OnixResearch/molten-worktrees/node-content-service`. `Cargo.toml` declared `-p molten-node-runtime -p molten-core -p molten-node-host --all-targets` as the selected scope. Exact Octet invocation through the unchanged pinned hook:

```text
OCTET_PRECOMMIT_USE_INSTALLED=true sh /nix/store/9krx6k4dx3fvp5icdhcmznygh2fnkx31-source/hooks/octet-deny-all.sh --artifact-dir target/octet
```

The environment selected installed Cargo/Rust nightly-2026-03-21, disabled Cargo wrappers, used offline dependencies and two jobs, and explicitly selected these existing tools:

| Tool | Exact identity |
| --- | --- |
| Octet CLI | `c9b06bcf565c51d4a77d210e61b69ae51db9df25`, BLAKE3 `dbf4b36ceacdcc8d372a48498ae3cb513692a22d4bff30774db20fb4e9295f4f` |
| Octet library | BLAKE3 `10919bdd10b0049c1b1113f9757dc563441c6709b91efb02323ff5d32489447b` |
| Dylint driver | BLAKE3 `8e14cfcb3f0cfd993c5886e574408e11a49456dbb637b766e3d512e698d4474b` |
| Rust compiler | `rustc 1.96.0-nightly (ac7f9ec7d 2026-03-20)` |

The first attempt passed the unqualified `liboctet.so` path as `TIGERSTYLE_LINT_LIB` and failed before compilation (`dylint-driver: could not parse ...`); it has no lint coverage. The second attempt used the toolchain-qualified `liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so` alias that resolves to the same pinned object. No lint flags, source selection, severity, or baseline were relaxed.

The second attempt exited **2** after Cargo exit **101**. The retained verbatim `status.json` and complete `summary.txt` report `integration-failure`, **2,293 errors**, zero warnings, 44 findings marked autofixable but none applied. Every reported finding names `molten_core`; compilation failed before the selected host or runtime could establish clean coverage. The source is an uncommitted worktree, so these raw artifacts cannot be paired with a frozen source inventory or a production binary. No policy was minted; startup, VM, native replay, and promotion remain denied.

A separate `nix eval .#packages.x86_64-linux.molten-node.drvPath --raw` exited 1 because the locked Radicle `executable-extent-src` checkout was absent locally. The same pinned revision on the public seed returned HTTP 500. No package derivation or Nix build is claimed. Local Cargo build/tests and denied CLI smoke are distinct evidence, not replacements for the strict gate.
