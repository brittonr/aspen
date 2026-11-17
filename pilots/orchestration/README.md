# Aspen standalone batch orchestration pilot

The reviewed source candidate is now maintained in-tree. Run the pinned repository
shell and `scripts/verify.sh` before use. This workspace implements a standalone,
single-controller, multi-worker import-free WebAssembly Component batch profile
and a separately installed **`pure-net-eval/v1`** sequential native pure workload.
Neither is a production release, an HA controller, or a registered Molten native
system extension. The existing node daemon is unchanged.

The original allocation functional core and application shell are retained. The
new controller, worker, durable adapters and CLI build around that work. The
historical reference-scheduler patch remains pinned to Molten commit
`bb6f3830ee7327da9875ea85a8c8e25697eddc35`; it is not applied over current mainline.

## Build and exercise

Use the repository's Nix development shell and pinned `nightly-2026-05-26` toolchain.
Wasmtime 45.0.0, Iroh 1.0.0, Redb 4.1.0 and Preserves 5.0.0-rc.7 are explicitly
selected. No private Molten dependencies are needed by this standalone workspace.

From the repository root:

```sh
CLIPPY_STRICT=1 nix develop -c bash pilots/orchestration/scripts/verify.sh
nix develop -c bash pilots/orchestration/scripts/demo.sh
```

`verify.sh` formats the authored source, executes the Rust tests (including a
separate-process two-worker test), runs Clippy and builds the release binary.
It writes `target/validation/verify.log` beneath this workspace. `CLIPPY_STRICT=1`
makes warnings fatal. See [the adoption guide](../../docs/standalone-batch-orchestration.md)
and [source.ncl](source.ncl) for provenance, repairs and evidence boundaries.

The main binary is `target/release/molten-orchestrator`, or under an explicit
`CARGO_TARGET_DIR`. The demo uses a fresh private temporary directory and keeps its
state and logs on exit. It never reuses or deletes an existing deployment root.
Its default UDP ports 45100 and 45101 can be changed with `CONTROLLER_PORT` and
`WORKER_PORT`. Do not publish deployment keys or databases as public evidence.

The reviewed standalone `Cargo.lock` is committed separately from the root lockfile.
Dependency updates must retain that separation and repeat the strict verification
and real-process demo. Molten contributors maintain this pilot.

## Implemented execution path

An enrolled operator publishes component/data objects and submits a bounded DAG
with an explicit idempotency key. The controller validates the immutable job,
selects a fresh eligible worker, materializes content, and atomically commits the
assignment **and its resource reservation** in one Redb state transaction. Only
a definite successful commit authorizes that caller to send one start request.

The worker independently checks the controller's authenticated Iroh identity,
namespace, generation, policy, request identity and capacity. Acceptance is
journaled before a launch ticket is minted. An exact duplicate returns the
existing record and never produces another ticket. Cancellation of a not-yet-seen
request creates a durable tombstone, fencing a delayed start.

The worker executes an import-free component through Wasmtime, publishes its
output, and records a stopped terminal result. The controller verifies the exact
request/result binding and durably copies successful output before releasing the
reservation or enabling dependencies. Literal inputs precede dependency outputs;
dependency outputs follow sorted dependency-id order.


## Explicit pure-net workload

`pure-net-eval/v1` uses this same controller/worker attempt ledger, reservation,
retry and accepted-output publication path. It is opt-in: component grants do
not authorize native calculation, and successful compilation is not a grant.
The producer is Noble `f7af9e972aa2968d22978794a667d267cf2745e8` with a separately
identified Aspen observation overlay. Its performance NO-GO remains in force;
this service does not claim a speedup or default adoption.

The operator path is:

1. Export reviewed limits from `profiles/pure-net.ncl` with
   `nickel export --format json`. The Rust defaults are mechanism ceilings, not
   a qualified deployment profile.
2. Run `molten-orchestrator pure-net manifest --backend sequential-net
   --limits limits.json --out manifest.frame` using the actual deployment
   executable. The manifest binds that executable's bytes, compiled dependency
   closure, producer/overlay, arithmetic, rules and all finite limits. Use
   `--backend independent-reference` only for the explicitly selected reference.
3. Run `molten-orchestrator pure-net compile --source source.noble
   --input-arity 2 --inputs ordered-inputs.json --program-out program.frame
   --inputs-out inputs.frame`. The inputs JSON is an ordered I64 array; `-`
   with `[19,7]` evaluates to `[12]`.
4. Create policy with exact `new-policy --pure-net-manifest MANIFEST_ID
   --pure-net-program PROGRAM_ID` grants. Initialize and enroll controller,
   worker and operator roots using the normal commands below. Server roots
   require an explicit nonzero `--bind` address.
