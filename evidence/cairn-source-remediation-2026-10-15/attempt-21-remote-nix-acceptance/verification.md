# Attempt 21: remote Nix acceptance after local daemon lock

## Outcome

The required Nix checks for F09/F10 passed on a remote builder:

```sh
nix build --no-update-lock-file --no-link -L \
  .#checks.x86_64-linux.nextest \
  .#checks.x86_64-linux.clippy \
  .#checks.x86_64-linux.nextest-config
```

- Exit code: `0`
- Elapsed: `905.2s` (2026-10-19T10:43:00-04:00 to 10:58:30-04:00)
- `nextest`: 1,741 tests run, 1,741 passed, 0 skipped; CI receipt
  `blake3:6371e2745a98f6f67577b6beff0019caf23b9759b77b0bfc2c38c9ee5b858540`
  with decision `pass`.
- `clippy`: passed as part of the same build closure.
- `nextest-config`: passed as part of the same build closure.
- Outputs:
  - `/nix/store/pxr26xvx3a53bwi4mh1cycdi8w1kv1b2-molten-nextest`
  - `/nix/store/djsbqa59mgdzxvw8y374vfsp7100qzrl-all-workspace-members-clippy`
  - `/nix/store/66fa7qhhw555ch9k2plfy105myld3syb-molten-nextest-config-check`

The repository flake evaluation passed:

```sh
nix flake check --no-build --offline --no-update-lock-file -L
```

- Exit code: `0`
- Result: `all checks passed!`
- Scope: evaluation only; this is not full flake-gate build acceptance.

The full `nix flake check` build did not complete in the bounded session. It
reached a cold remote closure (LLVM, GCC, QEMU, OpenBLAS, NixOS VM test inputs)
and was stopped after the remote builder had spent the session building that
closure. No source-check failure was observed before the stop; the run is not
passing gate evidence.

## Why the local host was not used

The local Nix client was blocked waiting on the daemon socket. A root-owned
`nix-store --add-root` process (PID 2300007) had been alive for roughly 20 hours
with zero user/system CPU time. The local client could not make progress and
noninteractive sudo was unavailable. The remote `aspen1` builder was used with
the same source archive and the same check attributes.

## Source identity

- Local HEAD: `e2db8c1eafbe5150926b2ff30461a936e4eea636`
- Source archive SHA-256:
  `7049e9f19856d56355ba63f6058ba9edd853eae466e7ac68257432011c40abbb`
- Remote host: `aspen1`, Linux 7.2.4, Nix 2.36.0,
  system features include `kvm` and `nixos-test`.

Private flake/cargo Git inputs were supplied from the local Nix Git cache using
an explicit `GIT_CONFIG_GLOBAL` URL rewrite and `--offline`. This is a
development-time input transport substitution; it is not release evidence and
does not change the check derivations.

## Non-claims

- The three required Nix checks passed; the full repository flake gate did not
  complete.
- This record does not establish F09/F10 sync/archive, release readiness,
  whole-core Octet warning-debt removal, or production correctness.
- The local host daemon/worker condition remains an operator-owned prerequisite.
