
fn supervision_refs(
    suite: &ServiceSupervisionSuite,
    monitor_refs: &[String],
    notification_refs: &[String],
) -> Result<Vec<String>> {
    let total = suite
        .links
        .len()
        .checked_add(monitor_refs.len())
        .and_then(|total| total.checked_add(notification_refs.len()))
        .ok_or_else(|| MoltenError::invalid_harness("service supervision ref count overflow"))?;
    let mut refs = Vec::with_capacity(total);
    refs.extend(suite.links.iter().map(|link| link.link_ref.clone()));
    refs.extend_from_slice(monitor_refs);
    refs.extend_from_slice(notification_refs);
    Ok(refs)
}
