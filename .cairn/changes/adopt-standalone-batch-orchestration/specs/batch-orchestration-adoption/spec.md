# Batch Orchestration Adoption Specification

## ADDED Requirements

### Requirement: Vendored pilot source carries exact provenance

r[molten.batch_orchestration.vendor] The repository MUST vendor the reviewed
orchestration workspace under `pilots/orchestration` as repository-owned source with a
provenance record naming the candidate bundle, the base commit
`bb6f3830ee7327da9875ea85a8c8e25697eddc35`, the independent review date, and every
applied repair, plus a committed resolved `Cargo.lock`.

#### Scenario: Provenance is complete

- GIVEN the vendored tree and its provenance record
- WHEN adoption review inspects the source identity
- THEN the bundle origin, base commit, review date, and full repair list MUST be
  reconstructible from repository-owned files.

#### Scenario: A lockfile is missing or unreconciled

- GIVEN the workspace builds without a committed `Cargo.lock` or with an unreconciled
  one
- WHEN adoption validation runs
- THEN the change MUST NOT complete.

### Requirement: Review repairs land with regression coverage

r[molten.batch_orchestration.repairs] The vendored source MUST include the three
review repairs: the `redb::ReadableDatabase` read-transaction import, a capability-root
directory durability sync that does not `fsync` an `O_PATH` descriptor, and
strict-Clippy conformance across all seven crates; each behavioral repair MUST retain a
regression fixture that fails under the previous implementation.

#### Scenario: Durability sync works on Linux

- GIVEN a worker or controller store opens through `CapabilityRoot`
- WHEN the directory sync executes on a Linux filesystem
- THEN the sync MUST succeed or fail with a real I/O error, never `EBADF` from an
  `O_PATH` descriptor.

#### Scenario: A repair loses its regression fixture

- GIVEN the strict-Clippy run passes
- WHEN the repair review checks the durability and import repairs
- THEN each MUST show a fixture that fails against the pre-repair behavior.

### Requirement: In-tree validation reproduces the independent evidence

r[molten.batch_orchestration.validation] The vendored workspace MUST pass workspace
tests over all targets, `CLIPPY_STRICT=1` Clippy, the release build of the operator
CLI, and the two-process end-to-end demo inside this repository, with logs, toolchain
versions, and the source revision retained as evidence.

#### Scenario: Full verification passes in-tree

- GIVEN the vendored workspace in this repository
- WHEN `scripts/verify.sh` runs with `CLIPPY_STRICT=1` and then `scripts/demo.sh` runs
- THEN every test, lint, build, and the DAG demo MUST pass with recorded evidence.

#### Scenario: A test fails after vendoring

- GIVEN any workspace test, lint, build, or demo step fails in-tree
- WHEN adoption validation runs
- THEN the change MUST treat the failure as a defect to resolve and MUST NOT complete
  with weakened assertions.

### Requirement: Reference scheduler conformance runs at the pinned base

r[molten.batch_orchestration.reference] The `lease_epoch` reference scheduler patch
MUST apply only through the guarded installer at the exact base commit in a clean
worktree, MUST pass `scripts/reference-tests.sh` with repository dependencies, and MUST
NOT proceed by removing installer guards on a moved checkout.

#### Scenario: The base commit matches

- GIVEN a clean worktree at `bb6f3830ee7327da9875ea85a8c8e25697eddc35`
- WHEN `apply.py --check`, `git apply`, and `scripts/reference-tests.sh` run
- THEN the patch MUST apply cleanly and every reference regression MUST pass with
  repository dependencies available.

#### Scenario: The checkout has moved

- GIVEN the target branch no longer matches the pinned base
- WHEN the guarded installer runs
- THEN installation MUST be refused and reconciliation MUST be a deliberate explicit
  step, never a guard removal.

### Requirement: The pilot stays outside the admitted system

r[molten.batch_orchestration.boundary] The vendored pilot MUST NOT register in
`SystemExtensionHost`, join the root Cargo workspace, change ALPN registries or
system-tier manifests, consume Basalt/Cairn production admission, or emit invented
admission receipts; its operator policy MUST be recorded as a distinct named standalone
deployment profile with its own database.

#### Scenario: The pilot runs as designed

- GIVEN the vendored controller, worker, and client processes
- WHEN the demo or integration tests execute
- THEN all admitted-system surfaces MUST remain unchanged in the diff.

#### Scenario: An implicit fallback appears

- GIVEN a proposal, patch, or configuration that treats the pilot as a substitute for
  Molten placement, admission, delivery, or execution authority
- WHEN adoption review runs
- THEN the change MUST be denied as a boundary violation.

### Requirement: Native integration is planned with explicit ports and non-claims

r[molten.batch_orchestration.integration] The change MUST deliver an integration plan
mapping the pilot ports to Molten owners (placement and membership, admission policy,
content identity translation, coordination delivery, execution and supervision,
observability) with explicit follow-on gating and the delivery's own non-claims, and
MUST NOT implement any port rewiring in this change.

#### Scenario: The integration plan exists

- GIVEN the vendored `docs/MOLTEN-INTEGRATION.md` mapping
- WHEN adoption review inspects the plan
- THEN every Molten owner-to-port mapping MUST appear with its integration law and its
  follow-on gate, and no port rewiring MAY appear in this change's diff.

#### Scenario: Integration is attempted early

- GIVEN a task that rewires a Molten port to the pilot within this change
- WHEN validation runs
- THEN the change MUST fail as out of scope.
