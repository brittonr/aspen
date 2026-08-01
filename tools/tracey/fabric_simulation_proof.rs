// Trace-only coverage anchors for deterministic whole-system simulation.
//
// Product implementation lives in `src/fabric_simulation/` and
// `crates/molten-core/src/fabric_simulation/`. Positive and negative validation
// lives beside those modules and in `tests/fabric_simulation_boundary.rs`.
// Cairn Tracey currently scans `tools/` and `crates/`, but not top-level `src/`.

// r[impl molten.fabric_simulation.world_manifest]
// r[verify molten.fabric_simulation.world_manifest]
// r[impl molten.fabric_simulation.same_core]
// r[verify molten.fabric_simulation.same_core]
// r[impl molten.fabric_simulation.port_substitution]
// r[verify molten.fabric_simulation.port_substitution]
// r[impl molten.fabric_simulation.scheduler]
// r[verify molten.fabric_simulation.scheduler]
// r[impl molten.fabric_simulation.fault_model]
// r[verify molten.fabric_simulation.fault_model]
// r[impl molten.fabric_simulation.invariants]
// r[verify molten.fabric_simulation.invariants]
// r[impl molten.fabric_simulation.replay_shrink]
// r[verify molten.fabric_simulation.replay_shrink]
// r[impl molten.fabric_simulation.live_sim_differential]
// r[verify molten.fabric_simulation.live_sim_differential]
// r[impl molten.fabric_simulation.reference_services]
// r[verify molten.fabric_simulation.reference_services]
// r[impl molten.fabric_simulation.fabric_sufficiency]
// r[verify molten.fabric_simulation.fabric_sufficiency]
// r[impl molten.fabric_simulation.claim_ladder]
// r[verify molten.fabric_simulation.claim_ladder]
// r[impl molten.fabric_simulation.evidence]
// r[verify molten.fabric_simulation.evidence]
// r[impl molten.fabric_simulation.operator_workflow]
// r[verify molten.fabric_simulation.operator_workflow]
// r[impl molten.fabric_simulation.final_validation]
// r[verify molten.fabric_simulation.final_validation]
