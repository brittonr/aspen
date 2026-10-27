pub(super) fn validate(
    bundle_ref: &str,
    envelope: &super::AdmissionEnvelope,
    blockers: &mut impl crate::bounded::VecSink<String>,
) {
    if envelope.schema_id != super::COMPONENT_ADMISSION_ENVELOPE_SCHEMA || envelope.bundle_ref != bundle_ref {
        blockers.push_item("component admission envelope is stale or bound to another bundle".to_string());
    }
    for (label, refs) in [
        ("Valence sidecar", &envelope.valence_sidecar_refs),
        ("Cairn acceptance", &envelope.cairn_acceptance_refs),
        ("policy", &envelope.policy_refs),
        ("authority", &envelope.authority_refs),
        ("resource", &envelope.resource_refs),
    ] {
        if refs.len() > super::MAX_COMPONENT_EVIDENCE_REFS || !super::super::super::model::valid_ref_collection(refs) {
            blockers.push_item(format!(
                "component admission envelope {label} refs are missing, malformed, duplicate, or unsorted"
            ));
        }
    }
}
