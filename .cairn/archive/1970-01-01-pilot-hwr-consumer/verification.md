# Molten external runtime pilot evidence — 2026-10-26

## Identity, ownership and scope

The isolated pilot depends on immutable producer candidate
`ce9b222be993c43e9960ce7c12aed2a5130ecf76`, repository
`rad:z3hRCegTsS8jpJVgxYfb9psEzxHpG`, publisher
`z6MksnXbFoE8zkCkGWhHc8zuxpnEUhrJHv2KECRV4GSv9gkx`.
Both manifest dependencies, the generated pilot lockfile and `source.ncl` select
that source. No Cargo path patch is part of the run or delivered dependency.
The producer's ordinary Radicle adapter advertises canonical main only; its
publisher-qualified adapter also advertises the candidate branch. Initial runs
against the ordinary adapter could not resolve the revision; the explicit
publisher resolved it and the command executed successfully. The push reported
no seeds, so network replication is not established.

Molten Wasm component runtime maintainers own this standalone actor fixture,
policy, WIT identity, epoch driver, exact output assertions and repeatable
command. The producer owns mechanism admission and bounded guest exchange.

## Actual command and results

Run from the Molten repository, using its Nix shell:

```sh
CARGO_NET_GIT_FETCH_WITH_CLI=true nix develop --builders '' -c cargo run --locked --manifest-path pilots/hwr-consumer/Cargo.toml
```

The first successful run updated only the pilot dependency resolution before
executing. Observed output:

```text
molten-hwr-consumer world=molten:component-runtime@1.0.0/actor export=actor-turn class=Completed actor=7 counter=42 output=070000002a000000 bytes=8
fuel_remaining=9973 fuel_budget=10000
denials malformed=GuestDenied oversized=InputLimitExceeded missing-epoch=DeadlineDenied
scope=pilot-only production-adoption=false materialization-trust=false authority=false
```

The embedded WAT was compiled as a real component; its binary header, exact
WIT BLAKE3 identity, preserved actor id and incremented counter were asserted.
The consumer stopped and joined its own epoch thread before outcome assertions.
Producer evidence records its clean deny-all gate, 60 tests and successful
x86_64-linux hermetic checks; those are not Molten production-runtime gates.

## Review and non-claims

Integration-owner source review checked that this workspace is isolated from the
root Molten dependency graph, binds no product imports or WASI, rejects an absent
engine-bound epoch service, and preserves the three explicit false authority
and adoption fields. This is an implementation review, not an independent
approval receipt.

The immediate outcome is the exact actor turn and denials above. The durable
capability is a repeatable consumer-owned component boundary exercise. No
production cutover/parity, Mantle trust, canonical Preserves receipt, actor
persistence/effects/replay, host isolation, performance or release claim follows.

## Completed focused source and lifecycle checks

After configuring the standalone workspace's existing Octet metadata convention,
the first source run reported non-trait imports, panicking setup failures and a
compound assertion. Qualification, explicit error propagation, separate fuel
assertions and extraction of the materialization-facts phase removed all findings
without disabling lints or changing runtime behavior.

From `pilots/hwr-consumer`, inside `nix develop ../..`, final `cargo fmt`,
`cargo clippy --locked --all-targets -- -D warnings`, `cargo run --locked` and
`cargo octet check --artifact-dir target/octet` all completed successfully.
The final locked command reproduced the exact output and denials above. Octet
reported **clean, zero findings**, using producer toolchain
`nightly-2026-03-21-x86_64-unknown-linux-gnu`, profile `workspace-metadata`.
This is pilot source evidence, not a root Molten source gate or release receipt.

Native strict Cairn validation with the sibling provider's explicit policy
passed without findings before and after synchronization and archival. The
final receipt is
`906e57a531af7ae0fdaeacf1affb105ff57f82d624cacb2b5567e0e89ded84a9`.
The inherited Tracey guard passed with 2,966 requirements, 1,032 referenced,
1,934 uncovered and zero dangling references. The inherited debt list and its
BLAKE3 identity are unchanged; only observed metadata counts are refreshed.
These checks establish structural linkage, not marker truth or release readiness.
The compiler cache reported disk-I/O trouble and disabled caching; compilation
and execution completed. No cache-health claim follows.

The targeted hermetic `inherited-tracey-debt` Nix check passed: nine guard tests,
four classifier tests, exact unchanged classification, Nickel metadata export
readback and baseline BLAKE3 verification all completed. Separately,
`nickel export pilots/hwr-consumer/profile.ncl --format json` matched the wire
`profile.json` byte-for-byte, and `nickel typecheck pilots/hwr-consumer/source.ncl`
completed. These are the applicable pilot/configuration checks, not a full-root
Molten build or production-runtime adoption gate.

## Failed commit-hook attempt retained

The first commit attempt passed whitespace and root formatting, then root
Octet failed during dependency compilation with `No space left on device
(os error 28)` under `/home/brittonr/.cache/aspen-verify-20261023`. Its summary
was `integration-failure`, twelve `compiler_error` findings; this was not a
successful root source gate or a commit. Both root and `/tmp` storage were full.
Only this runtime-boundary task's completed compilation caches were removed;
the root verification cache and its failed gate observations were retained.
No hook, lint or source gate was disabled.

After clearing only the completed task caches, the retry created commit
`93b245a26`. Whitespace, root formatting and the required root Octet hook passed.
The root `status.json` reported `clean`, zero total/warning/error findings and
a successful Cargo process, with configuration identity
`b3:2b46918cdad7431d09fa6fc374bcf94bc3d51eb0a23f7a278f6c40b59298fa64`
and profile identity
`b3:a37946124e9477919893b99b205e4d5e58ebdf5ca18550c59c61cf94905221ae`.
This successful retry does not erase the failed storage observation above.
