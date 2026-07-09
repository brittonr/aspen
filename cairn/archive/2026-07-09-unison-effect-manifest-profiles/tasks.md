## Tasks

- [x] [serial] r[molten.effects.ability_manifest_boundary] Extend effect manifest handling so executable artifacts declare effect ids, operations, schemas, resource classes, capability needs, policy refs, and evidence refs before execution.
- [x] [serial] r[molten.effects.handler_profile_admission] Define handler-profile admission receipts for production, local, chaos, profiling, and replay handlers with manifest compatibility checks.
- [x] [parallel] r[molten.effects.undeclared_effect_denial] Deny undeclared effect requests, wrong operation schemas, profile mismatches, missing capabilities, and stale profile receipts before side effects.
- [x] [parallel] r[molten.effects.profile_replay_binding] Bind exact effect manifest and handler profile refs into replay, transcript, evaluation-cache, job DAG, and remote execution receipts.
- [x] [serial] r[molten.effects.unison_adaptation_validation] Add positive and negative fixtures proving declared effects pass, undeclared effects deny, profile mismatches deny, and Unison compatibility is not claimed.