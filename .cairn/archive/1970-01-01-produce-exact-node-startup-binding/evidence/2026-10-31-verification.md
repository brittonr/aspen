# Verification: exact node startup binding producer (2026-10-31)

Integration worktree `/home/brittonr/.cargo-target/aspen-open-cairns-20261029`, branch
`cairn/finish-open-changes-20261029`. Only commands listed here were run; results are copied
from their output. Authority used: the explicitly experimental key
`molten-experimental-startup-20261030:n9TuYdOAjKyPUs7+zoSGmQ/OtYQFcXQQEWxuW1Udtkw=`, generated
for this experiment and selected independently of any evidence. It is not production operator
approval.

## Real producer runs

Each output was built with
`nix build --builders '' -L --out-link <root> path:<clean checkout>#packages.x86_64-linux.molten-startup-source-binding`
from a clean detached checkout of the named commit (no untracked files), so the bound source is
exactly that commit's tree. Out-links keep GC roots (an earlier run's outputs were deleted by
store garbage collection before they could be used).

| Commit | Output | Octet gate (in the build) | `source_ref` | `executable_ref` |
|---|---|---|---|---|
| `c267d062c` | `/nix/store/bic1xc7d7iih5340vqgsjqsmsmrvhwm6-molten-node-source-binding-v1` | `octet gate pass receipt=blake3:8f2fc039…22bb07b` | `blake3:4885db81…e968f82b` | `blake3:445b3caa…ae22364a` |
| `ea61f0463` | `/nix/store/siwk1z4sjf45qj2w9zf2pbwxhr9zk47p-molten-node-source-binding-v1` | `octet gate pass receipt=blake3:5672b0cd…34bd60bcaf` | `blake3:7a02179c…5b81dc6c06` | `blake3:9c0eb011…ccdf1963d` |

Both bind `producer_profile` `molten-nix-source-binding-v1`, schema
`molten-node-source-binding-v1`, the same policy (`blake3:e045c1f5…bce394c`), profile
(`blake3:01176770…720f66b`) and toolchain (`blake3:a79ba646…76a2b32`), four retained cohort
artifacts (primary, library, gate receipt, remediation receipt) and a 21,043-path measured
build closure. The `ea61f0463` build also ran the three recovery fault variants (patched copies
of the same tree), each with its own passing gate.

The first attempt at `740be6a53` failed before any measurement: the private release-channel
Cargo source was fetched with `pkgs.fetchgit` inside the sandbox, which has no ssh
(`cannot run ssh`). Commit `567be00a0` maps that exact revision to the client-fetched flake
input, and every later build used it.

## Independent signing and trust selection

`nix store sign --key-file <experimental secret> <output>` (via
`molten-startup-smoke-20261030/run.sh`). The outputs also carry this host's automatic
`britton-desktop-1` signature; startup does not select it. The trust record selects one full
key and the measured `binding.json`, `policy` and `profile` identities:

| Commit | `binding_ref` | trust record sha256 |
|---|---|---|
| `c267d062c` | `blake3:0aa989be1d3c1d02908d54998810267b078ca99179eafcbed5f3047849287e7c` | `4d91aeaf46784b387d6a2314b310b83fc90ce6fd14263eeece5f87058a654043` |
| `ea61f0463` | `blake3:61ad165fe3224f04c94778b916224c834e4bf6a2019538f4e49b53451268affa` | `e40b43703fece42ed4b80ca4da75887c6122699537f0b457ebf78705de7be8ae` |

## Normal-startup positive and negatives

`scripts/smoke-startup-source-binding.py` through `run.sh`, once per producer output. Both runs
printed the same outcome:

