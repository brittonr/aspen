# Standalone batch orchestration pilot

<!-- r[impl molten.batch_orchestration.vendor] -->
<!-- r[impl molten.batch_orchestration.boundary] -->

`pilots/orchestration` is a repository-owned, seven-crate workspace for a
single-controller, multi-worker DAG service. It is independent of the root Cargo
workspace and does not register a Molten system extension. Its named deployment
profile is `standalone-single-controller-import-free-component-batch-v1`.

The immediate consumer is an operator running bounded, import-free component DAGs
with a separate controller/worker database and explicit peer enrollment. The
durable contribution is a repeatable pilot with pure allocation and DAG cores,
application-owned ports, real Redb/Iroh/Wasmtime adapters, a lockfile, an operator
CLI and a multi-process exercise. Molten contributors own maintenance under
`pilots/orchestration`.

## Provenance and repairs

[The provenance record](../pilots/orchestration/source.ncl) identifies the reviewed
repository-root candidate archive by BLAKE3, its member prefix, the original Molten
base `bb6f3830ee7327da9875ea85a8c8e25697eddc35`, the 2026-09-09 independent review,
and every adoption repair. Imported Rust source carries AGPL-3.0-or-later headers;
dependencies retain their upstream terms.

<!-- r[verify molten.batch_orchestration.repairs] -->

- `adapters/src/storage.rs` imports `redb::ReadableDatabase` for read transactions
  and propagates failures configuring immediate durability before table mutation.
  Storage tests cover canonical state/content reopen, CAS conflict and exclusive
  process ownership.
- `adapters/src/root.rs` opens the held directory with real read flags before
  synchronization instead of passing a cap-std `O_PATH` descriptor to `fsync`.
  `adapters/tests/root.rs` exercises create/synchronize/replace after the directory
  is renamed, rejects a create-only overwrite and checks the replacement stayed
  beneath the held capability. Existing storage tests reject traversal and a
  symlinked database.
- `Resources::checked_add` retains checked CPU, memory and slot arithmetic; all
  callers use the fallible name.
- `Command::Schedule` borrows its transient request. This removes an oversized
  command variant and a redundant request clone without introducing a heap box.

## Repeat the exercised path

<!-- r[verify molten.batch_orchestration.validation] -->

From the repository root, use its pinned Nix development shell:

```sh
CLIPPY_STRICT=1 nix develop -c bash pilots/orchestration/scripts/verify.sh
nix develop -c bash pilots/orchestration/scripts/demo.sh
```

The first command checks the dependency boundary, formats source, runs all
standalone workspace targets, runs warnings-denied Clippy and builds the release
CLI. It retains `pilots/orchestration/target/validation/verify.log`. The committed
standalone `Cargo.lock` resolves public registry dependencies separately from
Molten's root lockfile. The toolchain is `nightly-2026-05-26`.

The demo launches real controller and worker processes, submits a two-task DAG,
fetches the final object and requires exactly `b"ok"`. It retains its private run
directory and stops only its own children. `CARGO_TARGET_DIR` is supported for the
release executable; set `TMPDIR` to a filesystem with space for state and logs.
Do not publish `identity.key`, private deployment roots or database contents as
public evidence. The separate CLI integration test uses two workers and exercises
restart persistence, duplicate submission, unenrolled-peer denial and cancellation.

## Historical scheduler patch

<!-- r[verify molten.batch_orchestration.reference] -->

The source archive retains its original guarded `apply.py`. Its default invocation
is the dry-run check (there is no `--check` argument). It validates the exact HEAD,
original Git blob identities, a clean worktree and absence of overlay destinations
before generating a patch. Apply and test it only in a fresh worktree at
`bb6f3830ee7327da9875ea85a8c8e25697eddc35`, never over a moved mainline checkout.

Current mainline uses an assignment token binding job, owner and lease epoch in
`crates/molten-core/src/fabric_simulation/reference.rs`. The historical overlay's
`lease_epoch` command field is not reapplied or substituted for that contract.
The adoption's historical test execution and moved-checkout refusal are recorded
with its Cairn evidence; passing the standalone workspace alone does not validate
that scheduler patch.

## Native integration remains separate

<!-- r[impl molten.batch_orchestration.integration] -->

[The port integration plan](../pilots/orchestration/docs/MOLTEN-INTEGRATION.md)
names each owner, the pilot port, the integration law and the required follow-on
gate for lifecycle, placement, reservation persistence, coordination delivery,
content identity, admission, execution/supervision and observability. There is no
port rewiring in this adoption. Root workspace membership, dispatcher, ALPN
registries, system-tier manifests and Basalt/Cairn admission remain unchanged.

Operator policy is not production admission. Same-host evidence does not qualify
cross-host networking, controller HA, production isolation, valuable workloads or
exactly-once external effects. No fixture proves Wasmtime, component, worker,
transport or whole-system correctness, or release eligibility.
