# Resource lifecycle public count admission

## Implemented and observed

- [x] [serial] Map public placement/completion boundaries, existing 64/128 constants, internal test-only callsites, and separate target-property sizing gap. r[molten.resources.placement_constraint_preflight] r[molten.resources.reconcile_required_effect_intent_preflight]
- [x] [serial] Reproduce one-past-limit placement and effect-intent failures before editing production source, with exact-bound and admitted-invalid precedence cases already passing. r[molten.resources.placement_constraint_preflight] r[molten.resources.reconcile_required_effect_intent_preflight]
- [x] [serial] Preflight both caller counts before diagnostic work while preserving decision structs, count labels, and admitted-input ordering; pass the two focused red-to-green regressions. r[molten.resources.placement_constraint_preflight] r[molten.resources.reconcile_required_effect_intent_preflight]
- [x] [serial] Compile and run a disposable external `molten::supplies` caller showing 64/128 pass, 65/129 deny, and admitted-invalid errors; remove temporary source afterward. r[molten.resources.placement_constraint_preflight] r[molten.resources.reconcile_required_effect_intent_preflight]
- [x] [serial] Compose disjoint helper/source/test/docs/Cairn changes atop 64 packages without overwriting pre-existing resource test edits; verify changed behavior. r[molten.resources.placement_constraint_preflight] r[molten.resources.reconcile_required_effect_intent_preflight]
- [x] [serial] Run scoped formatting, Clippy, library Octet, and strict Cairn in owner and composition; record warning-only/full-gate limits and the unbounded target-property contract. r[molten.resources.placement_constraint_preflight] r[molten.resources.reconcile_required_effect_intent_preflight] r[molten.resources.placement_target_property_contract_missing]
