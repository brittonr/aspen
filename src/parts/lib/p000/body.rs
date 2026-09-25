
macro_rules! compat_module {
    ($name:ident, $target:ident) => {
        pub mod $name {
            pub use super::$target::*;
        }
    };
}
compat_module!(ast_grep_runtime_authority_audits, ast_grep_runtime_authority_core);
compat_module!(artifacts, objects);
compat_module!(authority, delegation);
compat_module!(catalog, inventory);
compat_module!(catalog_mcp, inventory_api);
compat_module!(chunk_store, blocks);
compat_module!(capability_tokens, capabilities_core);
compat_module!(coordination, orchestration);
compat_module!(delivery_idempotency, dedupe);
compat_module!(deterministic_replay, playback);
compat_module!(effects, actions);
compat_module!(error, failures);
compat_module!(eval_cache, memo);
compat_module!(evidence, proofs);
compat_module!(evidence_chain, lineage);
compat_module!(federation, mesh);
compat_module!(harness, testbed);
compat_module!(iroh_exchange, netlink);
compat_module!(job_dag, workload);
compat_module!(ledger, journal);
compat_module!(lifecycle, phases);
compat_module!(nixos_vm, machine);
compat_module!(node_daemon, daemon_core);
compat_module!(node_identity, credential);
compat_module!(node_iroh, transport);
compat_module!(node_runtime, kernel);
compat_module!(node_profile_config, node_profile_config_core);
compat_module!(node_service_fsm, node_service_fsm_core);
compat_module!(octet_gate, quality);
compat_module!(octet_remediation, remediator);
compat_module!(operator_context_profile, context_profile_core);
compat_module!(operator_dogfood, pilot);
compat_module!(operator_gateway, edgeway);
compat_module!(peer_bootstrap, peering);
compat_module!(plugin_host, extension);
compat_module!(preserves_rail, codec);
compat_module!(prod_readiness, launch);
compat_module!(prod_release_profile, release_profile_core);
compat_module!(external_live_pilot, pilot_readiness);
compat_module!(prod_soak, burnin);
compat_module!(protocol_session, conversation);
compat_module!(sans_io_protocol, sans_io_protocol_core);
compat_module!(provenance, lineage_meta);
compat_module!(eventual_surface, propagation_core);
compat_module!(raft_control_plane, quorum);
compat_module!(raft_membership, raft_membership_core);
compat_module!(remote_dataspace, meshspace);
compat_module!(resources, supplies);
compat_module!(retention, custody);
compat_module!(rewrites, transforms);
compat_module!(runtime, engine);
compat_module!(schema_identity, descriptor);
compat_module!(secrets, vault);
compat_module!(service_records, registry);
compat_module!(service_runtime, worker_core);
compat_module!(service_supervision, watchdog);
compat_module!(project_config_portability, config_portability_core);
compat_module!(project_effective_config, effective_config_core);
compat_module!(deterministic_drift, drift_core);
compat_module!(distributed_testing, distributed_core);
compat_module!(multinode_testing, multinode_core);
compat_module!(state_machine_proof, proof_trace_core);
compat_module!(requirement_traceability, trace_core);
compat_module!(testing_hardening, hardening_core);
compat_module!(transcripts, narratives);
compat_module!(typed_storage, cells);
compat_module!(upgrades, migrations);

pub mod core_api {
    pub use molten_core::*;
}

pub mod prelude {
    pub use crate::MoltenError;
    pub use crate::Result;
    pub use crate::core_api::prelude::*;
}

pub use failures::MoltenError;
pub use failures::Result;

pub fn greeting() -> &'static str {
    "hello from molten"
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

    #[test]
    fn greeting_mentions_project_name() {
        assert!(super::greeting().contains("molten"));
    }

    #[test]
    fn prelude_exposes_core_boundary_planner() {
        let admitted = AdmissionInputs {
            has_authority: true,
            evidence_fresh: true,
            resource_allowed: true,
            adapter_supported: true,
        };
        let plan = plan_adapter_effects(admitted, &[EffectKind::ReceiptWrite]);
        assert_eq!(plan.decision, BoundaryDecision::Admit);
        assert_eq!(plan.effects, vec![EffectKind::ReceiptWrite]);
    }

    #[test]
    fn prelude_boundary_planner_denies_missing_authority_without_effects() {
        let denied = AdmissionInputs {
            has_authority: false,
            evidence_fresh: true,
            resource_allowed: true,
            adapter_supported: true,
        };
        let plan = plan_adapter_effects(denied, &[EffectKind::StoreWrite]);
        assert_eq!(plan.decision, BoundaryDecision::Deny);
        assert!(plan.effects.is_empty());
    }
}
