use super::*;
use crate::fabric_observability;

fn reference(index: u8) -> String {
    format!("blake3:{index:064x}")
}

fn profile() -> fabric_observability::ObservationProfile {
    fabric_observability::ObservationProfile {
        schema: fabric_observability::OBSERVATION_PROFILE_SCHEMA.into(),
        profile_id: "synthetic-ebpf".into(),
        profile_ref: reference(1),
        bounds: fabric_observability::ObservationBounds {
            max_descriptors: 6,
            max_labels_per_sample: 1,
            max_label_name_bytes: 32,
            max_label_value_bytes: 64,
            max_series: 6,
            max_events: 1,
            max_event_detail_bytes: 128,
            max_queued_bytes: 4096,
            max_snapshots: 1,
            max_scan_items: 1,
            max_findings: 1,
            max_diagnostics: 1,
            min_export_interval_ticks: 1,
        },
        redaction_rules: Vec::new(),
        non_claims: fabric_observability::REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn context() -> fabric_observability::ObservationContext {
    fabric_observability::ObservationContext {
        source_id: "synthetic-ebpf".into(),
        source_ref: reference(2),
        profile_ref: reference(1),
        scope: fabric_observability::ClaimScope::LocalComponent,
        generation: 1,
        observed_tick: 10,
        valid_until_tick: 15,
        resource_ref: reference(3),
        evidence_refs: vec![reference(4)],
        non_claims: fabric_observability::REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn policy(scope: fabric_observability::ClaimScope) -> fabric_observability::ReadinessPolicy {
    fabric_observability::ReadinessPolicy {
        schema: fabric_observability::READINESS_POLICY_SCHEMA.into(),
        policy_ref: reference(5),
        target_scope: scope,
        required_source_ids: vec!["synthetic-ebpf".into()],
        scope_evidence_refs: Vec::new(),
        allow_degraded: false,
        as_of_tick: 10,
    }
}

// r[verify linux_ebpf_observability.failure]
// r[verify linux_ebpf_observability.confidentiality]
#[test]
fn only_complete_current_local_windows_can_satisfy_local_readiness() {
    let (mut adapter, mut window) = super::tests::fixture();
    adapter.binding.scope = reference(3);
    window.binding = adapter.binding.clone();
    for (index, mapping) in adapter.mappings.iter_mut().enumerate() {
        mapping.descriptor.profile_ref = reference(1);
        mapping.descriptor.descriptor_ref = reference(index as u8 + 10);
    }
    let profile = profile();
    let context = context();
    let projection = project_ebpf_window(&profile, &adapter, &window, &context, 10).expect("complete fixture");
    assert_eq!(projection.samples.iter().map(|sample| sample.value).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5, 6]);
    let inputs = [projection.health];
    let local = fabric_observability::ClaimScope::LocalComponent;
    let healthy = fabric_observability::HealthState::Healthy;
    assert_eq!(
        fabric_observability::evaluate_health_readiness(&profile, &policy(local), healthy, &inputs).readiness,
        fabric_observability::ReadinessDecision::Pass
    );
    for scope in [
        fabric_observability::ClaimScope::Cluster,
        fabric_observability::ClaimScope::ReleaseProduction,
    ] {
        assert_ne!(
            fabric_observability::evaluate_health_readiness(&profile, &policy(scope), healthy, &inputs).readiness,
            fabric_observability::ReadinessDecision::Pass
        );
    }
    window.final_close_succeeded = false;
    let partial = project_ebpf_window(&profile, &adapter, &window, &context, 10).expect("partial fixture");
    assert!(!partial.complete);
    assert_ne!(
        fabric_observability::evaluate_health_readiness(&profile, &policy(local), healthy, &[partial.health]).readiness,
        fabric_observability::ReadinessDecision::Pass
    );
    window.final_close_succeeded = true;
    assert_eq!(project_ebpf_window(&profile, &adapter, &window, &context, 16), Err(EbpfIssue::Stale));
    window.accounting = None;
    assert_eq!(project_ebpf_window(&profile, &adapter, &window, &context, 10), Err(EbpfIssue::Accounting));
}
