# Shared runtime adoption compatibility preflight — 2026-10-26

## Disposition and ownership

**Cutover is blocked.** The completed synchronous producer boundary is available,
but its published API cannot preserve Molten's admitted fuel-sliced execution and
failure-stage receipt contract. The existing local synchronous and time-sliced
paths, product dependencies, profiles, consumer wiring and receipt cohort remain
unchanged. This is a prerequisite investigation, not partial runtime adoption.

The immediate outcome is an executable counterexample to an information-losing
adapter. The durable contribution is a frozen source inventory and explicit
upstream handoff, avoiding a second product-local Wasmtime implementation or a
silent loss of scheduling behavior. Molten's component-runtime maintainers own
consumer parity and admission; hardened-runtime maintainers own the missing
product-neutral execution API. Mantle maintainers own producer bundle identity
and verification. Producer success grants none of Molten's product authority.

## Observed source identities

- Molten baseline: `d2f21b1b4c90163de6034d37b0cbd03e820adced`.
- Completed hardened-runtime archive publication:
  `0455e0958e6f8948d0df5f62950be39519d717a0`.
- Exact independently fetched implementation exercised by Molten's locked pilot
  and the disposable compatibility probe:
  `ce9b222be993c43e9960ce7c12aed2a5130ecf76` at
  `rad://z3hRCegTsS8jpJVgxYfb9psEzxHpG/z6MksnXbFoE8zkCkGWhHc8zuxpnEUhrJHv2KECRV4GSv9gkx`.
  The producer archive names this exact implementation. The remote fetch also
  advertised canonical `main` at `0455e09`; network replication is not claimed.
- Mantle publication evidence names full revision
  `6d58dea6e65b36d8052ad5bf7bd540edda92cc88`, resolved from the archive's
  `6d58dea6e` identifier. Its archived producer checks are reported evidence,
  not checks rerun in this investigation. A new Molten production dependency on
  that revision has **not** been added or independently fetched here.

The producer lifecycle sources are
`hardened-wasmtime-runtime/.cairn/archive/1970-01-01-complete-runtime-extraction-boundary/{tasks,verification}.md`
and
`mantle/.cairn/archive/2026-09-10-publish-wasm-component-consumer-verifier/{tasks.md,evidence/consumer-verifier-validation.md}`.
These repository names locate evidence owners; sibling paths are not product
runtime dependency configuration.

## Blocking execution contracts

### Fuel-based suspension and continuation

Molten's existing `src/wasm/component/runtime/sliced.rs` configures
`fuel_async_yield_interval`, holds one suspended invocation future, and resumes
that same future one poll at a time. `runtime/slicedshell.rs` records quantum
boundaries, returns a generation-fenced scheduler requeue command, and publishes
the completed turn once. Existing behavioral cases in
`tests/slicing/yielding.rs` cover sibling progress, budget exhaustion, exact
same-work fuel consumption, continuation replay and no duplicate publication.

The published shell exposes `ComponentInvocation::prepare` and synchronous
`ComponentInvocation::run`; its engine and store construction are private. It
exposes no suspended invocation, fuel quantum, poll/resume operation, or slice
observation. This is host-driven yielding of a synchronous WIT export, **not** a
request to enable the guest Component Model async proposal. The latter remains
denied in both cohorts and is not a substitute.

Running the component synchronously, restarting it after a fuel trap, retaining a
second local engine, or replacing work quanta with epoch deadlines would change
observable behavior or retain duplicate product-neutral mechanism. None satisfies
`molten.wasm_component_adoption.parity`, `.cutover` and `.rollback`.

Required upstream handoff: a published bounded resumable component invocation
with cumulative limits, exact quantum observations, cancellation/teardown,
remaining-fuel readback and ownership of output through cleanup. Molten continues
to own its scheduler, continuation identity, generation fences and receipts.

### Failure phase is lost

`hardened-wasmtime-shell/src/model.rs::ComponentInvocation::run` creates the
store, links, instantiates and calls the guest before returning one
`InvocationOutcome { class, output, fuel_remaining }`. `store.rs` normalizes
instantiation traps and invocation traps to the same class without recording
phase. Molten's `runtime.rs::instantiate` instead classifies an instantiation
failure as `ComponentInstantiationDenied`; `invoke` classifies an invocation
trap as `ComponentTrap` or `FuelExhausted`. The product shell consequently binds
different successful stage parents before its denial receipt.

A disposable executable probe used two import-free actual components under the
same independently pinned runtime, profile, input, export and operated epoch
service. One core module traps in its start function; the other instantiates and
traps in `actor-turn`. Both prepared successfully. Observed output:

```text
fixture_phase=instantiation artifact_blake3=96235d6134fefe28c28a276bc93bb3795e37f5e72760783f89a947aaabaf3f42 outcome=InvocationOutcome { class: GuestTrapped, output: [], fuel_remaining: 9999 }
fixture_phase=invocation artifact_blake3=4f5a8875ca1e1b43c9d72cb1c85cfec7cf4b63b8871fd5be1c6b2b0262119df0 outcome=InvocationOutcome { class: GuestTrapped, output: [], fuel_remaining: 9997 }
```

