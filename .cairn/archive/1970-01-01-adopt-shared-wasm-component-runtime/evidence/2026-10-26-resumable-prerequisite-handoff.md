# Resumable producer prerequisite handoff — 2026-10-26

## Disposition

The two missing producer API surfaces recorded in the [compatibility preflight](2026-10-26-compatibility-preflight.md) are now published and independently executable. The adoption change is no longer held on an absent resumable invocation or absent typed phase. This supersedes only that prerequisite blocker, not the preflight's recorded observations or the requirement for complete local/shared parity before cutover.

The product runtime, root dependencies/lock, generated plans, profiles, actor/extension paths, effects and canonical receipt cohort remain unchanged. No adoption implementation task is closed by this handoff. Production adoption is not complete; do not sync or archive this change on producer evidence alone.

Immediate outcome: a completed immutable producer can be evaluated by the consumer. Durable contribution: independently fetched public-API evidence plus an exact adoption input, without a product-local fork or information-losing fallback. Hardened-runtime maintainers own the execution mechanism; Molten component-runtime maintainers own the adapter, frozen corpus, materialization boundary and product acceptance.

## Exact producer identities

- Exercised implementation: `b84bb2970bedda50ef63366e120e06d796df92f3`.
- Completed archive publication: `6381322a68407da71325c4c33113d11ada9af67f`.
- Source: `rad://z3hRCegTsS8jpJVgxYfb9psEzxHpG/z6MksnXbFoE8zkCkGWhHc8zuxpnEUhrJHv2KECRV4GSv9gkx`.
- Contract: `hwr.shell.v3`; Wasmtime remains exactly `45.0.0`.
- Native producer archive: `.cairn/archive/1970-01-01-add-resumable-component-invocation/`, in the producer repository. The archive's date prefix is the native tool's output, not the execution date.

Both implementation and archive commits were pushed to the canonical Radicle `refs/heads/main`; an explicit reference query returned the complete archive revision. Push output also reported `No seeds found`. No network replication or GitHub availability is claimed.

The producer's archive records passing 8 core unit, 29 admission and 36 actual execution cases, strict Clippy, formatting, deny-all pinned Octet with zero findings, strict native Cairn gates and host Nix build/tests/Octet checks. Those are producer-owned checks, not a rerun of Molten's complete parity corpus. Other Nix systems were explicitly omitted.

## Independently fetched public API execution

A disposable external Cargo manifest pinned **both** producer crates to the complete archive revision above, using Git dependencies and CLI Git fetching, without source-path patches. Cargo fetched the published Radicle ref and compiled the two crates at `6381322a`.

The actual-component probe invoked each guest synchronously and through `Arc<ComponentInvocation>::start_sliced` / `ResumableInvocation::run_slice`, comparing whole terminal outcomes for these cases, checking every yielded ordinal/configured quantum and refusing a second completion. Observed output:

```text
schema=hwr.shell.v3 case=continued yields=20 class=Completed phase=Invocation output=[79, 75] fuel_remaining=98584
schema=hwr.shell.v3 case=start-trap yields=0 class=GuestTrapped phase=Instantiation output=[] fuel_remaining=99999
schema=hwr.shell.v3 case=export-trap yields=0 class=GuestTrapped phase=Invocation output=[] fuel_remaining=99997
scope=upstream-api-smoke product-adoption=false authority=false
```

The success case uses one retained store/future through a finite loop, 64 configured fuel per quantum and a 100000 total budget. Output is `OK`; 20 pending polls do not imply exactly 20 × 64 consumed fuel. Both traps remain `GuestTrapped` while their typed phases differ. This specifically addresses the earlier missing-API counterexample, not universal receipt/resource equivalence.

## Remaining consumer acceptance