```
real-normal-startup negative=missing denied exit=1 lifecycle=unchanged
real-normal-startup negative=stale denied exit=1 lifecycle=unchanged
real-normal-startup negative=wrong-candidate denied exit=1 lifecycle=unchanged
real-normal-startup negative=tamper denied exit=1 lifecycle=unchanged
real-normal-startup negative=missing-gate-closure denied exit=1 lifecycle=unchanged
real-normal-startup negative=synthetic denied exit=1 lifecycle=unchanged
real-normal-startup negative=read-error denied exit=1 lifecycle=unchanged
real-normal-startup positive=passed authority=explicitly-experimental production-approval=none
```

The positive runs `node init`, `node run` and `node stop` with the signed output's own
`bin/molten` and checks that the startup receipt binds the accepted `binding_ref`. Every
negative compares all lifecycle bytes in the node root (lock, receipts, evidence) before and
after the denied start. The tampered and incomplete-closure controls are input-addressed copies
of the real output, signed by the same experimental key. Logs: `smoke-c267.log`,
`smoke-ea61.log` under `/home/brittonr/.cargo-target/molten-producer-builds` and
`/home/brittonr/.cargo-target/molten-startup-smoke-20261030`.

Two further controls on the `c267d062c` output (throwaway `extra-negatives.sh`):

```
real-normal-startup negative=unsigned denied exit=1 lifecycle=unchanged reason=error: invalid harness artifact: source output has no signature by the independently selected key
real-normal-startup negative=wrong-key denied exit=1 lifecycle=unchanged reason=error: invalid harness artifact: source output has no signature by the independently selected key
```

`unsigned` is a fresh copy of the output that carries only the host's automatic local-build
signature; `wrong-key` selects a freshly generated key that signed nothing.

Unsupported versions and fixture profiles are refused by the pure admission
(`molten_core::startup_binding::tests::historical_and_fixture_producer_profiles_are_not_promoted`),
and unauthenticated output by
`admission_requires_authenticated_exact_executing_candidate`; both ran in the workspace suite
below.

## Repository checks on `c267d062c`

| Command | Result |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` (default, `spacewasm-pilot`, `component-rpc-telemetry`) | each exit 0 |
| `cargo octet check` (primary) and `-p molten -- --lib` | `Status: clean`, `Findings: 0` each |
| `cargo test --workspace --no-fail-fast` | 2911 passed, 0 failed, 28 ignored |

The 28 ignored tests need independently provisioned signed outputs or external cohorts; the
ones run explicitly are listed below.

## Signed rails on the producer output

`MOLTEN_TEST_SOURCE_TRUST` selected the `ea61f0463` output:

| Suite | Result |
|---|---|
| `cargo test -p molten --test outside_in -- --ignored` | 9 passed, 0 failed (4102 s) |
| `cargo test -p molten --test cliharness -- --ignored` (10 non-cluster tests) | 10 passed |
| the 3 signed cluster-harness tests, after `MAX_CLUSTER_CHILD_TIMEOUT_MS` rose to 30 min | 3 passed (2034 s) |
| `cargo test -p molten --test nativesystemextension recovery::live_public_node_composition -- --ignored --exact` | 1 passed (21,983 s) |
| `cargo test -p molten --test nativesystemextension recovery::live_public_node_broken_variants_fail_shared_properties -- --ignored --exact` | 1 passed (22,197 s) |

The first signed cluster-harness attempt failed with `local-executable-child-timeout`: the old
300 s per-child ceiling did not admit a signed start that re-measures a 21,043-path build
closure on a loaded host. The ceiling is now 30 minutes, matching the outside-in scenario
ceiling, with regression test
`cluster_harness::tests::child_timeout_ceiling_admits_signed_startup_and_still_bounds_a_child`.

## Traceability

The inherited-tracey-debt guard first failed on two accepted requirements with no source reference (`molten.audit_f02.real_evidence`, `molten.startup_binding.refresh`); implementation and verification markers were added at the admission, loader and fixture-isolation sites, after which the guard reported `verdict=pass`, `dangling=0` (`evidence/2026-10-31-closeout/tracey.log`).
