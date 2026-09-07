# Workspace dependency registration: confirmed reproduction

This checkpoint diagnoses the failed source gate at commit `6667e5616b978f7bb6f64201b76d452634815612`.
It does not repair the tool or approve startup.

## Goal and boundary

The goal was to distinguish a dependency registration defect from a missing library, compiler incompatibility, or Molten source issue.
The budget was three workspace probes and three direct-driver probes, using existing tools only.
All six probes ran. No tool, compiler, package, or production binary was rebuilt.
The review was single-agent and correlated.

Tool identities remain the prior exact set:

- CLI: `dbf4b36ceacdcc8d372a48498ae3cb513692a22d4bff30774db20fb4e9295f4f`.
- Library: `10919bdd10b0049c1b1113f9757dc563441c6709b91efb02323ff5d32489447b`.
- Driver: `0c465775ce14d8394840fc6c1389a7e830c9eaf92c8bcea4492cba36f7cd5b24`.
- March-21 rustc: `5891756528d0229025735e63e32e2d57e7b49ad5f6fdac09592d623218d3bd8b`.
- Deny-all hook: `41603febce44635585c6010557f6991008976b7e2f7e417c9c78f5299690522f`.

## Workspace differential

The fixture has two empty library crates. `registration-app` depends on workspace member `registration-core`.
Workspace metadata selects only the app and specifies `--all-targets`.
Each case uses a fresh target directory, offline Cargo, an isolated environment, and the unchanged all-deny hook.
Lockfile generation precedes checking. Source and tool hashes match before and after the three cases.

| Fixture selection | Octet exit | Status | Findings |
| --- | --- | --- | --- |
| App primary; core dependency | 2 | integration-failure | 68 errors, 1 warning |
| Both primary (`--workspace`) | 0 | clean | 0 |
| Core primary (`-p registration-core`) | 0 | clean | 0 |

The failing case has the same 68 E0602 unknown-lint errors as Molten's actual gate. Cargo exits 101.
The controls change fixture scope only. They are not alternate production commands or accepted scope changes.
Pueue task 10710 completed all three cases in 724 ms.

## Direct-driver differential

The direct probes use the same installed driver and library, with an explicit existing sysroot.
They set `DYLINT_NO_DEPS=1` and diagnostic flags `-D unknown_lints -D no_todo`.
This reduced flag set isolates the defect; it is not a production gate.
The first two probes differ only in `CARGO_PRIMARY_PACKAGE` and their output location.

| Source | Primary variable | Driver exit | Diagnostic |
| --- | --- | --- | --- |
| Empty | absent | 101 | E0602: unknown lint `no_todo` |
| Empty | `1` | 0 | none |
| Bare `todo!()` | `1` | 101 | `todo!()` marks unfinished production code |

Pueue task 10713 completed in 118 ms.
The third probe proves that the primary path actually runs the fixed lint, rather than merely ignoring lint flags.

## Recorded driver source

The installed driver records derivation `z4rxlp80ki1m9snl5x5wcbvffiy13s28-dylint-driver-5.0.0.drv`.
Its `src` is `/nix/store/01rfnmc9vcnlvn05iw98j86mmnbl66kd-dylint-driver-src`.
The derivation record has no patches and names the existing March-21 toolchain.
The recorded `driver/src/lib.rs` has BLAKE3 `1fdb3cd154e254fe73d823129014e6c731a58ee1017d0b29126fae238886d7c8`.
These are recorded build inputs, not independent reproducibility or signature evidence.
A matching file does not equate the complete recorded source with Octet's separate test-support source tree.

In that exact source, `driver/src/lib.rs`:

- Lines 303–305 read `DYLINT_NO_DEPS` and `CARGO_PRIMARY_PACKAGE`.
- Lines 320–321 skip registration when no-deps is enabled and the package is not primary.
- Lines 432–435 collect `DYLINT_RUSTFLAGS` without that condition.
- Lines 482–486 append those flags to the compiler arguments.

Pinned Octet `cargo-octet/src/engine.rs` installs the workspace wrapper and sets `DYLINT_NO_DEPS=1`.
The hook supplies plugin deny flags plus unknown-lint denial.
Together, these paths send plugin lint names to a dependency invocation that intentionally does not register those names.
The live differential confirms this interaction with the installed binary.

Missing-library and compiler-incompatibility explanations do not explain the successful primary controls.
Molten-specific source is unnecessary: empty workspace members reproduce the failure.

## Proposed repair boundary

Align plugin-flag injection with the driver's no-deps registration predicate.
Preserve all deny flags and real lint checks for primary packages. Do not remove unknown-lint denial globally.
Do not widen Molten's scope, alter its cohort policy, or introduce a startup bypass.
Octet's `flake.nix` owns `dylintDriverSrc` and `dylintDriver`, the packaging boundary for a reviewed upstream-source patch.
A repair must test dependency and primary paths, bare-todo rejection, and the unchanged production command.

No repair or replacement driver was built. Prior build approval covered the CLI and lint library, not a new driver.
Driver-only repair/build approval is required before proceeding, using the existing compiler and an audited build plan.
No Stage0, compiler build, Mantle/Darkhttpd rebuild, VM, native replay, or startup authority is claimed.
Tasks 4 and 5 remain open. Historical actual-source-gate failure evidence remains unchanged.
