# DAG fabric admission binding — verification

## Accepted dispatch

The federation live loopback and distinct-process rails route the supervised callback through
`run_dag_sync` under the admitted descriptor and commit only after verification
(`../federated-pull-system-extension/evidence/2026-10-30-live-rails.md`).

## Denials

- `mesh::liveproof::bindingtests::federation_manifest_requires_the_owner_admitted_dag_binding`:
  an unadmitted binding ref and the transport binding ref both fail manifest admission; the exact
  admitted DAG binding admits.
- `mesh::liveproof::bindingtests::supervised_dag_effect_denies_stale_generation_and_foreign_request`.
- `dag_sync::tests::resource_denial_fails_before_any_transfer`: no fetch and no persisted progress.
- `dag_sync::tests::cancellation_corruption_and_authority_denial_fail_before_false_completion`.
- Live `corrupt-content` rails deny before registry mutation.

## Conformance input

`2026-10-30-dag-owner-conformance-run.txt` (BLAKE3
`56234be682d161db5f7c35d66ac420a419562a56428efa3fc472794c0ebc4249`) is the executed DAG owner
suite transcript used as the conformance reference for the live rails. It is
integration-owner-reviewed and explicitly experimental, not independent owner approval.

## Integrated source checks (executed 2026-10-30)

Run in the integration worktree `cairn/finish-open-changes-20261029` (base
`d7e435e711f7f58167900ce7419121390b0842d3`) through `nix develop` (`nightly-2026-05-26`):

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p molten --all-targets --features spacewasm-pilot -- -D warnings` | exit 0 |
| `cargo clippy -p molten --all-targets --features component-rpc-telemetry -- -D warnings` | exit 0 |
| `cargo octet check --artifact-dir …` | `Status: clean`, 0 findings |
| `cargo octet check -p molten --artifact-dir … -- --lib` | `Status: clean`, 0 findings |
| `cargo test --workspace --no-fail-fast` | 2,907 passed, 0 failed, 28 ignored |
| `nix build .#checks.x86_64-linux.inherited-tracey-debt` | pass, `dangling=0` |
| `cairn validate --root . --policy ../cairn/cairn-policy/generated/cairn-policy.json --strict` | `valid: true` |

The pinned Octet producer used by the startup source binding (cargo-octet 0.1.0 on
`nightly-2026-03-21`, Nix cohort) also reported `Status: clean` for the primary and library
cohorts. The 28 ignored tests require independently provisioned signed producer outputs or
external cohorts and are reported only where they were explicitly executed.

## Non-claims

A binding grants no peer, resource, object, registry or merge authority.
