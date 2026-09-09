# Tasks

## Baseline and output contract

- [ ] [serial] Run current world-merge core and publication tests before core edits and record the exact baseline. r[molten.audit_f13.validation]
- [ ] [serial] Reproduce equal absent inputs with and without schema metadata through public planning and publication. r[molten.audit_f13.absence] r[molten.audit_f13.validation]
- [ ] [serial] Inventory output constructors, canonical encoders, readers, handlers, migrations, and publication matches before selecting the closed output representation. r[molten.audit_f13.output_kind] r[molten.audit_f13.compatibility]

## Implementation and regression cases

- [ ] [serial] Implement pure absent, selected, and generated output cases with profile admission and impossible-state rejection. r[molten.audit_f13.output_kind]
- [ ] [serial] Update the publication shell so admitted absence omits the root without generated writes or deletion effects. r[molten.audit_f13.absence]
- [ ] [parallel] Add positive absent, selected, generated-empty, and mixed-output fixtures with exact port-call assertions. r[molten.audit_f13.validation]
- [ ] [parallel] Add negative contradictory output, unsupported absence, missing generated schema, wrong identity, and failed persistence cases with unchanged heads. r[molten.audit_f13.output_kind] r[molten.audit_f13.validation]

## Compatibility and checks

- [ ] [serial] Bind output kind into canonical plan identity and implement explicit legacy rejection or re-planning with round-trip tests. r[molten.audit_f13.compatibility]
- [ ] [serial] Update world-merge documentation and run combined migration/output fixtures without weakening admission or retention boundaries. r[molten.audit_f13.absence] r[molten.audit_f13.validation]
- [ ] [serial] Rerun focused tests, workspace tests, Clippy with denied warnings, scoped strict Octet, relevant Nix checks, and Cairn validation and gates. Retain exact results and blockers. r[molten.audit_f13.validation]
