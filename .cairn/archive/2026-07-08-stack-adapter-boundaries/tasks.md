## Tasks

- [x] [serial] Defined the Molten stack evidence envelope contract and role vocabulary. r[molten.evidence.stack_adapters.envelope]
- [x] [serial] Added positive fixtures for complete Basalt, UCAN, Trellis, Octet, Valence, Cairn, and Mantle evidence refs. r[molten.evidence.stack_adapters.envelope.positive]
- [x] [serial] Added negative fixtures for missing roles, stale refs, unsupported schemas, and overbroad claims. r[molten.evidence.stack_adapters.envelope.negative]
- [x] [serial] Added adapter-port documentation and marked approved modules for upstream-specific crate usage. r[molten.evidence.stack_adapters.ports]
- [x] [serial] Added an initial dependency-boundary diagnostic for stack-owned crates used outside approved adapters. r[molten.evidence.stack_adapters.dependency_boundary]
- [x] [serial] Ran focused contract checks, runtime fixture checks, and Cairn validation/gates for this change. r[molten.evidence.stack_adapters.validation]

Implementation evidence: `docs/stack-evidence-envelope/valid.ncl` exports; `missing-role.ncl`, `stale-ref.ncl`, and `overbroad-claim.ncl` fail closed; `molten_core::stack::validate_stack_evidence_envelope` has positive/negative role, schema, ref, and non-claim tests; `docs/dependency-boundary-policy/valid.ncl` records approved adapter boundaries. Checks passed: `cargo test -p molten-core`, `cargo test --lib`, Nickel fixtures, pre-commit, and Cairn validation.
