# Molten standalone batch orchestration pilot

The reviewed source candidate is now maintained in-tree. Run the pinned repository
shell and `scripts/verify.sh` before use. This workspace implements a standalone,
single-controller, multi-worker **import-free WebAssembly Component batch profile**.
It is not a production release, an HA controller, or a registered Molten native
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

## Architecture

| Layer | Owns | Dependencies |
|---|---|---|
| `core` | Original fixed-size allocation identities, fences and transitions | None; `no_std` |
| `application` | Original single-allocation use cases and output ports | `core` |
| `batch-core` | Bounded DAG, controller and worker transitions, resource accounting, placement policy, explicit operator-policy rules | Original `core`, Serde; `no_std` + `alloc` |
| `batch-application` | Effect ordering, CAS/reconciliation, controller and worker use cases; application-owned ports | `batch-core`, `async-trait`; no runtime or backend |
| `codec` | Versioned Preserves projection, canonical identity and verified reconstruction | `batch-core`, Preserves, BLAKE3, Serde |
| `adapters` | Capability-rooted Redb, file-policy observations, Iroh, Wasmtime, bounded RPC ingress and composition | Application ports and selected infrastructure |
| `cli` | Explicit root acquisition, enrollment, operator commands and launch | Application and adapters |

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
ALPN `molten-orchestrator/batch-rpc/v1`. No relay, DNS discovery or transport retry
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
