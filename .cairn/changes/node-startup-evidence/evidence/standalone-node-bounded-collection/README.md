# Bounded collection classifier: selected-source strict gate remains denied

Producer worktree: `/home/brittonr/git/OnixResearch/molten-worktrees/node-content-service`, parent `2770c075fef8ce39f14aa589760980044f2f787f`. Octet owner worktree: `/home/brittonr/git/octet-worktrees/molten-reviewed-20260925`, reviewed local commit `08f7784bc314620511c466ca948565429c6aba6c` atop `c1b0e0392aeebbafe1971deda6f4767f20b05fba`. The eventual producer commit containing these files, not its parent alone, identifies the exact edited producer sources. No consumer Octet pin, source selection, strict profile, hook, lint severity, or baseline was changed.

The owner classifier now recognizes a named constant array or an immutable borrowed slice with a preceding exiting `len() > FINITE_LITERAL_CONST` guard, provided each iteration grows the collection by at most one element. It does not infer a bound for a dynamic manifest slice, nested loop, `extend`, mutable source, or non-exiting/vacuous guard. The independent UI fixture passed with **11 expected warnings**; bounded const-array, finite slice, and zero-bound slice cases emitted none. The pinned Rust owner library suite passed **155 tests**. The library was built from the owner worktree by a scoped offline `nix build .#octet`, after a dry-run showed exactly one Octet derivation and no compiler or Stage0 work. Built library: `/nix/store/41634n8ssg1mp912nscbi5vg3wprw40a-octet-0.1.0/lib/liboctet.so`, BLAKE3 `420a11001120b60a440999e2e92519c30ab7279121af6ae576cef86a63b70dd4`.

In producer `fabric_durability/transition.rs`, a compile-time `usize::BITS <= u64::BITS` assertion justifies exact `usize`-to-`u64` count casts instead of three impossible-conversion `u64::MAX` fallbacks. The `fabric_durability::tests` suite passed **15 tests**, including buffered/flushed crash counts and recovery counts above the diagnostic limit. An independent throwaway public-API executable, compiled against `cargo build --offline --locked -p molten-core` with the same installed compiler, invoked `simulate_process_crash` and `evaluate_recovery` on a populated state. It exited 0 and printed `affected_items=1 buffered_after=0 snapshot_count=1 unresolved_effect_count=1`; its source and executable were removed after the smoke. Source BLAKE3: `transition.rs` `52feeb4c2c797db181507a871ff10686f820332085381fd1a3c22431ec433497`; `tests.rs` `d88a8115e2329398e8d70dcadd98f4fd7b23f343a7b4ebd84bef3634ac989253`. Owner classifier source BLAKE3 `f372303b3f54b4793bf478a40f4d55c41a467c54512b8589bc24af44730fe086`.

Unmodified installed hook `/nix/store/9krx6k4dx3fvp5icdhcmznygh2fnkx31-source/hooks/octet-deny-all.sh`; installed unwrapped CLI `/nix/store/c14wxbczlzph9l1j5sfy6d4nh1ghdcxz-cargo-octet-0.1.0/bin/cargo-octet` (BLAKE3 `dbf4b36ceacdcc8d372a48498ae3cb513692a22d4bff30774db20fb4e9295f4f`); driver `/nix/store/r5bzbvda2ydnz09c6vxvqhxsmh37nhpw-dylint-driver-5.0.0/bin/dylint-driver` (BLAKE3 `8e14cfcb3f0cfd993c5886e574408e11a49456dbb637b766e3d512e698d4474b`); existing Rust `/nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21`, `rustc 1.96.0-nightly (ac7f9ec7d 2026-03-20)`.

Strict command from the producer directory, with fresh `target/gate-collection-zero`:

```sh
env PATH="$PWD/target/verified-cli/bin:/nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21/bin:/nix/store/y8xj70yyzsml4z3aa5gsn9pvpx4za1py-clang-wrapper-21.1.8/bin:/nix/store/q2vpbpf8vqzizz6836vlw3n83cjr8bma-mold-unwrapped-wrapper-2.40.4/bin:$PATH" \
  LD_LIBRARY_PATH=/nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21/lib \
  RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2 \
  CARGO_TARGET_DIR="$PWD/target/gate-collection-zero" \
  TIGERSTYLE_TOOLCHAIN=nightly-2026-03-21-x86_64-unknown-linux-gnu \
  TIGERSTYLE_DRIVER_PATH=/nix/store/r5bzbvda2ydnz09c6vxvqhxsmh37nhpw-dylint-driver-5.0.0/bin/dylint-driver \
  TIGERSTYLE_LINT_LIB=/home/brittonr/git/octet-worktrees/molten-reviewed-20260925/target/collection-zero-lints/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so \
  OCTET_PRECOMMIT_USE_INSTALLED=true \
  sh /nix/store/9krx6k4dx3fvp5icdhcmznygh2fnkx31-source/hooks/octet-deny-all.sh \
    --artifact-dir target/octet-collection-zero
```

