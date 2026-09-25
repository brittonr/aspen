
pub fn build_record_value(input: &BuildRecordInput<'_>) -> Result<IoValue> {
    validate_ref(input.expected_artifact_ref, "provenance expected artifact ref")?;
    validate_refs(input.source_refs, "provenance build source ref")?;
    validate_ref(input.dependency_closure_ref, "provenance build dependency closure ref")?;
    validate_refs(input.toolchain_refs, "provenance build toolchain ref")?;
    validate_build_params(input.build_params)?;
    validate_ref(input.builder_ref, "provenance build builder ref")?;
    validate_refs(input.nix_derivation_refs, "provenance build nix derivation ref")?;
    validate_refs(input.policy_refs, "provenance build policy ref")?;
    validate_refs(input.evidence_refs, "provenance build evidence ref")?;
    Ok(record("provenance-build-record-v1", vec![
        string(crate::preserves_rail::PROVENANCE_BUILD_RECORD_SCHEMA),
        record("expected-artifact", vec![string(input.expected_artifact_ref)]),
        record("source", vec![refs_sequence(input.source_refs)]),
        record("dependency-closure", vec![string(input.dependency_closure_ref)]),
        record("toolchain", vec![refs_sequence(input.toolchain_refs)]),
        record("build-params", vec![build_params_sequence(input.build_params)]),
        record("builder", vec![string(input.builder_ref)]),
        record("nix-derivations", vec![refs_sequence(input.nix_derivation_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
    ]))
}