The source-pin task remains open: neither the root runtime nor the Mantle verifier dependency has been migrated. The existing independently pinned pilot is not the production runtime. The parity/mapping task remains open, including the producer's recorded failed-run memory/fuel difference, aggregate memory/transfer semantics, staged receipts, scheduling and cancellation. Pending consumer implementation must retain exact configured quantum versus measured terminal fuel meaning.

Molten still owns canonical Preserves and BLAKE3 identities, current policy/Basalt/UCAN and generation/replay/resource admission, the new closed effect-request world, actual actor and system-extension composition, coupled cutover/rollback and product receipts. The prior materialization schema and filesystem-authority analysis remains applicable; structural `Verified` is not byte verification or permission.

This handoff is an investigation/evidence update. Under `docs/proof-workflow.md` it uses the non-executable-change exemption: no permanent tests or product behavior were added, and producer/pilot execution cannot discharge the open implementation tasks. It does not prove Wasmtime/guest correctness, isolation, authority, source trust, whole-system safety or release eligibility.

## Independently exercised unchanged Molten actor pilot

From Molten baseline `380a7b16385ee1cdce2f6bd0dd3654cc9c620dfa`, the existing standalone `pilots/hwr-consumer/` sources, profile, WIT, guest and lock were copied to a disposable workspace. Only the two producer revisions in its copied manifest changed from `ce9b222be993c43e9960ce7c12aed2a5130ecf76` to `6381322a68407da71325c4c33113d11ada9af67f`; Cargo resolved the two producer packages without refreshing other locked package versions. No tracked pilot file changed.

Executed from the isolated Molten worktree inside its Nix shell, with executable task-owned scratch directories, debug information disabled and two build jobs:

```sh
CARGO_NET_GIT_FETCH_WITH_CLI=true nix develop --builders '' -c cargo run \
  --manifest-path /dev/shm/molten-hwr-published-probe-20261026/Cargo.toml
```

Exit 0 under the consumer's Rust `1.98.0-nightly` cohort. Exact output:

```text
molten-hwr-consumer world=molten:component-runtime@1.0.0/actor export=actor-turn class=Completed actor=7 counter=42 output=070000002a000000 bytes=8
fuel_remaining=9973 fuel_budget=10000
denials malformed=GuestDenied oversized=InputLimitExceeded missing-epoch=DeadlineDenied
scope=pilot-only production-adoption=false materialization-trust=false authority=false
```

The output, fuel and deny observations match the prior pilot execution recorded in the preflight. This is the same pilot at a newly fetched immutable cohort, not a full local/shared product parity comparison or production materialization execution. The consumer-owned epoch driver was joined. Compiler-cache disk-I/O warnings disabled caching, not compilation or execution.

BLAKE3 identities of the disposable actor probe inputs:

```text
3d48af48514fffcef8b11183b817af45fadcd1906642cd9b59107462e91d20ab  src/main.rs
c97a653c8385900e3078d36726a348e21122f6a5ae182b96069bb289e0719055  Cargo.lock
14b274fec58b3dd4600b7889db58cdd12f76f2bdb44e8b60a21b6d9cb21a52c8  profile.json
a899d8eaa34cce4aba12b378586f5fb744451a011340f0ce84ac9fbc10f03ab9  guest/actor.wat
e6b69908a1dcba063fd438d2cf44776ee5f7d7b7919f14aeefce766e72bfd82c  wit/actor.wit
```

The independently pinned continuation/phase probe source BLAKE3 was `96eccb96fa7104cbb8fa2508a2f95cdf034b1038d55839fcb263a25511591817`; its resolved lock BLAKE3 was `c08be9dc74f1792f88499373c0688c744957f7080eb89285aa7d7fb59d20f5ae`. Disposable sources are not installed product paths.

## Handoff document checks

Strict native Cairn validation through the sibling provider and its explicit policy passed with `valid: true`, empty findings/issues and policy BLAKE3 `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`. The affected task/evidence documents had four relative links and no broken target. These checks validate document/package structure, not adoption completeness.
