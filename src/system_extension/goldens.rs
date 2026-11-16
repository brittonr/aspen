use super::*;

// These reference records are assembled field-by-field from raw-u64 fixture inputs,
// independently of the values emitted by the migrated canonical adapters.
fn raw_field(label: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn raw_strings<'a>(values: impl IntoIterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.into_iter().map(crate::preserves_rail::string).collect())
}

fn raw_optional(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", vec![]),
    }
}

fn raw_checks(values: &[&str]) -> preserves::IOValue {
    raw_field("checks", raw_strings(values.iter().copied()))
}

fn raw_resources(resources: &ResourceEnvelope) -> preserves::IOValue {
    let number = crate::preserves_rail::u64_value;
    crate::preserves_rail::record("resource-envelope-v1", vec![
        raw_field("max-concurrent-callbacks", number(resources.max_concurrent_callbacks)),
        raw_field("max-queued-events", number(resources.max_queued_events)),
        raw_field("max-inflight-bytes", number(resources.max_inflight_bytes)),
        raw_field("max-open-streams", number(resources.max_open_streams)),
        raw_field("max-timers", number(resources.max_timers)),
        raw_field("max-effect-requests", number(resources.max_effect_requests)),
        raw_field("callback-deadline-ticks", number(resources.callback_deadline_ticks)),
        raw_field("shutdown-grace-ticks", number(resources.shutdown_grace_ticks)),
        raw_field("max-restart-attempts", number(resources.max_restart_attempts)),
        raw_field("overload-policy", crate::preserves_rail::string(resources.overload_policy.as_str())),
    ])
}

fn raw_usage(usage: ResourceUsage) -> preserves::IOValue {
    let number = crate::preserves_rail::u64_value;
    crate::preserves_rail::record("resource-usage-v1", vec![
        raw_field("concurrent-callbacks", number(usage.concurrent_callbacks)),
        raw_field("queued-events", number(usage.queued_events)),
        raw_field("inflight-bytes", number(usage.inflight_bytes)),
        raw_field("open-streams", number(usage.open_streams)),
        raw_field("timers", number(usage.timers)),
        raw_field("effect-requests", number(usage.effect_requests)),
    ])
}

fn assert_raw_identity(actual: &preserves::IOValue, reference: &str, expected: &preserves::IOValue) {
    assert_eq!(
        crate::preserves_rail::canonical_bytes(actual).unwrap(),
        crate::preserves_rail::canonical_bytes(expected).unwrap()
    );
    assert_eq!(reference, crate::preserves_rail::canonical_hash(expected).unwrap());
}

// r[verify molten.system_extension.lifecycle]
// r[verify molten.system_extension.manifest]
// r[verify molten.system_extension.evidence]
#[test]
fn raw_u64_manifest_lifecycle_callback_and_effect_canonical_identities_match() {
    let run = run_executable(ExecutionProfile::InProcessNative).expect("deterministic extension fixture");
    let [admitted, ..] = super::fixture::fixture_manifests(ExecutionProfile::InProcessNative).unwrap();
    assert_raw_manifest(&run, &admitted);
    assert_raw_lifecycle(&run, &admitted);
    assert_raw_callback(&run, &admitted);
    assert_raw_effect(&run, &admitted);
}

