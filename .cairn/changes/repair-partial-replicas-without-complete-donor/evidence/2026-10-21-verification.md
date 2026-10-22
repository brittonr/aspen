# Repair partial replicas without a complete donor: verification

## What changed

- `crates/molten-core/src/content_replication/repair/mod.rs`: the pure closure repair planner. It takes a
  required-closure set and a bounded inventory of per-replica, per-object verification facts, admits donor
  eligibility per object, and emits a bounded plan of per-object transfers with a completeness decision.
  `RepairCompleteness` is `complete` when every required object has an intact verifiable copy and `incomplete`
  with the unrecoverable identities otherwise. The incomplete plan still carries the transfers for the objects
  that do have donors. `verify_assembled_object` verifies an assembled object against its established identity and
  rejects that object alone. `verify_transfer_donor` refuses a donor that flapped between the plan and the
  transfer. The plan carries the bytes-not-authority and no-current-placement non-claims.
- `crates/molten-core/src/content_replication/identity.rs`: the domain-separated identity of a repair plan binds
  the required closure, the chosen donors, and the completeness decision.
- `crates/molten-core/src/content_replication/tests/closure.rs`: the multi-partial-donor closure scenario and the
  negative fixtures.
- `src/content_replication/service/mod.rs`: the reconcile shell now runs the closure repair planner over the
  observed inventory, with the manifest content set as the required closure and the inventory replicas as the
  per-replica, per-object verification facts.
- `src/content_replication/model.rs`: the operator status view carries the repair completeness, the unrecoverable
  identities, and the repair non-claims, and the reconcile outcome carries the repair plan.
- `src/content_replication/records.rs`: the canonical operator status record moves to
  `content-replication-operator-status-v2` and carries the repair completeness, the unrecoverable identities, and
  the repair non-claims. A view without the bytes-not-authority claim, with a completeness outside the closed
  vocabulary, or with an incomplete repair that names nothing is refused.
- `src/content_replication/tests/repair_status.rs`: the operator status fixtures for a complete repair, an
  incomplete repair that names the unrecoverable identity, and the refused views.

## Validation

- `cargo test -p molten-core --lib content_replication`: the closure fixtures pass and the existing replication
  baseline, including the F04 stale-success regression, keeps passing unchanged.
- `cargo test -p molten --lib content_replication`: 31 passed. The operator status fixtures cover a complete repair,
  an incomplete repair that names the unrecoverable identity, and the refused views.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test --workspace` with a fresh `CARGO_TARGET_DIR`: exit 0. The molten library suite reported 1,688
  passed, the core suite 497 passed, the native system-extension integration suite 9 passed, and the doc-test phase
  passed.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0 errors, with
  `cargo-octet 0.1.0` on `nightly-2026-03-21`.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings, `spec_findings: []` and
  `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls `cargo metadata --all-features`, and that
  call panics in the pinned cargo on the optional `rad://` sources. The panic reproduces on the unmodified tree and is
  unrelated to this change, which touches no manifest. The documented Cargo fallback was used.

## Deferred scope

None. The shell runs the closure repair planner and the operator status carries the completeness field, the
unrecoverable identities, and the bytes-not-authority non-claim.

## Non-claims

- Repair operates at the canonical-content boundary. A valid content hash proves which bytes were received; it proves
  nothing about which head is current, and it grants no capacity, authority, or placement.
- The repair creates no new content identity and recovers bytes matching known identities only.
- The repair plan is bounded by the admitted inventory and transfer limits. An unbounded inventory is refused.
