# Design: Adopt the standalone batch orchestration pilot

## Context

The candidate delivery is a seven-crate standalone workspace, not a tree member. The
independent review proved the pure cores are sound (14 batch-core transition tests pass
untouched) and located every defect in the untested adapter and lint surface. Adoption
must preserve that separation: vendor the mechanism, land the repairs, re-run the
evidence, and keep all admitted-system authority outside the pilot.

## Success Contract

The vendored workspace builds and passes its full test suite, strict Clippy, and the
two-process demo inside this repository, with recorded provenance and a resolved
`Cargo.lock`. The reference scheduler patch passes Aspen's own tests at the pinned
base. No admitted-system surface changes in this change.

## Decisions

### Vendor with provenance, not trust

The source enters `pilots/orchestration` as new repository-owned files under
AGPL-3.0-or-later, with a provenance record naming the candidate bundle, the base
commit `bb6f3830ee7327da9875ea85a8c8e25697eddc35`, the review date, and the applied
repair list. The resolved `Cargo.lock` is committed and reviewed.

### Land the three review repairs as first-class changes

1. `adapters/src/storage.rs`: qualify Redb read transactions through
   `redb::ReadableDatabase` (two sites).
2. `adapters/src/root.rs`: directory durability sync must not `fsync` a cap-std
   `O_PATH` descriptor; open the root directory with real flags for the sync and keep
   the capability-root admission checks intact.
3. Strict-Clippy conformance: resolve `should_implement_trait` on `Resources::add` by
   implementing the operator trait or renaming, without weakening the capacity checks.

Each repair keeps or extends its test: the durability repair must keep a regression
fixture that fails on `EBADF` under the previous implementation.

### Re-run all evidence inside the repository

`bash scripts/verify.sh` with `CLIPPY_STRICT=1`, the release build, and
`scripts/demo.sh` run from the vendored tree. Logs, `Cargo.lock`, toolchain versions,
and the source revision are retained as evidence. A failure is a defect to resolve, not
a reason to weaken assertions.

### Use the guarded patch path for the reference scheduler change

The `lease_epoch` patch applies only through `apply.py --check` guards at the exact
base commit in a clean worktree. If the checkout has moved, reconciliation is explicit;
the guards are never removed to force an apply. `scripts/reference-tests.sh` runs with
repository dependencies before the patch is considered landed.

### Keep the pilot unenrolled

The pilot keeps its own CLI, policy file, and database. It does not register in
`SystemExtensionHost`, join the root Cargo workspace, change the ALPN registry, or emit
admission receipts. Its operator policy is a named standalone deployment profile, not a
substitute for Basalt/Cairn admission. Any later enrollment is its own change.

### Treat integration as planned requirements, not implementation

`docs/MOLTEN-INTEGRATION.md` port mappings (placement, admission, content identity,
delivery, execution, observability) become explicit follow-on requirements with
non-claims. This change delivers the plan and the vendor; it does not rewire ports.

## Functional Core and Imperative Shell

- **Cores** (`core`, `batch-core`): allocation FSM, DAG, scheduling, journals,
  admission rules, recovery transitions. Stay pure and `no_std`.
- **Application** (`application`, `batch-application`): effect ordering over owned
  ports. Stays infrastructure-free.
- **Shell** (`adapters`, `cli`): Redb, capability-rooted files, Iroh, Wasmtime,
  process composition. All repairs that touch effects live here.

## Risks and Controls

- Vendored code may hide further compile-time or runtime defects. The full test suite,
  strict Clippy, and the demo run in-tree before the change can complete.
- The fsync repair touches durability. A regression fixture plus the existing
  restart-persistence tests bound the risk.
- The pilot can drift toward product use without enrollment gates. The boundary
  requirement and non-claims forbid implicit fallback or receipt invention.

## Non-Claims

Adoption evidence does not prove Wasmtime, component, worker, transport, cross-host,
or production correctness; it does not admit the pilot as a system extension; and it
does not establish controller HA, exactly-once external effects, or release eligibility.
