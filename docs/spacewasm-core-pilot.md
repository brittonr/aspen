# SpaceWasm core pilot

The optional `config/spacewasm/profile.ncl` authoring profile names
`spacewasm-core-mvp-pilot`, NASA source revision
`e24cf09355a90497148eb5029fdb8e3400bd63e3`, and `molten.wasm.abi.v1`.
Loading defaults does not select the engine. An absent profile, Wasmtime request,
or component request cannot fall back to this pilot.

The engine dependency is the Apache-2.0 [NASA SpaceWasm source](https://github.com/nasa/spacewasm/tree/e24cf09355a90497148eb5029fdb8e3400bd63e3),
not a semver-compatible substitute. Its upstream version is `0.0.0`.

## Admission boundary

`spacewasm::inspection::inspect` validates actual bytes with wasmparser's WASM1
feature set. It requires exactly four exports:

| Export | Type |
| --- | --- |
| `memory` | memory index zero |
| `molten_alloc` | `(i32) -> i32` |
| `molten_dealloc` | `(i32, i32) -> ()` |
| `molten_run` | `(i32, i32) -> i64` |

The only optional imports are unique `molten:hostcall/assert` and
`molten:hostcall/send`, each `(i32, i32) -> i64`. Signatures are resolved through
actual type/function indices. There must be one memory with positive initial
size and an explicit maximum no larger than 256 pages. Tables, start functions,
and every `memory.grow` instruction are denied, including unreachable growth.
Components and post-MVP proposals are denied before runtime construction.

The pure `admission::admit` function compares module identity, explicit engine,
profile, current actor/generation, authority/resource identities, verified
hostcall grants, and externally verified evidence. Blockers are typed, sorted,
and deduplicated. `Plan` cannot be constructed or mutated by external callers.
Its getters expose the exact admitted profile and evidence bindings.

`admission::verify_authority` obtains allowed hostcall names from actual Basalt
UCAN verification, resolver/proof/revocation/caveat services, and local Basalt
policy enforcement. It does not treat submitted receipt references as grants.
Scheduler currentness/resource observations remain explicit trusted inputs;
this function is not a scheduler or resource reservation service. Initial
admission does not consume a holder invocation: the live dispatch boundary must
verify its exact holder signature and replay/current-policy/resource conditions
again immediately before an effect.

## External evidence is not a self-hash

`evidence::verify` is an imperative verification boundary. Its result has no
public constructor. Deployment authority supplies immutable Nix-store verifier
paths and expected executable hashes, a published expected Mantle bundle identity,
Octet profile identity, Mantle cohort identity, and differential probe identity.
Do not populate those pins by reading the submitted receipt itself.

`Sources` borrows a `cap_std::fs::Dir`; every source path is relative and rejects
traversal. Reads are bounded: module/input 64 KiB, JSON reports 4 MiB, probe 64 MiB.
External programs receive Linux procfs paths to already-open capability files
or directories, including an already-open report destination. Executable hashes
are streamed with a 128 MiB bound, and process output is discarded rather than
collected without bounds. Each verifier has a 35-second outer process bound;
timeout/error kills its dedicated unreaped process group and reaps the child.
Other hosts fail closed at the procfs handoff.

The pinned Mantle `verify BUNDLE` checks the bundle; the consumer additionally
requires the candidate's exact named `fixtures/wasm/molten-abi-*.wasm` membership.
Pinned Octet `artifact verify --artifact-dir DIR` checks the static artifact,
whose bytes, artifact identity, ABI profile, source cohort, and error findings
are checked independently. Pinned ChaosControl
`spacewasm-molten-abi-differential PROBE MODULE INPUT REPORT` must emit
`chaoscontrol.spacewasm-molten-abi-differential.v1`, binding exact source,
module, canonical input, and probe identities. Reference and uninterrupted,
segmented, and streamed normalized rows must agree. Completed output must parse
as canonical Preserves; typed `invalid-output` must independently fail that parser
while retaining exact bytes and is never accepted actor output. Trap rows require
one of the closed normalized trap classes; non-trap rows require a null class.
Unsupported outcomes are denied. The older host-only differential schema is not
accepted. `Plan::input_ref()` binds the verified
input for the runtime to check before construction; `fixture_name()` binds the
exact named case. The canonical Preserves evidence reference is an identity of
verified reports, not an independent trust root.

Mantle's published expected bundle identity must come from the parent-owned
real build/publication step, never from the candidate's self-consistent local
manifest. Actual ABI differential production is a separate provider executable,
not a conversion of existing host-only receipts. No missing verifier, stale
evidence, report success flag, or unavailable publication is replaced with a
synthetic pass.

## Independent resources and claims

Limits separately represent total interpreter allocation, IR/code bytes, stack
bytes, guest memory pages, table elements (zero in this cohort), segment fuel,
total turn fuel, hostcall bytes, input/output bytes, continuation count, retained
continuation bytes, and retention turns. The IR hard ceiling is 65,536 bytes;
stack bytes must be aligned to four and allocation must fit the upstream signed
32-bit allocator accounting. A fixed interpreter arena does not prove bounded
guest memory. The shell applies a thread-bound allocation budget and an independent
IR allocation cap before streaming decode. Stack allocation is in 32-bit words;
the profile uses bytes. Guest linear memory has its own allocator and never grows.
Only one retained engine may exist on a thread; nesting is a typed denial.

Production and release are always false. The exact non-claims deny component
compatibility, portable state, cross-host migration, runtime correctness,
sandbox completeness, production readiness, and release eligibility.

## Actual execution and replay

`engine::Engine::start(bytes, Start)` consumes an unforgeable admitted plan,
canonical input, exact binding, current-admission resolver, and host effect port.
It rechecks module/input identity and current actor/generation before decoding
1024-byte chunks through NASA's `WasmStream` and compiling its actual IR. There is
no Wasmtime import, runtime fallback, or local guest build in this engine.

`resume(&ResumeContext)` runs a bounded instruction segment and returns a typed
yield, actual terminal observation, or non-replayable proof failure. Allocator, actor, and deallocator calls
share the total instruction budget. All modules are stepped at single-instruction
boundaries because the upstream runner does not expose consumed fuel; charging a
whole granted slice after an early return would incorrectly exhaust the turn.
This also pushes a hostcall's actual i64 result before admitted host pause, rather
than mistaking NASA's raw pause-without-result behavior for a completed ABI call.
Continuations retain owned stack/PC/memory/table state, not serialized pointers.
Actor generation, artifact, profile, input, authority, resource, effects, and
logical retention are fenced on every resume. Stale failures drop guest/code/stack
state; consuming `cleanup()` drops the remaining store and allocation guard.

`authority::AuthorizedHost` verifies actual UCAN, Basalt policy, exact holder
signature, operation profile, and replay admission immediately before calling the
injected effect mechanism. Scheduler resource/currentness remains its existing
owner's responsibility. `replay::Recording` records actual admitted returned bytes
or typed failed dispatches, including post-effect uncertainty without a fake reply.
`replay::Replay` has no live effect port and rejects missing/reordered/cross-authority
calls and unconsumed effects. Normalized observations compare the complete typed
terminal failure (including reason), canonical output versus rejected bytes versus
absent output, ordered hostcalls, resource class, and actual post-terminal memory/global
identity. Harness/proof errors and missing state cannot establish replay. Terminal
state is captured before alias-safe cleanup. Segment counters are engine facts,
not cross-engine equivalence claims.

`fixture::run` consumes exact admitted Mantle fixture members and real supplied
admission/effect contexts; it does not mint a fixture grant. `fixture::replay`
compares independently admitted segmented and uninterrupted executions. Operator
`status` values bind canonical observations and remain experimental.
`operator::run_configuration(root, relative_configuration, output)` is the
capability-rooted fixture command boundary. It verifies actual producer artifacts,
creates real Ed25519 UCAN/holder signatures under an isolated fixture-only trust
root, performs local in-memory assertion effects, and executes the NASA interpreter.
These keys are never a production credential or normal actor fallback.
`operator::replay_configuration(root, left_config, right_config, output)` independently
verifies both sets of producer artifacts and independently admits each supplied
profile. The left NASA execution records actual authorized host replies or typed
failures. Only after that execution completes does the right receive the recorded
log through `replay::Replay`, which has no live effect port; full consumption is
mandatory. `fixture::compare_replay` applies the same invariant, segmentation and
terminal/resource checks as `fixture::replay` before exact observation comparison.
The command publishes canonical replay status, never compares submitted status receipts.
Only segment fuel is excluded from the shared resource identity; all other resource
bounds and the complete scenario (including failure reasons/cancellation schedule) remain bound.


The bounded JSON configuration has required fields `trust`, `profile`, `name`,
`scenario`, `bundle`, `octet`, `module`, `probe`, `input`, and `chaos_report`.
`trust` contains explicit published bundle/cohort/profile/probe identities and
immutable Nix-store verifier executable paths plus their expected BLAKE3 hashes;
none is inferred from the candidate. Every source path is relative to `root`.
`profile` is the complete typed profile. `scenario` is `"Standard"`,
`"StaleGeneration"`, `"HostPause"`, `{"HostTrap":{"reason":"fixture-host-trap"}}`,
`{"HostUncertain":{"reason":"fixture-after-commit"}}`, or
`{"Cancel":{"after_segments":0,"reason":"fixture-cancel"}}`.
Host scenarios require the actual `molten-abi-hostcall` member. Host uncertainty first
commits the exact assertion bytes to the isolated in-memory effect log, then returns
`Failure::Uncertain`; it never fabricates successful actor output. Cancellation calls
the real engine cancellation boundary using the supplied current-admission context.
For rejected output, expected bytes come from the externally verified differential
report, not from substituting the input or fabricating an empty result.
The fixed memory-denial fixture requires an explicit one-page profile. Successful
acceptance writes create-new, synced `status.preserves`; no existing receipt is
overwritten. Canonical status identity and independently pinned profile references
are required by `status::readback`. Normal live actor execution still requires its
real scheduler/current-resource resolver and admitted effect mechanism.
### Concrete operator smoke command

The opt-in smoke below runs actual independent NASA executions and canonical status
readback. It requires real published artifacts and operator-selected pins; it does
not build guests, infer trust from submitted reports, or fall back to synthetic data.
Create two complete JSON configurations under an explicitly selected artifact root.
For the segmented configuration set `profile.limits.segment_fuel` to `1`; for the
uninterrupted configuration set it to the same `profile.limits.total_fuel` bound.
Keep every other profile field, fixture member, canonical input, scenario, and
authority/effect/resource binding equal. The left performs actual authorized effects;
the right consumes their recorded outcomes without executing those effects again.
Each configuration must independently name the correct verified module, bundle,
Octet directory, probe and ChaosControl report.
Verifier executables and published trust identities must come from the actual
provider publication/admission step, never placeholder digests or self-derived pins.

With `SPACEWASM_ROOT` naming that directory, `SPACEWASM_LEFT` and `SPACEWASM_RIGHT`
naming capability-relative JSON files, `SPACEWASM_OUTPUT` naming a precreated
empty output directory, and `SPACEWASM_EXPECTED_TERMINAL` set to the exact class
(`finished`, `deterministic-trap`, `total-instruction-bound`, `denied`, `cancelled`,
`effect-uncertain`, `invalid-canonical-output`, or `stack-bound`), run:

```sh
nix develop -c cargo test --lib \
  spacewasm::operator::smoke::configured_replay_smoke -- --ignored --exact --nocapture
```

Run each row below with its own empty output directory and actual named member:

| Actual member | Scenario | Required terminal |
| --- | --- | --- |
| `molten-abi-roundtrip` | `"Standard"` | finished with exact canonical input/output |
| `molten-abi-trap` | `"Standard"` | actual `Trap("unreachable")` |
| `molten-abi-out-of-fuel` | `"Standard"` | total instruction budget exhausted |
| `molten-abi-roundtrip` | `"StaleGeneration"` | live-generation denial before execution |
| `molten-abi-out-of-fuel` | `{"Cancel":{"after_segments":0,"reason":"fixture-cancel"}}` | scheduled cancellation before execution |
| `molten-abi-hostcall` | `{"HostUncertain":{"reason":"fixture-after-commit"}}` | committed assertion followed by uncertainty |
| `molten-abi-invalid-output` | `"Standard"` | exact rejected bytes, never canonical output |

Executed replay requires an actual left fuel yield and no right fuel yield. The
only no-instruction exemption is explicit bound boundary evidence for preexecution
denial/cancellation on both sides. The memory-limit fixture remains an actual
one-page admission-denial diagnostic via `run_configuration`, not a fabricated
post-execution observation or a replay of nonexistent guest state.

The smoke prints the receipt identity and experimental readback after a create-new,
synced `status.preserves` publication. Reusing an occupied output directory fails
rather than overwriting evidence. A changed terminal reason, output classification,
ordered hostcall, resource class, or final-state identity must deny replay.



## Runnable negative fixtures

`src/spacewasm/tests.rs` covers missing/oversized memory maxima, growth, start,
tables, ambient imports, undeclared hostcalls, wrong hostcall/run signatures,
post-MVP instructions, and promotion. Admission-local tests cover absent-profile
fallback denial and deterministic aggregation of stale currentness/resource and
missing-evidence blockers. Evidence-local negatives reject missing external
publication pins, hostless member substitution, unsupported outcomes and invalid
canonical output. Operator boundary regressions additionally cover resource identity
across scheduling-only changes, cancellation/uncertainty scenario binding, malformed
rejected-byte hex, and actual assertion commit-before-uncertainty versus trap-before-commit.
The focused command is `cargo test spacewasm::`.

No builds, tests, linters, or formatters were run for this slice, as explicitly
required by the assignment. The commands and fixtures above are runnable
validation instructions, not claimed successful execution evidence.
