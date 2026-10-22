# Publish withdrawable service demand: verification

## What changed

Service demand now has a lifetime. The demand record, the demand set derived from the live
dataspace assertions, and the demand-derived predicate input live here:

- `src/runtime/dataspace/parts/state/p004/body.rs`: the `ServiceDemandAssertion` record carries the
  service ref, the demand form (`dependency-gated` or `force-run`), the demander ref, and the operation
  identity. `ServiceDemandSet::derive` reads the live assertions, ignores assertions that are not demand
  records, fails closed on a malformed demand record, and collapses duplicates per owner and service.
  The set answers the derived facts: the demanded services, the force-run declarations, the demanding
  scopes per service, the live demand count, and whether the last live demand is withdrawn.
- `src/runtime/predicates/parts/mod/p002/body.rs`: `RuntimeServiceDependenciesState` gains the
  `live_demand_demanders` fact and the `with_demand_assertions` constructor, which fills the demanded
  services, the force-run declarations, and the live demanders of the service from the demand set. The
  predicate stays pure over the derived facts. The state's canonical input record moves to
  `runtime-service-dependencies-state-v2` because it now binds the live demanders.
- `src/runtime/predicates/parts/mod/p004/body.rs`: a shutdown with a live demander is denied with the
  `service-shutdown-with-live-demand` diagnostic, and the receipt records the
  `shutdown-after-last-demand-withdrawal` check.

The demand is coordination state, not authority. It cannot start work by itself, and it proves no
provider exists.

## Validation

- `cargo test -p molten --lib demand_`: 11 passed, 0 failed. The fixtures cover the demand record
  round trip, the owner-scoped deduplication, the second demander keeping the service alive, the
  withdrawn last demand making shutdown eligible, the explicit force-run declaration, the malformed
  demand record failing closed, the restart message creating no demand state, the dependency-gated
  demand waiting for readiness and then starting, the force-run demand bypassing the order by
  declaration, the withdrawal allowing shutdown while another demander prevents it, and the unverified
  provider never satisfying demand.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test --workspace` with a fresh `CARGO_TARGET_DIR`: exit 0. The molten library suite reported 1,675 passed,
  the core suite 492 passed, the native system-extension integration suite 9 passed, and the doc-test phase passed. The
  fresh target directory matters here: the Octet producer runs under its own pinned toolchain in the shared target
  directory, and the doc-test build then cannot load the externs it finds there. That is a target-directory
  interaction, not a source defect.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0 errors, with
  `cargo-octet 0.1.0` on `nightly-2026-03-21`. The first runs reported findings in the new code: 18 on the first pass
  (unwraps in production code, unbounded collection growth, a platform-dependent public size, and non-trait imports) and
  4 on the second. The demand value is now fallible, the collections are bounded, the count is a saturating counter, and
  the re-run is clean.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings, `spec_findings: []` and
  `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls `cargo metadata --all-features`, and that
  call panics in the pinned cargo on the optional `rad://` sources. The panic reproduces on the unmodified tree and is
  unrelated to this change, which touches no manifest. The documented Cargo fallback was used.

## Non-claims

- Demand is not authority and not a capacity signal. Nothing infers capacity from concurrency, no
  placement decision follows from a demand record, and no start is claimed to happen exactly once.
- The predicate remains pure. It receives the demand facts derived from the assertion set and never
  reads ambient state.
- A restart request stays a message. It changes retry and restart state and never becomes demand.
