## Tasks

- [x] [serial] r[molten.modularity.adapter_ports.explicit_ports] Identified the evidence-backed runtime operation workflow in `docs/modularity-boundaries.md` and defined `molten_core::planning::EffectPlan` as the narrow port/plan boundary.
- [x] [serial] r[molten.modularity.adapter_ports.admission_before_effects] Extracted pure planners (`plan_adapter_effects`, `plan_store_write`, `plan_retention_gc`, `plan_job_execution`, `plan_node_enqueue`) so denied decisions return no mutation, transport, execution, or clock-dependent operation.
- [x] [serial] r[molten.modularity.adapter_ports.effect_receipts] Modeled receipt-writing as an explicit planned effect and documented shell responsibility for canonical evidence for pass, deny, unavailable, and replay outcomes.
- [x] [parallel] r[molten.modularity.adapter_ports.tests] Added positive and negative `molten-core` planner tests covering admitted plans, missing authority, stale evidence/resource denial paths, malformed values, and unsupported adapter capabilities.
- [x] [serial] r[molten.modularity.adapter_ports.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, Nickel fixture checks, pre-commit, and Cairn validation.
