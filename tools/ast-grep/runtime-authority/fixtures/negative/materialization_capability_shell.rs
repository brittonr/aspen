fn explicit_destination_shell(path: &std::path::Path) -> molten::error::Result<molten::materialization::Root> {
    molten::materialization::Root::open(path)
}

fn capability_materialize(
    root: &molten::materialization::Root,
    plan: &molten::materialization::Plan,
    payloads: &[molten::materialization::Payload],
) -> molten::error::Result<molten::materialization::Receipt> {
    root.materialize(plan, payloads)
}
