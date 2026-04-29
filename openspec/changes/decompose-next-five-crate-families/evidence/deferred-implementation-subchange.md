# Deferred Implementation Subchange

Parent tasks I3 through I17 and V1 through V4 are deferred to `decompose-next-five-crate-families-implementation`.

Reason: those tasks require policy/checker implementation, crate boundary changes, downstream fixtures, negative mutation suites, compatibility compile/test rails, and positive/negative behavior tests across five crate families. That scope exceeds the local drain budget for a single parent task and matches the parent design risk mitigation: the parent selects/constrains the wave, while each family can become its own implementation slice.

Subchange artifact:

- `openspec/changes/decompose-next-five-crate-families-implementation/proposal.md`

Deferred scope includes:

- policy/checker entries and negative mutation evidence for selected families;
- foundational storage/trait first blockers;
- auth/ticket consumer migration and runtime API negative checks;
- jobs/CI reusable scheduler/config/run-state split and fixtures;
- trust/crypto/secrets pure-core extraction and tests;
- testing-harness reusable helper split and fixtures;
- downstream, compatibility, and positive/negative verification evidence.
