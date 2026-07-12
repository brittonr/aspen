fn explicit_destination_shell(path: &std::path::Path) -> molten::error::Result<molten::materialization::MaterializationRoot> {
    molten::materialization::MaterializationRoot::open(path)
}

fn capability_materialize(
    root: &molten::materialization::MaterializationRoot,
    plan: &molten::materialization::MaterializationPlan,
    payloads: &[molten::materialization::MaterializationPayload],
) -> molten::error::Result<molten::materialization::MaterializationReceipt> {
    root.materialize(plan, payloads)
}