5. Install the same manifest create-only in both server roots with
   `pure-net install --root ROOT --manifest manifest.frame`. Replacement,
   another executable/closure, or missing native grants fails closed.
6. Start the worker under Linux `prlimit` or equivalent OS limits no larger
   than the selected manifest's address-space and soft/hard CPU limits.
   Startup checks the observed limits; there is no unbounded fallback.
7. Publish `pure-net-manifest`, `pure-net-program` and `data` content with
   ordinary `put`. Submit a task whose workload is
   `{"PureNet":{"manifest":MANIFEST_ID,"artifact":PROGRAM_ID}}` and whose
   `inputs` contains the ordered-input object. The existing `submit`, `run`,
   `progress` and `get` paths remain authoritative. Decode a fetched report with
   `pure-net receipt --file receipt.frame`; readback never grants authority.

Native work is trusted in-process calculation, **not a sandbox**. One retained
arena/queue resumes across cooperative local quanta; total work does not reset
on a handoff. Each turn reserves at most `quantum_work + max_atomic_work` logical
units, including size-dependent rewrite validation, independently of semantic
interaction fuel. Decoder, frontend, validation, setup, graph/queue/memory,
readback, observations, concurrency and aggregate retry work have separate caps.
Whole-worker OS limits are not per-evaluation instruction metering.

Canonical receipts bind the exact program, reconstructed checked IR/net, ordered
input, manifest, executable/closure, limits and grants. Stable phase/work/outcome
observations replay; timing, process IDs, allocator addresses and profiling
diagnostics do not enter semantic identity. Current grants/revocations,
generation and cancellation fence successful publication after calculation.
Stopped failure reconciliation remains possible after execution revocation.

Repeat the real CLI and process-restart qualification from the repository root:

```sh
nix develop -c cargo test --manifest-path pilots/orchestration/Cargo.toml -p molten-pure-net --lib --locked
nix develop -c cargo test --manifest-path pilots/orchestration/Cargo.toml -p molten-orchestrator-adapters --test pure_net_runtime --locked
nix develop -c cargo test --manifest-path pilots/orchestration/Cargo.toml -p molten-orchestrator-cli --test pure_net_e2e --locked
```

The process campaign kills and waits for its own worker before reopening at
acceptance, begin, calculation, content, terminal commit and reply boundaries.
It checks durable attempts and one authoritative completion, not merely logs.
`MOLTEN_PURE_NET_EVIDENCE_DIR` retains private inspection/receipt copies;
`MOLTEN_PURE_NET_HEAPTRACK_PREFIX` with `MOLTEN_PURE_NET_ALLOCATIONS_DIR` enables
a separate worker-lifetime allocation observation. Raw deployment/profiler
material stays private. See the [adoption guide](../../docs/standalone-batch-orchestration.md)
for evidence classes and limitations. Molten contributors own the bridge,
snapshot importer, standalone adapter and repeatable qualification commands.
Parallel reduction, portable checkpoints, root-host adoption and cross-host
qualification remain separate gates.

## Architecture

| Layer | Owns | Dependencies |
|---|---|---|
| `core` | Original fixed-size allocation identities, fences and transitions | None; `no_std` |
| `application` | Original single-allocation use cases and output ports | `core` |
| `batch-core` | Bounded DAG, controller and worker transitions, resource accounting, placement policy, explicit operator-policy rules | Original `core`, Serde; `no_std` + `alloc` |
| `batch-application` | Effect ordering, CAS/reconciliation, controller and worker use cases; application-owned ports | `batch-core`, `async-trait`; no runtime or backend |
| `codec` | Versioned Preserves projection, canonical identity and verified reconstruction | `batch-core`, Preserves, BLAKE3, Serde |
| `adapters` | Capability-rooted Redb, file-policy observations, Iroh, Wasmtime, installed pure-net execution, bounded RPC ingress and composition | Application ports and selected infrastructure |
| `cli` | Explicit root acquisition, enrollment, operator commands and launch | Application and adapters |
| `render` | Original pure CPU scene renderer, versioned manifest, checked tiles/assembly and fixture shell | `batch-core`, `codec`, Serde |
| `pure-net` | Exact Noble source admission, sequential retained-arena reduction, independent source reference, finite work and canonical semantic receipts | `batch-core`, `codec`, reviewed source-only Noble snapshot |

Output ports are `StateStore`, `WorkerJournal`, `ContentPort`, `IdentityPort`,
`PlacementPort`, `AdmissionPort`, `WorkerClient`, `ExecutionPort` and
`ObservabilityPort`. Inbound adapters call `Controller` or `WorkerService`.
No filesystem, network, process, clock or executor access belongs to the core.
The file-policy adapter observes files; the pure core evaluates the policy rules.