fn assert_raw_manifest(run: &super::fixture::ExecutableRun, admitted: &AdmittedManifest) {
    let manifest = admitted.manifest();
    let string = |value: &str| crate::preserves_rail::string(value);
    let number = crate::preserves_rail::u64_value;
    let raw_manifest = crate::preserves_rail::record("system-extension-manifest-v1", vec![
        string(SYSTEM_EXTENSION_MANIFEST_SCHEMA),
        raw_field("extension-id", string(&manifest.extension_id)),
        raw_field("service-id", string(&manifest.service_id)),
        raw_field("implementation-ref", string(&manifest.implementation_ref)),
        raw_field("callbacks", raw_strings(manifest.callbacks.iter().map(|callback| callback.as_str()))),
        raw_field(
            "required-port-binding-refs",
            raw_strings(admitted.required_port_bindings().iter().map(|binding| binding.binding_ref.as_str())),
        ),
        raw_field(
            "optional-port-binding-refs",
            raw_strings(admitted.optional_port_bindings().iter().map(|binding| binding.binding_ref.as_str())),
        ),
        raw_field("capability-refs", raw_strings(manifest.capability_refs.iter().map(String::as_str))),
        raw_field("policy-refs", raw_strings(manifest.policy_refs.iter().map(String::as_str))),
        raw_field("provenance-refs", raw_strings(manifest.provenance_refs.iter().map(String::as_str))),
        raw_field("resource-envelope", raw_resources(&manifest.resources)),
        raw_field("execution-profile", string("in-process-native")),
        raw_field("state-schema", string(&manifest.state_schema)),
        raw_field(
            "compatible-state-schemas",
            raw_strings(manifest.compatible_state_schemas.iter().map(String::as_str)),
        ),
        raw_field("evidence-profile-ref", string(&manifest.evidence_profile_ref)),
        raw_field("tier-admission-ref", string(admitted.tier_admission_ref())),
        raw_field("initial-generation", number(1)),
        raw_field("non-claims", raw_strings(manifest.non_claims.iter().map(|claim| claim.as_str()))),
        raw_checks(&[
            "canonical-system-extension-manifest",
            "system-tier-admitted",
            "ports-exactly-bound",
            "capabilities-not-artifact-possession",
            "plugin-metadata-not-system-extension-admission",
            "execution-profile-no-fallback",
        ]),
    ]);
    assert_raw_identity(&run.manifest_value, &run.manifest_ref, &raw_manifest);
}

fn assert_raw_lifecycle(run: &super::fixture::ExecutableRun, admitted: &AdmittedManifest) {
    let manifest = admitted.manifest();
    let string = |value: &str| crate::preserves_rail::string(value);
    let number = crate::preserves_rail::u64_value;

    let lifecycle = run
        .evidence
        .iter()
        .find_map(|item| match item {
            Evidence::Lifecycle(receipt) => Some(receipt),
            _ => None,
        })
        .expect("lifecycle receipt");
    assert_eq!(lifecycle.event.kind, LifecycleEventKind::Install);
    let raw_lifecycle = crate::preserves_rail::record("system-extension-lifecycle-v2", vec![
        string(SYSTEM_EXTENSION_LIFECYCLE_SCHEMA),
        raw_field("manifest-ref", string(&run.manifest_ref)),
        raw_field("extension-id", string(&manifest.extension_id)),
        raw_field("service-id", string(&manifest.service_id)),
        raw_field("event", string("install")),
        raw_field("event-generation", number(1)),
        raw_field("previous-generation", number(0)),
        raw_field("next-generation", number(1)),
        raw_field("event-incarnation", number(0)),
        raw_field("previous-incarnation", number(0)),
        raw_field("next-incarnation", number(0)),
        raw_field("previous-phase", string("absent")),
        raw_field("next-phase", string("installed")),
        raw_field("restart-attempts", number(0)),
        raw_field("health", string("stopped")),
        raw_field("checkpoint-ref", raw_optional(None)),
        raw_field("failure-class", raw_optional(None)),
        raw_field("resource-usage", raw_usage(ResourceUsage::zero())),
        raw_checks(&[
            "generation-fenced",
            "incarnation-fenced",
            "transition-law-validated",
            "logical-inputs-only",
            "receipt-is-not-consensus-or-durability-proof",
        ]),
    ]);
    assert_raw_identity(&lifecycle.value, &lifecycle.receipt_ref, &raw_lifecycle);
}

