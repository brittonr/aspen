
fn metric_descriptor_value(descriptor: &MetricDescriptor) -> preserves::IOValue {
    record(DESCRIPTOR_RECORD, vec![
        string(METRIC_DESCRIPTOR_SCHEMA),
        field("descriptor-id", string(&descriptor.descriptor_id)),
        field("declared-descriptor-ref", string(&descriptor.descriptor_ref)),
        field("profile-ref", string(&descriptor.profile_ref)),
        field("name", string(&descriptor.name)),
        field("unit", string(&descriptor.unit)),
        field("kind", string(descriptor.kind.as_str())),
        field("aggregation", string(descriptor.aggregation.as_str())),
        field("allowed-label-names", strings_value(descriptor.allowed_label_names.iter().map(String::as_str))),
        field("description", string(&descriptor.description)),
        checks(&["exporter-neutral", "bounded-label-vocabulary"]),
    ])
}

fn metric_sample_value(sample: &MetricSample) -> preserves::IOValue {
    record(SAMPLE_RECORD, vec![
        string(METRIC_SAMPLE_SCHEMA),
        field("declared-sample-ref", string(&sample.sample_ref)),
        field("descriptor-ref", string(&sample.descriptor_ref)),
        field("context", context_value(&sample.context)),
        field("labels", labels_value(&sample.labels)),
        field("value", i64_value(sample.value)),
        checks(&["labels-validated", "secret-values-excluded"]),
    ])
}

fn observation_event_value(event: &ObservationEvent) -> preserves::IOValue {
    record(EVENT_RECORD, vec![
        string(OBSERVATION_EVENT_SCHEMA),
        field("declared-event-ref", string(&event.event_ref)),
        field("kind", string(&event.event_kind)),
        field("severity", string(event.severity.as_str())),
        field("context", context_value(&event.context)),
        field("detail", string(&event.detail)),
        field("attributes", labels_value(&event.attributes)),
        checks(&["bounded-event", "secret-values-excluded"]),
    ])
}

fn health_input_value(input: &HealthInput) -> preserves::IOValue {
    record(HEALTH_INPUT_RECORD, vec![
        string(HEALTH_INPUT_SCHEMA),
        field("declared-health-ref", string(&input.health_ref)),
        field("context", context_value(&input.context)),
        field("state", string(input.state.as_str())),
        field("diagnostic-refs", strings_value(input.diagnostic_refs.iter().map(String::as_str))),
        checks(&["freshness-explicit", "health-is-observation-only"]),
    ])
}

fn readiness_policy_value(policy: &ReadinessPolicy) -> preserves::IOValue {
    record(READINESS_POLICY_RECORD, vec![
        string(READINESS_POLICY_SCHEMA),
        field("policy-ref", string(&policy.policy_ref)),
        field("target-scope", string(policy.target_scope.as_str())),
        field("required-source-ids", strings_value(policy.required_source_ids.iter().map(String::as_str))),
        field("scope-evidence-refs", strings_value(policy.scope_evidence_refs.iter().map(String::as_str))),
        field("allow-degraded", bool_value(policy.allow_degraded)),
        field("as-of-tick", u64_value(policy.as_of_tick)),
        checks(&["scope-promotion-explicit", "unavailable-does-not-pass"]),
    ])
}

fn health_decision_value(decision: &HealthDecision) -> preserves::IOValue {
    record(HEALTH_DECISION_RECORD, vec![
        string(HEALTH_DECISION_SCHEMA),
        field("prior-state", string(decision.prior_state.as_str())),
        field("state", string(decision.state.as_str())),
        field("readiness", string(decision.readiness.as_str())),
        field("scope", string(decision.scope.as_str())),
        field("supporting-health-refs", strings_value(decision.supporting_health_refs.iter().map(String::as_str))),
        field("issues", issues_value(&decision.issues)),
        checks(&[
            "freshness-evaluated",
            "scope-promotion-explicit",
            "health-is-not-authority",
        ]),
    ])
}

fn integrity_plan_value(plan: &IntegrityPlan) -> preserves::IOValue {
    record(INTEGRITY_PLAN_RECORD, vec![
        string(INTEGRITY_PLAN_SCHEMA),
        field("declared-plan-ref", string(&plan.plan_ref)),
        field("profile-ref", string(&plan.profile_ref)),
        field("scope-ref", string(&plan.scope_ref)),
        field("generation", u64_value(plan.generation)),
        field("read-only", bool_value(plan.read_only)),
        field("require-complete", bool_value(plan.require_complete)),
        field("max-items", usize_value(plan.max_items)),
        field("max-findings", usize_value(plan.max_findings)),
        field("targets", sequence(plan.targets.iter().map(integrity_target_value).collect())),
        field("resource-ref", string(&plan.resource_ref)),
        field("policy-refs", strings_value(plan.policy_refs.iter().map(String::as_str))),
        field("evidence-refs", strings_value(plan.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&plan.non_claims)),
        checks(&["read-only-default", "bounded-targets", "repair-authority-excluded"]),
    ])
}

