//! An `Installation` binds scope mutations only to a running admitted host and the exact active
//! consistency group for its manifest, generation and service.

use super::*;

#[derive(Debug, Clone, Copy)]
struct Executor;

impl crate::system_extension::SystemExtensionExecutor for Executor {
    fn execution_profile(&self) -> crate::system_extension::ExecutionProfile {
        crate::system_extension::ExecutionProfile::InProcessNative
    }

    fn invoke(
        &mut self,
        _invocation: &crate::system_extension::CallbackInvocation,
    ) -> std::result::Result<crate::system_extension::CallbackOutcome, String> {
        Ok(crate::system_extension::CallbackOutcome {
            output_refs: vec![fixtures::reference("collaboration-callback-output")],
            effects: Vec::new(),
            state_ref: Some(fixtures::reference("collaboration-callback-state")),
            checkpoint_ref: None,
            health: crate::system_extension::HealthState::Healthy,
        })
    }
}

fn host() -> crate::system_extension::Dispatcher<Executor> {
    crate::system_extension::Dispatcher::new(super::manifesttests::admitted(), Executor).expect("admitted host")
}

fn declared_group(
    host: &crate::system_extension::Dispatcher<Executor>,
    generation: u64,
) -> crate::fabric_consistency::Group {
    let manifest = host.manifest().manifest();
    crate::fabric_consistency::canonical_group(crate::fabric_consistency::GroupInput {
        group_id: "group:collaboration-scope".to_string(),
        extension_id: manifest.extension_id.clone(),
        service_id: manifest.service_id.clone(),
        service_generation: generation,
        application_manifest_ref: host.manifest().manifest_ref().to_string(),
        engine_algorithm_profile: "raft".to_string(),
        engine_implementation_profile: "live-raft-static-v1".to_string(),
        membership_ref: fixtures::reference("collaboration-group-membership"),
        config_epoch: crate::fabric_consistency::INITIAL_CONSISTENCY_EPOCH,
        placement_ref: fixtures::reference("collaboration-group-placement"),
        fencing_ref: fixtures::reference("collaboration-group-fencing"),
        fencing_epoch: crate::fabric_consistency::INITIAL_CONSISTENCY_EPOCH,
        resource_profile_ref: fixtures::reference("collaboration-group-resources"),
        policy_refs: manifest.policy_refs.clone(),
        non_claims: vec![
            "consistency-does-not-prove-extension-semantics".to_string(),
            "local-status-is-not-quorum-currentness".to_string(),
        ],
        supported_read_modes: vec![
            crate::fabric_consistency::ReadMode::LocalStale,
            crate::fabric_consistency::ReadMode::Linearizable,
        ],
        max_command_bytes: 4_096,
        max_in_flight_operations: 8,
    })
    .expect("declared collaboration group")
}

fn active(declared: &crate::fabric_consistency::Group) -> crate::fabric_consistency::Group {
    let command = crate::fabric_consistency::PortCommandInput {
        request_ref: fixtures::reference("collaboration-group-open"),
        binding_ref: declared.binding_ref.clone(),
        group_id: declared.group_id.clone(),
        extension_id: declared.extension_id.clone(),
        service_id: declared.service_id.clone(),
        service_generation: declared.service_generation,
        application_manifest_ref: declared.application_manifest_ref.clone(),
        engine_algorithm_profile: declared.engine_algorithm_profile.clone(),
        engine_implementation_profile: declared.engine_implementation_profile.clone(),
        membership_ref: declared.membership_ref.clone(),
        config_epoch: declared.config_epoch,
        placement_ref: declared.placement_ref.clone(),
        fencing_ref: declared.fencing_ref.clone(),
        fencing_epoch: declared.fencing_epoch,
        resource_profile_ref: declared.resource_profile_ref.clone(),
        policy_refs: declared.policy_refs.clone(),
        authority_refs: vec![fixtures::reference("collaboration-group-authority")],
        observed_in_flight_operations: 0,
        operation: crate::fabric_consistency::Request::Open {
            mode: crate::fabric_consistency::GroupOpenMode::Create,
        },
    };
    let plan = crate::fabric_consistency::plan(declared, command).expect("group open plan");
    let outcome = crate::fabric_consistency::normalize(declared, &plan, crate::fabric_consistency::Input {
        request_ref: plan.request_ref.clone(),
        binding_ref: plan.binding_ref.clone(),
        service_generation: declared.service_generation,
        config_epoch: declared.config_epoch,
        fencing_epoch: declared.fencing_epoch,
        kind: crate::fabric_consistency::Kind::Opened,
        result_ref: Some(fixtures::reference("collaboration-group-opened")),
        evidence_refs: vec![fixtures::reference("collaboration-group-evidence")],
        diagnostics: Vec::new(),
    })
    .expect("normalized group open");
    crate::fabric_consistency::apply_outcome(declared, &plan, &outcome).expect("active collaboration group")
}

fn denial(result: crate::error::Result<Installation>) -> String {
    result.map(|_| ()).expect_err("installation denied").to_string()
}

// r[verify molten.collaboration_scope.versioned_extension]
#[test]
fn installation_binds_only_a_running_host_and_its_exact_active_group() {
    let mut host = host();
    let generation = host.manifest().manifest().initial_generation;
    let group = active(&declared_group(&host, generation));
    assert!(denial(Installation::from_host(&host, &group)).ends_with("collaboration-scope/stale"), "not running");
    host.activate(1).expect("initialize and start");
    assert_eq!(host.state().generation, generation, "activation keeps the admitted generation");
    let installation = Installation::from_host(&host, &group).expect("running host and exact active group");
    assert_eq!(installation.generation(), generation);
    let foreign_generation = active(&declared_group(&host, generation + 1));
    assert!(denial(Installation::from_host(&host, &foreign_generation)).ends_with("collaboration-scope/stale"));
    let declared_only = declared_group(&host, generation);
    assert!(denial(Installation::from_host(&host, &declared_only)).ends_with("collaboration-scope/stale"));
}