fn assert_raw_callback(run: &super::fixture::ExecutableRun, admitted: &AdmittedManifest) {
    let manifest = admitted.manifest();
    let string = |value: &str| crate::preserves_rail::string(value);
    let number = crate::preserves_rail::u64_value;

    let source = run
        .evidence
        .iter()
        .find_map(|item| match item {
            Evidence::Callback(receipt) => Some(receipt),
            _ => None,
        })
        .expect("callback invocation");
    let invocation = &source.invocation;
    let callback = super::canonical::callback_receipt(super::canonical::CallbackReceiptInput {
        manifest_ref: &run.manifest_ref,
        extension_id: &manifest.extension_id,
        service_id: &manifest.service_id,
        execution_profile: ExecutionProfile::InProcessNative,
        invocation,
        decision: CallbackExecutionDecision::ExecutorFailed,
        outcome: None,
        diagnostic: Some("fixture-failure"),
    })
    .unwrap();
    let raw_binding = crate::preserves_rail::record("system-extension-execution-binding-v1", vec![
        string(SYSTEM_EXTENSION_EXECUTION_BINDING_SCHEMA),
        raw_field("manifest-ref", string(&run.manifest_ref)),
        raw_field("execution-profile", string("in-process-native")),
        raw_field("callback", string(invocation.callback.as_str())),
        raw_field("generation", number(1)),
        raw_field("sequence", number(invocation.sequence)),
        raw_field("event-ref", string(&invocation.event_ref)),
        raw_field("decision", string("executor-failed")),
        raw_field("output-refs", raw_strings([])),
        raw_field("effect-refs", raw_strings([])),
        raw_checks(&[
            "executor-invoked",
            "profile-matched",
            "generation-matched",
            "outcome-validated-before-effect-release",
        ]),
    ]);
    let binding_ref = crate::preserves_rail::canonical_hash(&raw_binding).unwrap();
    assert_eq!(callback.execution_binding_ref, binding_ref);
    let raw_callback = crate::preserves_rail::record("system-extension-callback-receipt-v1", vec![
        string(SYSTEM_EXTENSION_CALLBACK_SCHEMA),
        raw_field("manifest-ref", string(&run.manifest_ref)),
        raw_field("extension-id", string(&manifest.extension_id)),
        raw_field("service-id", string(&manifest.service_id)),
        raw_field("callback", string(invocation.callback.as_str())),
        raw_field("generation", number(1)),
        raw_field("sequence", number(invocation.sequence)),
        raw_field("event-ref", string(&invocation.event_ref)),
        raw_field("execution-binding-ref", string(&binding_ref)),
        raw_field("decision", string("executor-failed")),
        raw_field("output-refs", raw_strings([])),
        raw_field("effect-refs", raw_strings([])),
        raw_field("state-ref", raw_optional(None)),
        raw_field("checkpoint-ref", raw_optional(None)),
        raw_field("diagnostic", raw_optional(Some("fixture-failure"))),
        raw_checks(&[
            "real-callback-execution-bound",
            "typed-effects-only",
            "per-event-evidence-profile",
            "success-is-not-semantic-correctness-proof",
        ]),
    ]);
    assert_raw_identity(&callback.value, &callback.receipt_ref, &raw_callback);
}

fn assert_raw_effect(run: &super::fixture::ExecutableRun, admitted: &AdmittedManifest) {
    let string = |value: &str| crate::preserves_rail::string(value);
    let number = crate::preserves_rail::u64_value;

    let completion = run.first_effect_completions.first().expect("effect completion");
    let binding = admitted.required_port_bindings().first().expect("transport port binding");
    let effect = run.first_request_effects.first().expect("approved effect");
    assert_eq!(completion.binding_ref, binding.binding_ref);
    let raw_effect = crate::preserves_rail::record("system-extension-effect-completion-v2", vec![
        string(SYSTEM_EXTENSION_EFFECT_COMPLETION_SCHEMA),
        raw_field("callback-receipt-ref", string(&completion.callback_receipt_ref)),
        raw_field("binding-ref", string(&binding.binding_ref)),
        raw_field("port-id", string(&binding.binding.key.port_id)),
        raw_field("port-version", string(&binding.binding.key.version)),
        raw_field("request-ref", string(&effect.request_ref)),
        raw_field("generation", number(1)),
        raw_field("output-schema-ref", string(&effect.output_schema_ref)),
        raw_field("output-ref", string(&completion.output_ref)),
        raw_field("output-value", raw_optional(None)),
        raw_checks(&[
            "exact-bound-port-routed",
            "generation-correlated",
            "output-schema-matched",
            "materialized-output-identity-checked",
            "completion-is-not-durability-proof",
        ]),
    ]);
    assert_raw_identity(&completion.value, &completion.completion_ref, &raw_effect);
}
