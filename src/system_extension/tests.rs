use super::*;

const EXPECTED_FIRST_EFFECT_COUNT: usize = 1;
const EXPECTED_RESTART_ATTEMPTS: u64 = 1;
const UPGRADED_GENERATION: u64 = 2;
const ROLLED_BACK_GENERATION: u64 = 3;

// r[verify molten.system_extension.callbacks]
// r[verify molten.system_extension.execution_profiles]
// r[verify molten.system_extension.final_validation]
// r[verify molten.system_extension.native_host.effect_completion_value.compatibility]
#[test]
fn executable_fixture_runs_real_callbacks_under_two_admitted_profiles() {
    let in_process = run_executable_system_extension_fixture(ExecutionProfile::InProcessNative)
        .expect("in-process executable fixture");
    let sandboxed = run_executable_system_extension_fixture(ExecutionProfile::SandboxedComponent)
        .expect("sandboxed executable fixture");

    for run in [&in_process, &sandboxed] {
        assert_eq!(run.first_request_effects.len(), EXPECTED_FIRST_EFFECT_COUNT);
        assert_eq!(run.first_effect_completions.len(), EXPECTED_FIRST_EFFECT_COUNT);
        assert!(run.first_effect_completions.iter().all(|completion| {
            completion.binding_ref.starts_with("blake3:")
                && completion.completion_ref.starts_with("blake3:")
                && completion.materialized_output.is_none()
        }));
        assert!(validate_executable_conformance(&run.conformance).is_empty());
        assert_eq!(run.upgraded_status.status.phase, LifecyclePhase::Running);
        assert_eq!(run.upgraded_status.status.generation, UPGRADED_GENERATION);
        assert_eq!(run.rolled_back_status.status.phase, LifecyclePhase::Running);
        assert_eq!(run.rolled_back_status.status.generation, ROLLED_BACK_GENERATION);
        assert_eq!(run.recovered_status.status.phase, LifecyclePhase::Running);
        assert_eq!(run.recovered_status.status.restart_attempts, EXPECTED_RESTART_ATTEMPTS);
        assert_eq!(run.final_status.status.phase, LifecyclePhase::Stopped);
        assert!(run.final_status.status.checkpoint_ref.is_some());
        assert!(run
            .evidence
            .iter()
            .any(|item| matches!(item, HostEvidence::Callback(receipt) if receipt.decision == CallbackExecutionDecision::ExecutorFailed)));
        assert!(run.evidence.iter().any(|item| matches!(item, HostEvidence::EffectCompletion(_))));
        assert!(run.evidence.iter().any(
            |item| matches!(item, HostEvidence::Migration(receipt) if receipt.operation == MigrationOperation::Upgrade)
        ));
        assert!(run.evidence.iter().any(
            |item| matches!(item, HostEvidence::Migration(receipt) if receipt.operation == MigrationOperation::Rollback)
        ));
        assert!(run.evidence.iter().any(|item| matches!(item, HostEvidence::Readiness(receipt) if receipt.ready)));
        assert!(run
            .evidence
            .iter()
            .any(|item| matches!(item, HostEvidence::Readiness(receipt) if !receipt.ready && receipt.health == HealthState::Stopped)));
        assert!(run.evidence.iter().all(|item| item.evidence_ref().starts_with("blake3:")));
    }
    assert_eq!(in_process.evidence.len(), sandboxed.evidence.len());
    assert_ne!(in_process.manifest_ref, sandboxed.manifest_ref);
}

// r[verify molten.system_extension.operator_readback]
// r[verify molten.system_extension.evidence]
#[test]
fn status_readback_is_canonical_bounded_and_excludes_secret_material() {
    let run = run_executable_system_extension_fixture(ExecutionProfile::SandboxedComponent)
        .expect("sandboxed executable fixture");
    let text = crate::preserves_rail::to_text(&run.final_status.value).expect("status text");

    assert!(run.final_status.status_ref.starts_with("blake3:"));
    assert!(text.contains("system-extension-status-v1"));
    assert!(text.contains("sandboxed-component"));
    assert!(text.contains("stopped"));
    assert!(text.contains("active-generation-visible"));
    assert!(!text.contains("private-key"));
    assert!(!text.contains("token-material"));
    assert!(!text.contains("environment-variable"));

    let malformed_text = text.replacen("stopped", "secret phase value", 1);
    let malformed = crate::preserves_rail::parse_text(&malformed_text).expect("malformed status syntax");
    let error = parse_operator_status_readback(&malformed).expect_err("unknown phase must not render");
    assert!(error.to_string().contains("unsupported system-extension phase"));
}

// r[verify molten.system_extension.evidence]
#[test]
fn deterministic_fixture_reproduces_manifest_status_and_evidence_identity() {
    let first =
        run_executable_system_extension_fixture(ExecutionProfile::SandboxedComponent).expect("first fixture run");
    let second =
        run_executable_system_extension_fixture(ExecutionProfile::SandboxedComponent).expect("second fixture run");
    let first_refs: Vec<_> = first.evidence.iter().map(HostEvidence::evidence_ref).collect();
    let second_refs: Vec<_> = second.evidence.iter().map(HostEvidence::evidence_ref).collect();

    assert_eq!(first.manifest_ref, second.manifest_ref);
    assert_eq!(first.final_status.status_ref, second.final_status.status_ref);
    assert_eq!(first_refs, second_refs);
}

// r[verify molten.system_extension.execution_profiles]
#[test]
fn unsupported_fixture_profile_has_no_silent_fallback() {
    let error = run_executable_system_extension_fixture(ExecutionProfile::NativeProcess)
        .expect_err("native process fixture profile is not admitted");

    assert!(error.to_string().contains("admits in-process-native or sandboxed-component"));
}