The standalone profile owns its own bounded reservation ledger and first-fit
placement policy. It does **not** pretend those reservations are committed in the
existing Molten membership service. A node-host integration must provide one
shared authoritative reservation owner rather than run two independent ledgers.
See `docs/MOLTEN-INTEGRATION.md` for the exact remaining boundary.

## Storage, identity and transport

One exclusive Redb database per controller or worker holds complete canonical
state and its reconstruction projection in the same immediate-durability
transaction. Whole-state CAS compares both. Terminal attempts and cancellation
tombstones are retained. Content objects use separately bounded tables and
transactional counters. Reads recheck the manifest, length and raw digest.

Canonical identity is BLAKE3 of the versioned **Preserves** projection. The JSON
projection is a non-authoritative materialization cache: decoding regenerates the
canonical bytes and requires exact equality. JSON adapter frames bind that same
Preserves-derived identity and reject alternate encoding, duplicate fields and
trailing data. This is an explicitly new RPC profile, not wire compatibility with
Molten's native-extension protocol. See `docs/PROTOCOL.md`.

Iroh uses the `Minimal` preset, explicit direct IP addresses, exact peer keys and
ALPN `molten-orchestrator/batch-rpc/v3`. No relay, DNS discovery or transport retry
is silently selected. A controller accepts only configured operator keys. A
worker accepts only its configured controller key. Connectivity is not enrollment.

Filesystem roots are explicitly supplied by the operator. Runtime files are
opened through a `cap-std` directory handle with final symlink following disabled;
database files never reopen through an ambient path. CLI-created roots are private
and key/config files are created with private Unix permissions.

## Recovery and failure policy

A lost transport result or unknown assignment commit is **not** a reason to start
another copy. Reservations remain held until a correctly bound, stopped terminal
observation is committed. Missing worker state triggers a cancellation tombstone,
not blind re-dispatch. A disappeared/unreachable worker never transfers ownership
by itself. This is deliberately conservative and may sacrifice availability.

A worker restart first acquires the exclusive database lock, then marks interrupted
in-process component work stopped/failed in a new boot generation. The live
Wasmtime adapter retains that database lifetime inside the blocking execution
closure: aborting an async caller cannot release the lock while its component is
still running. This recovery rule is valid **only** for this no-import,
in-process component profile. It cannot be reused for detached native processes.

Retries create new attempt identities and epochs, only after stopped terminal
failure, subject to attempt and logical-delay bounds. A failed dependency blocks
its dependents; independent branches may finish. A run is not reported terminal
while independent pending/active tasks remain. Cancellation stops pending tasks
and asks active workers to stop; their capacity remains reserved until teardown.

Graceful worker shutdown persists drain. Restart a deliberately drained worker
with `serve --resume`; then use the controller's `drain --resume --node KEY` command
to re-enable placement. Ordinary crash recovery does not silently erase tombstones.

## Enrollment and cross-host launch

Create one shared policy and distinct private roots. Do not copy private keys
between machines. Transfer the policy and public enrollment information through
your normal trusted provisioning channel.

```sh
BIN=target/release/molten-orchestrator
$BIN new-policy --out policy.json --allow-pure-components
$BIN init --root controller --role controller --policy policy.json --bind 0.0.0.0:45100
$BIN init --root operator --role client --policy policy.json
$BIN identity --root controller
$BIN identity --root operator

# On worker A; CONTROLLER_KEY is the controller's printed public id.
$BIN init --root worker-a --role worker --policy policy.json \
  --bind 0.0.0.0:45101 --controller CONTROLLER_KEY
$BIN identity --root worker-a

# On controller; worker address must be a reachable direct IP, not 0.0.0.0.
$BIN allow-operator --root controller --peer OPERATOR_KEY
$BIN add-worker --root controller --peer WORKER_A_KEY --address 192.0.2.20:45101
$BIN serve --root controller
# On worker:
$BIN serve --root worker-a
```

Replace uppercase values and documentation IP addresses with your actual public
ids and addresses. CLI server enrollment is static and takes effect on restart.
Capacity must match on both sides. Labels may be explicitly added to the
controller enrollment's `labels` object before startup; they are operator-declared,
not hardware attestation. With a direct reachable IP/UDP path, the same executable
supports separate hosts; that deployment path has not been validated here.

Default policy denies components unless allowlisted or the operator explicitly
selects `--allow-pure-components`. That flag grants only the import-free component
profile, not arbitrary native execution. It is still a trust decision about CPU,
memory and compiler exposure. Apply `revoke --component ID --root ROOT` to the
relevant controller/worker roots to block new execution. This does not itself
interrupt active work; use cancellation. Policy identity is immutable for an
existing state namespace; editing the policy file is not a silent upgrade path.

## Operator commands