**Exit 2; Cargo exit 101; integration-failure, 263 errors, zero warnings, zero autofixes.** All reported findings are in `molten_node_core`; host compiled without findings; runtime compilation/coverage is not established. Same selected workspace-metadata profile hash `b3:3cfa1bc99d6ed29271c6f9fd2cc5cc00465c56268a65d22ca6a0a7b2c3cd814a` and config hash `b3:58fad8b399c183822f3511c810b20ef9c8f2e8ae7f1f72710cb94f9297be0abc` as the prior 274-finding checkpoint `../standalone-node-ctfe-c1b/`. Findings fell from 274 to 263: bounded-growth reports from seven to two, with the two real dynamic manifest growth paths retained (`content_store_adapter/preflight.rs:78`, `validation.rs:54`); three fallible-conversion plus three sentinel reports disappeared. The 263 still include 97 repeated public path segments, 61 low-assertion-density findings, and 51 exhaustive-enum advisories. None was papered over with fake assertions, wildcard matches, shorter target selection, or severity changes.

The four adjacent raw files are byte-for-byte copies of the installed runner outputs. BLAKE3: `command.txt` `c534e2f190e87592ffd3bc0592f8be7b7462798cc1693889bfe760b7c6cf3693`; `status.json` `6b921f8b7ec2926713bee3599529d52da91d3b0c1e6e5e9ff058808d1851a6c5`; `summary.txt` `1d13ce68c35f6fea89ac204cfc28b809f366ce25a0e96fe02f35ca89ba2f02bd`; `provenance.jsonl` `8a5cbe8496fa4445ebaf50719d4d69ef48ff491a191e7b122909d912dc8714eb`.

The installed Cairn binary `/nix/store/n4di6l534s5ic3q17miwqj1imx6faws6-cairn-0.1.0/bin/cairn` could not parse this producer's older generated policy: `initial workflow profile is missing: outcome-machine`. An older producer-pinned Cairn at `3b4c280b893f2709aebea21fc51a4f9eeba3fe3b` parsed it but predates the canonical `.cairn` layout and falsely saw zero changes. The clean existing Cairn checkout at `15f00875562025e7ea7e0d1f4af24d1a2e2ac06f` has the `.cairn` layout and precedes the outcome-machine profile. Building `cargo build --offline --locked -p cairn-cli --bin cairn` with the installed compiler yielded `/home/brittonr/git/cairn/target/molten-policy-compatible/debug/cairn` (BLAKE3 `97347e9460435b730ed7f4d41b2568a94a38182b6c196e745661830bcab54063`). With that exact binary, `validate --root .` passed over **16 changes and 80 specs**; `gate proposal`, `gate design`, and `gate tasks` for `node-startup-evidence` each returned `verdict: PASS`. The tasks gate still reports **two unchecked tasks**. No policy, task checkbox, or archive was changed to obtain a pass.

`cargo fmt --all -- --check` failed on many pre-existing files outside this slice (including `content_store_adapter/node_service.rs`, `fabric/port.rs`, and `molten-node-runtime` sources). The two edited durability Rust files independently passed pinned `rustfmt --edition 2024 --check`, and `git diff --check` passed. No unrelated formatting churn was applied.

The producer commit remains a local review candidate. The strict gate is denied; no source-to-executable receipt, normal-node start, or VM lifecycle evidence exists. The Molten Cairn remains open.

## Mutable borrowed-container classifier correction (2026-09-25)

The original classifier suggested immutable views for four `&mut Vec<...>`
parameters in selected Molten source. A forwarding helper can grow a mutable
container without a direct `push` in the current function, and `&mut String` or
`&mut PathBuf` cannot generally be replaced with `&str` or `&Path`. The Octet
owner's `borrowed_argument_types` rule now considers only immutable references
and removes the dead direct-vector-push inspection. The UI fixture reproduced
three incorrect suggestions before the change; with the reviewed local library,
it retained exactly four immutable-reference warnings and no mutable warnings.
The local owner commit is `e096774749fbba05bbdeaccdb2021ae143cbd601`;
the published review branch still points to `08f7784bc314620511c466ca948565429c6aba6c`.
The 155 owner library tests and `cargo clippy -p octet --lib -- -D warnings`
passed. The exact owner Nix `ui`, `fmt`, and `nextest` checks also passed;
`nextest` ran 263 tests across six binaries, with 263 passed and five skipped.
The separate workspace-wide Nix `clippy` check remains red on 23 untouched
`octet-architecture-ir` `missing_errors_doc` diagnostics. The scoped offline
Nix library build was preflighted as one Octet derivation without compiler or
Stage0 work. Final library:
`/home/brittonr/git/octet-worktrees/molten-reviewed-20260925/target/borrowed-cutover-result/lib/liboctet.so`,
BLAKE3 `4fb05f09fad3ac857b1a314d38df7ed15dc9cbce6b43ca90407d1d31cd3abc28`.

