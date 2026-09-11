# Tasks: Review the caveat attenuation boundary

## Inventory

- [ ] [serial] Inventory every Molten attenuation site with its required narrowing, and classify each as a scope restriction or a payload filter. r[molten.runtime_spine.caveat_authority_boundary]
- [ ] [serial] Read the pinned UCAN and Basalt sources and record which caveat forms exist, whether they express a bounded filter over Preserves payloads, and what chain verification requires. r[molten.runtime_spine.caveat_authority_boundary]

## Decision

- [ ] [serial] Write the decision note under `docs/` that names the chosen outcome: consume the stack authority format, admit a bounded Molten filter core, or add nothing. r[molten.runtime_spine.caveat_authority_boundary]
- [ ] [serial] For an admitted Molten-owned form, fix the guard rails in the note: reverse-order evaluation with a written direction, unknown caveat rejects all inputs, unbound template references fail validation, and attenuation only narrows. r[molten.runtime_spine.caveat_authority_boundary]
- [ ] [parallel] Cut the follow-up change package for the chosen implementation path, or record why no follow-up is needed. r[molten.runtime_spine.caveat_authority_boundary]

## Verification

- [ ] [serial] Cross-link the decision note from `docs/synit-incorporation-review.md` and confirm that no source change in this package adds a caveat or filter type. r[molten.runtime_spine.caveat_authority_boundary]
- [ ] [serial] Run the Cairn gates and the repository validation command against the change package. r[molten.runtime_spine.caveat_authority_boundary]
