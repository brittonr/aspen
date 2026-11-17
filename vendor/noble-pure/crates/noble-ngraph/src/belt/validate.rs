mod machine;

pub(super) fn check<'a>(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    bytes: &'a [u8],
) -> Result<crate::belt::types::CheckedArtifact<'a>, crate::belt::types::Refusal> {
    super::support::admit_graph(checked)?;
    let view = super::codec::decode(ids, bytes)?;
    if view.graph_id != checked.id() {
        return Err(crate::belt::types::Refusal::GraphIdentity);
    }
    machine::check_stream(checked, &view)?;
    Ok(crate::belt::types::CheckedArtifact {
        bytes: view.bytes,
        graph_id: view.graph_id,
        belt_slots: view.belt_slots,
        save_slots: view.save_slots,
        instruction_digest: view.instruction_digest,
        artifact_binding: view.artifact_binding,
        instruction_count: view.instruction_count,
    })
}