Using that final library with the **same unmodified hook, CLI, driver, Rust
toolchain, workspace-metadata profile and config hashes** listed above,
`TIGERSTYLE_LINT_LIB` pointed at
`target/borrowed-cutover-lib/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so`
and the producer's `CARGO_TARGET_DIR` was `target/gate-borrowed-review`. The
hook wrote `target/octet-borrowed-final` and exited **2** (Cargo **101**):
**259 errors, zero warnings, zero autofixes**, all in `molten_node_core`.
Compared with 263, the four mutable-borrow false positives disappeared;
the two dynamic manifest growth warnings remain. This is **not** a successful
strict node gate or startup approval. The runner outputs are retained as:

- `borrowed-types-command.txt` — BLAKE3 `8352b73ad516dcfebcd8984b624db4fb9de2cc5b682f349ca686baac2383bf37`
- `borrowed-types-status.json` — BLAKE3 `ff487a9e004b06e9c994d4905a70072ef2dcf61cfd241ee70f03c7c1e50fcc8a`
- `borrowed-types-summary.txt` — BLAKE3 `045f101234f035105a5a4e88aa5ccac217dec3a5d12e9a1a980450d35ba461a1`
- `borrowed-types-provenance.jsonl` — BLAKE3 `c259d871283712b009ff2c1b1509a22ed8b250bd2bfc62dcdc86e09cbe521eef`

At this checkpoint, the producer source matched its `95b8e4ae` review candidate. The
strict gate is denied before runtime compilation, so no `build-inputs.json`
from an independent compiler trace, executable-bound source receipt, or
normal-node VM startup exists. No consumer policy or Cairn checkbox changed.

## Selected-source repair and profile-bound range selection (2026-09-25)

Starting from the producer's local evidence commit `788194c248218b886119dc43bf2cb47feed454c2`
and the owner's local classifier commit `e096774749fbba05bbdeaccdb2021ae143cbd601`,
the producer refactored `validate_build_inputs` into header, unit, required-root,
and bounded-path checks without weakening the exact source-inventory comparison.
Manifest validation now retains the header diagnostics and only the first bad
chunk's diagnostics in a fixed five-slot array. Direct range selection and
both ranged and full command construction reject a manifest above the admitted
profile's `max_chunk_count` before selecting refs. The shared range visitor
lets command construction count selected chunks without allocating or cloning
a discarded vector. No literal cap was invented for a profile-owned bound.

Pinned installed nightly `2026-03-21`, with `RUSTC_WRAPPER=''`,
`RUSTC_WORKSPACE_WRAPPER=''`, `CARGO_NET_OFFLINE=true`, and `CARGO_BUILD_JOBS=2`:
`cargo test --offline --locked -p molten-core --lib` passed **390/390**;
`cargo test --offline --locked -p molten --lib
content_store_adapter::tests::content_command_rejects_manifest_exceeding_profile_for_full_and_range_reads`
passed **1/1** (1361 filtered). The seven touched Rust files passed pinned
`rustfmt --edition 2024 --check`, and `git diff --check` passed. A throwaway
executable linked to the real `cargo build --offline --locked -p molten --lib
-p molten-core` outputs and called the public profile, range, and command APIs.
It exited 0 with
`selected=1 bounded_refs=1 accepted_chunks=1 direct_denied=true range_denied=true full_denied=true`.
Its source and executable were removed after the run. The earlier throwaway
build-inputs smoke also returned
`accepted=true misplaced_root_rejected=true first_invalid_chunk_only=true`.
These are source-behavior checks, **not** an independently traced compiler-input
receipt or a startup run.

The exact unmodified installed hook, CLI, driver, owner library, toolchain,
workspace-metadata profile, and config hashes in the preceding section were
used from the producer worktree. Final invocation:

