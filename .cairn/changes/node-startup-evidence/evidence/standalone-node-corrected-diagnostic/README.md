# Corrected-Octet selected-source diagnostic: denied, not an approved gate

Producer cwd: `/home/brittonr/git/OnixResearch/molten-worktrees/node-content-service`. Frozen Git revision: `8062a8e364142361f82f42db4e0819608b123b48`; Rust/manifests are the same as source checkpoint `510cd65c8247eb0ff112fea7d72b2d4f47ec71e2`. The committed workspace metadata selects all three normal-node packages (`molten-node-runtime`, `molten-node-core`, `molten-node-host`) with `--all-targets`. This diagnostic did **not** edit that selection or the strict hook flags, severity, or baseline.

The only tool change relative to `../standalone-node-exact-core-gate/` was the lint object: an existing locally audited corrective Octet library at `/nix/store/r5aw3lz2sa6h2cg9lphy881w0w1ra2id-octet-0.1.0/lib/liboctet.so`, BLAKE3 `e465f12461fb498e21a531d2c48a36c6fb4338abd60429f6cfba7622ecc9d194`, reached via a fresh toolchain-qualified symlink under `/tmp/molten-node-octet-corrected.7kGVeH/lib`. Retained prior `../borrowed-growth/README.md` describes its vector-growth classifier repair and source; `../const-calls/README.md` describes the preceding trait/non-const-call classifier repair. These are focused fixes, **not** approval to change Molten's pinned `octet-toolchain` input or an accepted release toolchain cohort.

Unchanged pinned CLI BLAKE3 `dbf4b36ceacdcc8d372a48498ae3cb513692a22d4bff30774db20fb4e9295f4f`, Dylint driver BLAKE3 `8e14cfcb3f0cfd993c5886e574408e11a49456dbb637b766e3d512e698d4474b`, and existing Rust `nightly-2026-03-21` (`rustc 1.96.0-nightly (ac7f9ec7d 2026-03-20)`). Exact diagnostic invocation:

```sh
PATH=/nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21/bin:/tmp/molten-node-octet-pinned-c9-cli/bin:$PATH \
LD_LIBRARY_PATH=/nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21/lib \
RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2 \
TIGERSTYLE_TOOLCHAIN=nightly-2026-03-21-x86_64-unknown-linux-gnu \
TIGERSTYLE_DRIVER_PATH=/nix/store/r5bzbvda2ydnz09c6vxvqhxsmh37nhpw-dylint-driver-5.0.0/bin/dylint-driver \
TIGERSTYLE_LINT_LIB=/tmp/molten-node-octet-corrected.7kGVeH/lib/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so \
OCTET_PRECOMMIT_USE_INSTALLED=true \
sh /nix/store/9krx6k4dx3fvp5icdhcmznygh2fnkx31-source/hooks/octet-deny-all.sh --artifact-dir target/octet-corrected-diagnostic
```

Exit **2**, Cargo exit **101**, `integration-failure`, **287 error findings, zero warnings, ten marked autofixable but none applied**. `molten-node-host` compiled without a finding under this corrected library; all 287 reported findings are in `molten_node_core`. Compilation ended there, so **runtime strict coverage is still unknown**. Distribution in the raw summary: `path_segment_repetition` 104; `assertion_density` 56; `fragile_exhaustive_enum_match` 51; `ambiguous_params` 14; `excessive_file_length` 11; `bool_naming` 10; `unbounded_collection_growth` 9. Other categories include five non-const suggestions, three fallible-integer reports (with three companion sentinel reports), and one `no_unwrap` on a checked compile-time reference length. The complete 287-location index is retained in the unedited `summary.txt` here.

Raw `status.json` and `summary.txt` match their generated counterparts byte-for-byte, SHA-256 `aa05e8401a4b1e9ef341ae2f9d07f34e80d5677a6d54657421a30ab473f375f3` and `b212d15f24272f7dc48c9f689a97a053f29118708b1f3efee8e04188a0bc3354`. This diagnostic only maps a new subset of findings. It cannot replace the pinned failing gate, independently bind the source to a Nix-built executable, authorize `real-cohort-not-approved`, launch a normal node, prove a VM lifecycle, or close either Cairn. The policy/owner must decide how the strict style rules apply to public compatibility names, deliberately sealed enums, and fallible runtime functions; fake assertions, catch-all matches, blanket allows, or shorter source selection are not acceptable repairs.
