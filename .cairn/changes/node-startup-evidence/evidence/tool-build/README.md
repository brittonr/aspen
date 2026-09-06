# Approved pinned Octet build

The operator approved building only the pinned Octet CLI and lint library with the existing March-21 compiler.
The build completed. Tool availability is resolved, but Molten startup approval remains unresolved.

## Build scope and result

The unchanged pin is `fc38f59330b626961d166febfdf1a5aa6575460f`.
The complete source and filtered source identities remain those recorded in `../tool-discovery/README.md`.

The dry run listed ten derivations: the two requested outputs and eight source/vendor/dependency-cache steps.
The actual build used that plan, offline, without substitutes or remote builders.
No Rust compiler, driver, Verus toolchain, Mantle, Darkhttpd, or package toolchain was rebuilt.
No default installed Octet wrapper was replaced.

The build finished with exit zero in **2 minutes 10.689 seconds**.
Its Nix-owned outputs are:

| Component | Output |
| --- | --- |
| CLI package | `hxd7wylwz756id11hksv9h5ywjjs66f2-cargo-octet-0.1.0` |
| Lint library | `d5kynl0p00wailfm1swkm3rbc7bm7hg8-octet-0.1.0` |

The upstream CLI package also emits its existing `cargo-slotcar` and `cargo-tigerstyle` compatibility executables.
The selected binary digests are:

- `bin/cargo-octet`: `cd4db81a8cf74be49b86c0ae2edab17963679652289f21eca4426fd64da3468a`.
- `lib/liboctet.so`: `10ea7d049202ebdf12e621c33ca26269a8e78ea4e9bc77186fad2e8fe42c366e`.

The build log records both real source compilations and their expected derivations.
Crane's preliminary dummy-source checks only prepare dependency artifacts. They are not Octet or Molten gate evidence.
The CLI build reported 15 existing dead-code warnings. This is not warning-free Octet self-acceptance.
Nix `store verify --no-trust` completed for both outputs. That operation verifies content, not signatures or authority.

Nix enforced one build at a time, two cores, a 900-second per-build timeout, and a 300-second silence timeout.
The user systemd unit bounded the Nix client to 1800 seconds. Its resource counters do not include daemon-owned compiler processes.

## Real smoke observations

The smoke tests used the new CLI and library, the existing compiler and driver, and the exact pinned `octet-deny-all.sh` hook.
All-target and all-feature flags were enabled in fresh dependency-free fixtures. No source suppression or warning baseline was added.

| Case | Observed result | Meaning |
| --- | --- | --- |
| Empty library, prepared lockfile | Exit 0, zero findings | Selected positive case passed. |
| Bare `todo!()` | Exit 0, zero findings | False-clean counterexample. Not accepted as a passing negative test. |
| `todo!("implement later")` | Exit 2, two `no_todo` errors | The library loads and rejects the message-bearing case. |
| Missing library | Exit 101, explicit load failure | The direct driver rejects a missing plugin. |

The two message-bearing findings come from the library and library-test targets.
This pinned CLI labels their Cargo failure `integration-failure`, despite the README's documented lint-error exit code of one.
The initial smoke harness expected one and failed. The final observation asserts two plus actual `no_todo` errors, not arbitrary nonzero exit.

The first positive attempt lacked `Cargo.lock`. Octet reported clean lint counts but exited two after rejecting the lockfile mutation.
The next attempts prepared lockfiles before execution. All failed attempts remain private and have retained log digests.
No clean-count summary alone was treated as success.

## Bare-todo coverage gap

The bare and message-bearing source files are retained here as distinct fixtures. Neither is production configuration.
The bare case remains a failed negative observation. The message-bearing case does not replace it.

Pinned `src/safety/no_todo.rs::check_expr` returns unless an expanded expression has `ExprKind::Block`.
The existing March-21 compiler's HIR shows a call for the bare macro, but an extra expanded block for the message-bearing macro.
The surrounding function block is not the macro expansion. This explains the observed difference in this bounded reproducer.
The pinned upstream `ui_tests/no_todo.rs` uses the message-bearing form.

This checkpoint does not patch the linter, change the pin, or declare general lint soundness.
It does not approve these tools as sufficient evidence for normal startup. The coverage defect needs explicit review.

## Remaining work and preserved boundaries

Real strict execution on the complete Molten source, source-to-runtime-binary association, and normal lifecycle admission remain open.
Both producer startup guards and the consumer VM guard remain unchanged.
No Molten gate, normal-service VM, native replay, physical deployment, or release/default promotion occurred in this build step.
No Stage0 or replacement package/toolchain path was used.

Evidence review was single-agent and correlated. No compiler correctness, reproducibility, independent signature verification, or full workspace acceptance is claimed.
The active lifecycle tasks remain open. No sync, archive, or primary-branch integration occurred.

Public files retain build observations, fixture inputs, and selected raw Octet status outputs.
Logs containing private paths remain private and are bound by `private-log-digests.txt`.
The status files are fixture observations, never startup receipts or approved cohorts.