fn scan_observation_value(observation: &ScanObservation) -> preserves::IOValue {
    record(SCAN_OBSERVATION_RECORD, vec![
        string(SCAN_OBSERVATION_SCHEMA),
        field("declared-observation-ref", string(&observation.observation_ref)),
        field("plan-ref", string(&observation.plan_ref)),
        field("item-ref", string(&observation.item_ref)),
        field("kind", string(observation.kind.as_str())),
        field("status", string(observation.status.as_str())),
        field("observed-content-ref", optional_string(observation.observed_content_ref.as_deref())),
        field("observed-length", optional_u64(observation.observed_length)),
        field("evidence-refs", strings_value(observation.evidence_refs.iter().map(String::as_str))),
        checks(&["read-only-observation", "capability-rooted-shell"]),
    ])
}

fn integrity_result_value(result: &IntegrityResult) -> preserves::IOValue {
    record(INTEGRITY_RESULT_RECORD, vec![
        string(INTEGRITY_RESULT_SCHEMA),
        field("plan-ref", string(&result.plan_ref)),
        field("decision", string(result.decision.as_str())),
        field("scanned-items", usize_value(result.scanned_items)),
        field("declared-items", usize_value(result.declared_items)),
        field("findings", sequence(result.findings.iter().map(integrity_finding_value).collect())),
        field("complete", bool_value(result.complete)),
        field("mutation-performed", bool_value(result.mutation_performed)),
        field("issues", issues_value(&result.issues)),
        checks(&[
            "partial-scan-cannot-pass",
            "findings-grant-no-mutation-authority",
            "result-scope-bounded-to-plan",
        ]),
    ])
}

fn adapter_profile_value(adapter: &ObservationAdapterProfile) -> preserves::IOValue {
    record(ADAPTER_PROFILE_RECORD, vec![
        string(OBSERVATION_ADAPTER_PROFILE_SCHEMA),
        field("adapter-id", string(&adapter.adapter_id)),
        field("declared-adapter-ref", string(&adapter.adapter_ref)),
        field("profile-ref", string(&adapter.profile_ref)),
        field("class", string(adapter.class.as_str())),
        field("max-queued-bytes", u64_value(adapter.max_queued_bytes)),
        field("timeout-ticks", u64_value(adapter.timeout_ticks)),
        field("drop-on-backpressure", bool_value(adapter.drop_on_backpressure)),
        field("required", bool_value(adapter.required)),
        field("evidence-refs", strings_value(adapter.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&adapter.non_claims)),
        checks(&["versioned-adapter", "terminal-failures-explicit"]),
    ])
}

fn adapter_outcome_value(outcome: &AdapterOutcome) -> preserves::IOValue {
    record(ADAPTER_OUTCOME_RECORD, vec![
        string(OBSERVATION_ADAPTER_OUTCOME_SCHEMA),
        field("operation-ref", string(&outcome.operation_ref)),
        field("adapter-ref", string(&outcome.adapter_ref)),
        field("payload-ref", string(&outcome.payload_ref)),
        field("kind", string(outcome.kind.as_str())),
        field("dropped-observations", u64_value(outcome.dropped_observations)),
        field("service-policy-signal", bool_value(outcome.service_policy_signal)),
        field("issues", issues_value(&outcome.issues)),
        checks(&["bounded-terminal-outcome", "exporter-failure-does-not-mutate-service"]),
    ])
}

fn adapter_status_value(status: &ObservationAdapterStatus) -> preserves::IOValue {
    record(ADAPTER_STATUS_RECORD, vec![
        string(OBSERVATION_ADAPTER_STATUS_SCHEMA),
        field("adapter-ref", string(&status.adapter_ref)),
        field("class", string(status.class.as_str())),
        field("kind", string(status.kind.as_str())),
        field("observed-tick", u64_value(status.observed_tick)),
        field("queued-bytes", u64_value(status.queued_bytes)),
        field("dropped-observations", u64_value(status.dropped_observations)),
        field("evidence-refs", strings_value(status.evidence_refs.iter().map(String::as_str))),
        field("issues", issues_value(&status.issues)),
        checks(&["terminal-status-explicit", "adapter-status-is-not-service-authority"]),
    ])
}

