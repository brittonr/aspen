# Tasks: Route explicitly admitted installed native effects

## Governance

- [x] [serial] Register the separately approved proposal/design/specification, concrete current collaboration consumer, ownership and dependency order. r[molten.system_extension.native_effect_installation] r[molten.system_extension.native_effect_consumer]

## Concurrent source slices

The following three source slices may proceed concurrently against the fixed `NativeEffectRouter` and domain `native_runtime::{admit,Runtime::open}` contracts. Shared host mechanisms have one owner; shared loader/effect configuration has another; scope factory/application code remains domain-owned.

- [x] [parallel] Implement bounded canonical exact effect configuration and independent verifier-backed current installation/route authority, without source-grant reuse or raw reference authority. r[molten.system_extension.native_effect_installation] r[molten.system_extension.native_effect_authority]
- [x] [parallel] Implement explicit NativeRuntime router registration, actual Supervisor route/completion calls, verified completion inventory and Unknown preservation without clearing approved effects. r[molten.system_extension.native_effect_completion] r[molten.system_extension.native_effect_uncertainty]
- [ ] [parallel] Complete the concrete scope factory, fresh independent scope authorization, actual bounded retained Raft/application worker and committed result schema, with no queue-as-commit fallback. r[molten.system_extension.native_effect_authority] r[molten.system_extension.native_effect_completion] r[molten.system_extension.native_effect_lifetime]

## Serial source integration and current consumer

- [x] [serial] Compose the public loader/InstalledHost with the concrete domain runtime interface, retain exact current grants and actual active installation checks, and expose the retained owner operation path without reopening locked stores. r[molten.system_extension.native_effect_installation] r[molten.system_extension.native_effect_lifetime] r[molten.system_extension.native_effect_consumer] r[molten.system_extension.native_effect_uncertainty]
- [ ] [serial] Finish the installed native collaboration fixture proving genuine callback → actual Raft proposal → committed scope state → materialized native completion callback, plus denied/uncommitted/post-effect-failure cases. r[molten.system_extension.native_effect_consumer] r[molten.system_extension.native_effect_completion] r[molten.system_extension.native_effect_uncertainty]

## Parent-owned validation and protected operations

- [ ] [serial] Run the coherent integrated source build, actual installed-consumer baseline/negative/process-loss proof and retain canonical observed evidence from the genuine signed source/admission cohort. r[molten.system_extension.native_effect_consumer] r[molten.system_extension.native_effect_uncertainty] r[molten.system_extension.native_effect_lifetime]
- [ ] [serial] Run required formatting/source-policy/review/strict lifecycle validation before any specification synchronization, archive, commit or publication. r[molten.system_extension.native_effect_installation] r[molten.system_extension.native_effect_authority] r[molten.system_extension.native_effect_completion]

Checked source tasks record implementation, not successful validation or executed acceptance. Scope worker/fixture integration and every execution/protected gate remain open until their owning work and observed proof finish. No build, test, linter, formatter, gate, synchronization, archive, commit or publication was run by this worker. Signed source artifact production is parent-owned; its absence does not prevent source implementation and never permits fabricated evidence.
