# Tasks: Node Control Supervisor Policy

## Phase 1: Policy and receipts

- [x] [serial] r[molten.node_control_supervisor_policy.spec.policy_artifact] Add canonical supervisor policy artifacts and ledger classification.
- [x] [serial] r[molten.node_control_supervisor_policy.spec.supervisor_receipts] Add canonical supervisor receipts and bind refs into service-run receipts.

## Phase 2: CLI workflow

- [x] [parallel] r[molten.node_control_supervisor_policy.spec.policy_cli] Add `molten node supervisor-policy-fixture` and `serve --supervisor-policy`.

## Phase 3: Runner gates

- [x] [serial] r[molten.node_control_supervisor_policy.spec.stale_lock_gate] Fail closed on stale service locks unless policy admits recovery.
- [x] [serial] r[molten.node_control_supervisor_policy.spec.restart_bounds] Enforce bounded restart attempts before service-run side effects.
- [x] [serial] r[molten.node_control_supervisor_policy.spec.shutdown_drain] Emit shutdown-drain receipts and deny over-bound drains.
- [x] [parallel] r[molten.node_control_supervisor_policy.spec.not_authority] Keep supervisor policy separate from operation authority and provenance gates.
- [x] [parallel] r[molten.node_control_supervisor_policy.spec.fail_closed] Add unit and CLI coverage for recovery, duplicate denial, restart bounds, and shutdown drain outcomes.