fn observation_snapshot_value(snapshot: &ObservationSnapshot) -> preserves::IOValue {
    record(SNAPSHOT_RECORD, vec![
        string(OBSERVATION_SNAPSHOT_SCHEMA),
        field("snapshot-id", string(&snapshot.snapshot_id)),
        field("profile-ref", string(&snapshot.profile_ref)),
        field("scope", string(snapshot.scope.as_str())),
        field("generation", u64_value(snapshot.generation)),
        field("as-of-tick", u64_value(snapshot.as_of_tick)),
        field("valid-until-tick", u64_value(snapshot.valid_until_tick)),
        field("series", sequence(snapshot.series.iter().map(aggregated_series_value).collect())),
        field("event-refs", strings_value(snapshot.event_refs.iter().map(String::as_str))),
        field("health-refs", strings_value(snapshot.health_refs.iter().map(String::as_str))),
        field("integrity-result-refs", strings_value(snapshot.integrity_result_refs.iter().map(String::as_str))),
        field("adapter-outcome-refs", strings_value(snapshot.adapter_outcome_refs.iter().map(String::as_str))),
        field("evidence-refs", strings_value(snapshot.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&snapshot.non_claims)),
        checks(&[
            "bounded-operator-view",
            "freshness-explicit",
            "snapshot-is-not-release-authority",
        ]),
    ])
}

fn bounds_value(bounds: &ObservationBounds) -> preserves::IOValue {
    record("observation-bounds", vec![
        field("max-descriptors", usize_value(bounds.max_descriptors)),
        field("max-labels-per-sample", usize_value(bounds.max_labels_per_sample)),
        field("max-label-name-bytes", usize_value(bounds.max_label_name_bytes)),
        field("max-label-value-bytes", usize_value(bounds.max_label_value_bytes)),
        field("max-series", usize_value(bounds.max_series)),
        field("max-events", usize_value(bounds.max_events)),
        field("max-event-detail-bytes", usize_value(bounds.max_event_detail_bytes)),
        field("max-queued-bytes", u64_value(bounds.max_queued_bytes)),
        field("max-snapshots", usize_value(bounds.max_snapshots)),
        field("max-scan-items", usize_value(bounds.max_scan_items)),
        field("max-findings", usize_value(bounds.max_findings)),
        field("max-diagnostics", usize_value(bounds.max_diagnostics)),
        field("min-export-interval-ticks", u64_value(bounds.min_export_interval_ticks)),
    ])
}

fn context_value(context: &ObservationContext) -> preserves::IOValue {
    record("observation-context", vec![
        field("source-id", string(&context.source_id)),
        field("source-ref", string(&context.source_ref)),
        field("profile-ref", string(&context.profile_ref)),
        field("scope", string(context.scope.as_str())),
        field("generation", u64_value(context.generation)),
        field("observed-tick", u64_value(context.observed_tick)),
        field("valid-until-tick", u64_value(context.valid_until_tick)),
        field("resource-ref", string(&context.resource_ref)),
        field("evidence-refs", strings_value(context.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&context.non_claims)),
    ])
}

fn redaction_rule_value(rule: &RedactionRule) -> preserves::IOValue {
    record("redaction-rule", vec![
        field("label-name", string(&rule.label_name)),
        field("class", string(rule.class.as_str())),
        field("marker", string(&rule.marker)),
    ])
}

fn labels_value(labels: &[MetricLabel]) -> preserves::IOValue {
    sequence(
        labels
            .iter()
            .map(|label| {
                record("label", vec![
                    field("name", string(&label.name)),
                    field("value", string(&label.value)),
                    field("class", string(label.class.as_str())),
                ])
            })
            .collect(),
    )
}

fn integrity_target_value(target: &IntegrityTarget) -> preserves::IOValue {
    record("integrity-target", vec![
        field("item-ref", string(&target.item_ref)),
        field("kind", string(target.kind.as_str())),
        field("expected-content-ref", optional_string(target.expected_content_ref.as_deref())),
        field("expected-length", optional_u64(target.expected_length)),
    ])
}

fn integrity_finding_value(finding: &IntegrityFinding) -> preserves::IOValue {
    record(INTEGRITY_FINDING_RECORD, vec![
        string(INTEGRITY_FINDING_SCHEMA),
        field("finding-id", string(&finding.finding_id)),
        field("item-ref", optional_string(finding.item_ref.as_deref())),
        field("class", string(finding.class.as_str())),
        field("expected-ref", optional_string(finding.expected_ref.as_deref())),
        field("observed-ref", optional_string(finding.observed_ref.as_deref())),
        field("recommendation", string(finding.recommendation.as_str())),
        field("grants-mutation-authority", bool_value(finding.grants_mutation_authority)),
    ])
}