```sh
env PATH="$PWD/target/verified-cli/bin:/nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21/bin:/nix/store/y8xj70yyzsml4z3aa5gsn9pvpx4za1py-clang-wrapper-21.1.8/bin:/nix/store/q2vpbpf8vqzizz6836vlw3n83cjr8bma-mold-unwrapped-wrapper-2.40.4/bin:$PATH" \
  LD_LIBRARY_PATH=/nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21/lib \
  RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2 \
  CARGO_TARGET_DIR="$PWD/target/gate-borrowed-review" \
  TIGERSTYLE_TOOLCHAIN=nightly-2026-03-21-x86_64-unknown-linux-gnu \
  TIGERSTYLE_DRIVER_PATH=/nix/store/r5bzbvda2ydnz09c6vxvqhxsmh37nhpw-dylint-driver-5.0.0/bin/dylint-driver \
  TIGERSTYLE_LINT_LIB=/home/brittonr/git/octet-worktrees/molten-reviewed-20260925/target/borrowed-cutover-lib/liboctet@nightly-2026-03-21-x86_64-unknown-linux-gnu.so \
  OCTET_PRECOMMIT_USE_INSTALLED=true \
  sh /nix/store/9krx6k4dx3fvp5icdhcmznygh2fnkx31-source/hooks/octet-deny-all.sh \
    --artifact-dir target/octet-profile-bound-final
```

**Exit 2, Cargo exit 101: 261 errors, no warnings or autofixes.** Of those,
260 are in `molten_node_core` and one in `locators`, a test crate compiled in
this particular cached Cargo run; other cached runs included no test crate or
48 `leaves` findings. Compare by crate rather than treating a changed total
as source progress: the 259-finding pre-refactor checkpoint was only
`molten_node_core`. Its two unbounded-growth findings are gone, while
`molten_node_core` now has one new `usize_in_public_api` for a public
selected-count helper and two new `ambiguous_params` from the fixed-array
issue helpers. The remaining policy-denied inventory includes 97 repeated
path segments in `molten_node_core`, 63 low-assertion-density reports, and
51 exhaustive-enum advisories. Adding artificial assertions, renaming
public API, disabling checks, or discarding selected sources would not
establish the requested strict gate.

The four final raw runner outputs are adjacent, with `profile-bound-` prefixes.
BLAKE3, in command/status/summary/provenance order:
`dba588b8ebb571c1bd82b29f9be50ae49629fd65f7b3d905d74f3733654908ca`,
`970e84b0b13d8f8760006d068749e5950750976b8036f6325b4c71e25bcd301c`,
`565da4565b1e9cd6592acf22642189fd604123bf4a0d08f92aeea13a1aeea4e8`,
`a1c5882547fe998817bc05e7b0289f861434547fbb7086c194e87a8f2c0b2356`.
Intermediate `source-refactor-` files retain the first 261-error run,
including an explicit-enum-match fix not yet applied; their four BLAKE3 hashes:
`393ef023d7a2c6987181c2b96ff5496ed2620a9c349bf4caa1489c11a48003c6`,
`970e84b0b13d8f8760006d068749e5950750976b8036f6325b4c71e25bcd301c`,
`ebb4c000c2ef8221caf781029724245eaf8e0d590ebd23bbe70d6aae55c8c2b0`,
`863517a201c435547fcb7cd255f33106a2d7d6b47ea6a1e3ac8f113f3ceb4522`.
The `source-refactor-expanded-` four files retain the later 308-error run
(260 `molten_node_core`, 48 `leaves`); BLAKE3:
`57364cf3a3a9009b0c0686259a0fbb41f2aa56a33d04b5ac3e90a527a0188ce8`,
`66b7bfb41bfb0d37d55755a4f1e3028eca75406ed7dc7e0ef231fe0bdc019845`,
`8488a3d0738a190245d4f279c8bf753070c19bed276dcf228cc18b17f8840cd3`,
`2bd0b9f4ad4b4088ce1069fa70bfcbf7ce11eb1c7837a3f7f429151ad5058111`.

Final source BLAKE3:
`preflight.rs` `bf1f8c34f5947e342d748b643379d6de84ed047c827e3cf48ef7c096e0b49d34`,
`validation.rs` `9bba8153e94fb114c13b8f5ba871f6243a381817e38022b65bbb4e5bc8f99d3e`,
`node_startup.rs` `1bfef9b5746f95c1c9b6ef0eb5e3959307810a9433ba61b2905f0058dd63b117`,
`src/content_store_adapter/integration.rs`
`60305a1a44da3385d229b2f5d9e79382337ab7b6db2a2a800f464069586d39d5`.
The three touched test files' hashes are retained in the verification
transcript. Strict denial precedes compiler-dependency collection and VM
startup; neither task checkbox, consumer approval, nor Cairn archive moves.
