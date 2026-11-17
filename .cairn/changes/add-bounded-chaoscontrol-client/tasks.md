## Client boundary

- [x] [serial] Implement versioned finite input, canonical retention and prerequisite-valid prefix reduction. r[molten.chaoscontrol_client.bounded_input]
- [x] [serial] Connect WorkloadExecutor to existing admitted native multi-process Raft operations and independent observations. r[molten.chaoscontrol_client.real_execution]
- [x] [serial] Preserve storage uncertainty, explicit recovery provenance and per-participant fault-off horizons. r[molten.chaoscontrol_client.observation]
- [x] [serial] Add healthy, reached duplicate-application, property-suppressed, no-op, missing-participant, unreached-fault and incomplete controls. r[molten.chaoscontrol_client.controls]
- [x] [serial] Document supported guest dependency handoff and exact unavailable prerequisites. r[molten.chaoscontrol_client.guest_boundary]

## Evidence and closeout

- [x] [serial] Execute focused native tests, retain original-build independent repeat and validate source/lifecycle policy. r[molten.chaoscontrol_client.verification]
- [ ] [depends:chaoscontrol-molten-cohort] Produce the compatible measured guest closure and obtain independent campaign/replay qualification through ChaosControl #19. r[molten.chaoscontrol_client.guest_boundary]

No execution, replay, qualification or closure evidence is claimed by unchecked tasks.

## Parent native observation (2026-11-13)

The bounded-client selection passed 15 tests. A separate retention-enabled
native run passed all six native tests, including healthy recovery, reached
duplicate application, required-property suppression, fresh-process original
repeat and actual valid-prefix reduction. `evidence/native-2026-11-13/` retains
the resulting canonical inputs, owner observations, fault/horizon markers and
checks. The source-revision environment inputs were unavailable for this local
run; these artifacts do not establish immutable-source or guest qualification.
The missing measured guest, persistent-device/transport cohort and independent
ChaosControl campaign/replay acceptance remain open and are not archived.

## Immutable native evidence reconciliation

The later [2026-11-15 invocation](evidence/native-2026-11-15/invocation.json)
and [result log](evidence/native-2026-11-15/run.log) retain six passing native
tests from the exact Nix-built executable, including the independent
fresh-process original repeat and valid-prefix reduction. They supersede the
earlier missing-source-context statement for that native build only.

The [offline reconciliation](evidence/native-2026-11-15/reconciliation.json)
remeasured the original source NAR and executable and matched their recorded
BLAKE3 identities. Original/repeat build bindings, canonical input and oracle
bytes are identical. This records existing execution plus new identity checks,
not a new test run or qualification of later builds. No retained canonical
execution bytes were changed.

The guest-closure and independent ChaosControl campaign/replay task remains
unchecked. Native process-crash evidence cannot satisfy that boundary; the
change remains active rather than archived.