```sh
TARGET=CONTROLLER_KEY@192.0.2.10:45100
$BIN put --root operator --target "$TARGET" --kind component --file examples/constant.wat
$BIN put --root operator --target "$TARGET" --kind data --file input.bin
$BIN submit --root operator --target "$TARGET" --key run-001 --file job.json
$BIN status --root operator --target "$TARGET"
$BIN status --root operator --target "$TARGET" --run RUN_ID
$BIN status --root operator --target "$TARGET" --raw
$BIN plan --root operator --target "$TARGET"
$BIN cancel --root operator --target "$TARGET" --run RUN_ID
$BIN drain --root operator --target "$TARGET" --node WORKER_KEY
$BIN get --root operator --target "$TARGET" --id OUTPUT_ID --out output.bin
```

An example DAG is generated by `scripts/demo.sh`. Task ids and dependency lists
must be sorted and unique. Reusing the same submit key with a changed specification
is an explicit conflict. A new key intentionally creates a new run even when the
specification is identical. Output files and deployment roots are create-only.

The guest ABI is `run(list<list<u8>>) -> list<u8>`. `examples/constant.wat` exercises
it without another build tool. `examples/concat-guest` supplies a real Rust guest
that concatenates its inputs; `scripts/build-guest.sh` builds it with
`wasm32-unknown-unknown` and wraps it using `wasm-tools`. Install that target and
CLI separately. Guest build/validation has not been executed here either.

## Bounds and deployment limits

The current profile retains at most 128 runs, 64 tasks per run, 8 attempts per task,
32 enrolled workers and 128 active attempts. Each worker retains at most 4096
attempt records. Each store retains at most 4096 content objects and 1 GiB of raw
content; each object and the total inputs to a task are limited to 4 MiB. Canonical
state and reconstruction are each limited to 32 MiB; an RPC frame to 40 MiB, with
at most eight active ingress connections. Configured deadlines are bounded.

There is **no automatic GC or administrative erasure of dedupe history**. Reaching
a bound fails closed. Plan a separately reviewed retention/migration procedure;
do not delete a worker database to make room while old requests can still arrive.

CPU is accounted in reservations, not enforced per task by this Rust runtime.
Wasmtime limits guest linear memory, instances, tables, fuel and execution epochs.
Those are not a cap on total compiler/native heap memory. Compilation itself is
not interrupted by fuel or epochs. Run workers under OS/cgroup CPU, memory and
process limits; see the example systemd unit. A compiler failure/OOM can lose the
worker process. No production isolation, scale or security claim is made.

Not implemented in this profile: root node-host activation/port registration,
Basalt/Cairn provenance admission, shared Molten placement/delivery ownership,
controller HA or Raft integration, general containers/native jobs, WASI/hostcalls,
persistent services, rolling deployments, automatic node-failure replacement,
NAT/relay qualification, history GC and durable metrics export. These are distinct
integration/production scopes, not silently substituted with passing fixtures.

## Validation

See `docs/AUTHORSHIP-VALIDATION.md` for what actually ran during authorship. Rust
tests cover core transitions, policy/identity rules, canonical reconstruction,
real Redb reopen/CAS, cancellation tombstones, uncertain commits, actual Wasmtime
execution, and a two-worker multi-process DAG. Inclusion of a test is not a passing
result. Same-host multi-process tests are not cross-host qualification.

The original scheduler fix requires the full Aspen dependency environment. After
applying the guarded patch to its exact base, run `scripts/reference-tests.sh`.
Old simulation reports do not validate the changed epoch-bearing command shape.

## Durable progress and rendering

Aspen's [progress v1 contract](docs/PROGRESS.md) exposes controller-accepted output
references and terminal runs through the existing admitted Iroh RPC. State,
events and cursor position commit together in Redb; a selected-run snapshot and
watermark share one read transaction. Pages retain stable canonical identities,
explicit retention/artifact gaps and bounded work. Observation disconnect never
requests cancellation. The implementation shares the authoritative
`crates/molten-core/src/committed_stream` source, so packaging must include that
tree as declared by `default.nix`; no forked stream core is maintained here.

The [original CPU renderer](render/README.md) builds an import-free component,
renders a frozen 640x360 scene through ordinary batch tasks, checks complete tile
coverage and emits a verified image. `render-demo.sh` consumes accepted progress
and retains inspectable manifests, artifacts and outcomes. See the exact Nix
entrypoints and qualification limits in the linked docs. Existing `molten-*`
crate/binary/schema names are Aspen's internal Molten codename, not another product.

New lifecycle/progress/render fixtures are authored, not passing evidence. No
cross-host, machine-loss, production-isolation or cross-architecture equality
claim follows from their presence. Parent integration must regenerate/review
the standalone and guest locks and run the source/package/process rails.