The probe asserted `GuestTrapped` and empty output for both cases. Fuel is a work
observation, not a failure-phase discriminator. Product adapters must not infer
phase from arbitrary guest instruction costs or untrusted diagnostic text.

Reproduction inputs were the existing pilot profile, eight-byte input
`0700000029000000`, and root `(list<u8>) -> result<list<u8>, string>` export
`actor-turn`. Both guests have fixed one-page memory, a realloc export returning
1024, and an unreachable exported turn. Only the instantiation case adds
`(func $start unreachable) (start $start)`. The probe used exact artifact hashes,
no imports, a positive 1000-tick deadline and a joined consumer-owned epoch
thread. It was run via `nix develop --builders '' -c cargo run` in an isolated
scratch workspace pinned to the same two producer crates; no sibling source or
path patch was used. Its temporary source is not an installed product path.

The exact instantiation-trap guest template is below. Remove only
`(func $start unreachable)` and `(start $start)` for the invocation-trap guest.
Encoding with the pilot's locked `wat = 1.257.1` reproduces the artifact identities
above; whitespace is not part of the component binary identity.

```wat
(component
  (core module $m
    (memory (export "memory") 1 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      i32.const 1024)
    (func (export "turn") (param i32 i32) (result i32) unreachable)
    (func $start unreachable)
    (start $start))
  (core instance $i (instantiate $m))
  (func (export "actor-turn")
    (param "input" (list u8))
    (result (result (list u8) (error string)))
    (canon lift (core func $i "turn")
      (memory $i "memory") (realloc (func $i "realloc")))))
```

Disposable reproduction-source BLAKE3:
`5e5d5697d96b861369a8c271d95783c0c3b46bd9a2c9bd885fd585cf04bbde3a`;
its resolved lockfile BLAKE3:
`ba2e41a2c61f618542b4d8fb7e8cca2b083493fc57afef406ee84538b183c97a`.

Required upstream handoff: a typed failure phase or equivalent staged execution
API sufficient to preserve inspection, successful instantiation and final
execution/denial receipt inputs. Normalized classes alone are insufficient.

## Materialization compatibility is consumer work, not a schema alias

Molten currently calls its product DTO schema
`mantle.component-materialization-bundle.v1` in
`src/wasm/component/evidence/materialization.rs`. Its `mantle_bundle_ref` hashes
a product-specific sorted line projection; this is **not** the Mantle producer
bundle's canonical identity and is not a canonical packed-Preserves bundle hash.
The local test fixture constructs those product DTOs synthetically; it does not
establish execution of Mantle's published byte verifier.

Mantle's actual producer schema is
`mantle-wasm-component-materialization-bundle-v1`, with report schema
`mantle-wasm-consumer-verification-report-v1`. The producer bundle binds source
closure, lock, final portable component, WIT and package inputs, eight required
stage kinds, expected Octet/runtime profile identities and fixed non-claims.
`verify_consumer_bundle` is structural-only. A production bridge must require
successful `ConsumerRoot::verify_bundle`, `Structural` **and** `Bytes` layers,
matching per-role observations and exact runtime-profile identity, then retain
that producer identity separately from Molten's admission envelope and canonical
receipts. Structural `Verified` alone is not byte verification.

The published file shell stores an explicit `PathBuf` root and uses a final-leaf
`O_NOFOLLOW` open. Source inspection does not establish descriptor-relative,
race-resistant authority over every parent component. Integration must evaluate
that boundary against Molten's node-state filesystem authority; this preflight
makes no stronger filesystem claim and does not add a weaker local replacement.

## Frozen consumer inputs and required mapping

This inventory fixes the existing local corpus at the Molten baseline above; it
does **not** assert a complete local/shared parity run or a total accepted mapping.
The third implementation task remains open because the two execution mappings
above cannot be supplied by the published API.

| Product observation | Required shared mapping / retained owner |
|---|---|
| Profile, WIT, feature and byte identities | Explicit shared feature and artifact facts after unchanged Molten admission; no identity or schema reinterpretation |
| World/import/export facts | Exact component function paths and schema identities; no WASI or undeclared binder |
| Production materialization | Actual published Mantle bundle byte verification, separately linked Molten envelope; no synthetic-fixture promotion |
| Canonical input/output | Shared bounded raw carrier; Molten independently parses and hashes canonical Preserves |
| Invocation success/guest denial/fuel exhaustion | Typed product translation after exact phase, output, resource and receipt validation |
| Instantiation trap versus invocation trap | **Blocked:** published result omits phase |
| Quantum yield/resume, ledger and final fuel | **Blocked:** published shell has no resumable invocation API |
| Deadline | Consumer-operated engine-bound epoch service; must not substitute wall-clock scheduling for deterministic fuel quanta |
| Resources and cleanup | Compare aggregate limits, transfer refusal and cleanup fuel explicitly; no assumed parity with existing per-memory limits |
| Actor and extension execution | Non-test product composition and matching Mantle bundles; consumer enum labels or a direct pilot call are insufficient |
| Authority and effects | New closed versioned product import; current policy, Basalt/UCAN, resource, generation and replay admission remain in Molten |
| Receipts and rollback | One coherent dependency/profile/adapter/consumer cohort; no mixed local/shared canonical evidence |

BLAKE3 source identities recorded before product changes:

```text
8d2d51c095c935dc868ae9cce92f5f002c710b0d62a8fd44c009a119286dbdd7  src/wasm/component/runtime.rs
3c2e21da660d606cffc15e4c68353e0d5533439c050476a6cfe3b24794d42e9d  src/wasm/component/runtime/shell.rs
e673c73db0e4945530f7f833585c55d55b10fe38427178da10b7f3b9c8c98abe  src/wasm/component/runtime/sliced.rs
e4afc9899514e4efc23208dd971847a00b62d879804031f0f89d05f9dd7a9c0d  src/wasm/component/runtime/slicedshell.rs
4c8f951081d068904c6fe0ac224e9944e2c3758fc894a64cbac8549eb5928556  src/wasm/component/tests/support.rs
289acc9f6e246566d20fc49c986e3dcafa960b7c4697fbff19620e3d98db1a9c  src/wasm/component/tests/slicing/yielding.rs
c54b690a58d281d98d857d2878e92c911d048cb1af87ccc01faca3e2a53d4cd7  src/wasm/component/tests/shell.rs
5bea351924145de8946669680125e99c837fc61ac6737e114ba3b9edf7ea0b89  docs/wasm-component-runtime/generated/profile.json
48b088f86827c4fa31ffe6c2151dd32a8db53b3f38afef2b2e1b394d43d20852  docs/wasm-component-runtime/generated/time-sliced-profile.json
83be349bb27975ada30dbe60817c5404df7862babdf02ae229b399886e76d5e8  wit/molten-component-runtime/runtime.wit
```

## Executed shared consumer smoke

From the Molten implementation worktree, inside its Nix shell, using a fresh
executable scratch target, debug information disabled and two build jobs:

```sh
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo run --locked --manifest-path pilots/hwr-consumer/Cargo.toml
```

Exit 0. Observed exact output:

```text
molten-hwr-consumer world=molten:component-runtime@1.0.0/actor export=actor-turn class=Completed actor=7 counter=42 output=070000002a000000 bytes=8
fuel_remaining=9973 fuel_budget=10000
denials malformed=GuestDenied oversized=InputLimitExceeded missing-epoch=DeadlineDenied
scope=pilot-only production-adoption=false materialization-trust=false authority=false
```

The independent pilot still explicitly excludes production composition,
materialization trust and authority. The compiler cache reported disk I/O errors
and disabled caching; both this command and the compatibility probe compiled and
executed successfully without cache hits. This is an environment observation,
not a runtime denial or a disabled correctness check.

## Claim and completion limits

No root dependency pin, lockfile, generated build plan, product runtime, profile,
actor, extension, effect, replay or receipt implementation changed. No runtime
implementation task is marked complete on the strength of this preflight. Do not
sync or archive this change. Unblock only after immutable upstream publication
supplies compatible resumable execution and stage observations, then rerun the
full frozen positive/negative corpus and actual product consumer rails.

The investigation does not prove Wasmtime correctness, component correctness,
effect correctness, host isolation, actor correctness, extension correctness,
whole-system safety, source trust or release eligibility. It adds no production
assertions or permanent tests. Non-executable evidence updates use the documented
prose-only exemption in `docs/proof-workflow.md`; that exemption cannot discharge
the open behavioral implementation tasks.

## Executed local product baseline

Both commands ran in the repository Nix shell against the untouched product
source, with `TMPDIR` and `CARGO_TARGET_DIR` under a task-owned executable
`/dev/shm/aspen-runtime-adoption-20261026` scratch root,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`, and two build jobs:

```sh
cargo test -p molten --lib -- wasm_component:: wasm_performance:: system_extension:: hostcall
cargo test -p molten --lib -- wasm_executor wasm_preserves_abi
```

Observed results, respectively:

```text
test result: ok. 85 passed; 0 failed; 0 ignored; 0 measured; 1653 filtered out
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1727 filtered out
```

The selections overlap; these are not 96 distinct cases. The first exercised
actual synchronous component execution and canonical replay, real fuel-sliced
callbacks with sibling progress and resumed same-work fuel, extension lifecycle,
generation fencing, policy-bound hostcalls, materialization/receipt denials and
runtime/performance-profile negative cases. The second additionally exercised
the existing ordinary actor core-module/Preserves ABI: admitted execution and
exact input/output refs, invalid modules and output, absent memory/WIT, WASI and
undeclared hostcall denial. Neither is evidence that the ordinary actor or
extension has migrated to the shared runtime.

## Structural validation

Executed after recording the blocked metadata and evidence:

```sh
nix run path:/home/brittonr/git/OnixResearch/cairn#cairn -- validate --root . \
  --policy /home/brittonr/git/OnixResearch/cairn/cairn-policy/generated/cairn-policy.json --strict
```

Exit 0, `valid: true`, empty `findings`, `change_issues` and `spec_findings`.
The selected explicit provider policy BLAKE3 was
`7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`.
This checks lifecycle structure, not behavioral completeness or archive readiness.
