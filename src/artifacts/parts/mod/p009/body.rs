
pub fn impact_refs_with_root(root: &CapabilityArtifactRoot, seeds: &[String]) -> Result<Vec<String>> {
    validate_refs(seeds, "artifact impact seed ref")?;
    let db = ensure_index_tables(root)?;
    let mut impacted: std::collections::BTreeSet<String> = seeds.iter().cloned().collect();
    let mut frontier: Vec<String> = seeds.to_vec();
    while let Some(current) = frontier.pop() {
        let dependents = {
            let read_txn = db.begin_read().map_err(index_error)?;
            let reverse = read_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
            if let Some(bytes) = reverse.get(current.as_str()).map_err(index_error)? {
                parse_refs_value(&parse_canonical_bytes(bytes.value())?, "reverse")?
            } else {
                Vec::new()
            }
        };
        for dependent in dependents {
            if impacted.insert(dependent.clone()) {
                push_bounded(&mut frontier, dependent, MAX_ARTIFACT_RECORDS, "artifact impact frontier")?;
            }
        }
    }
    Ok(impacted.into_iter().collect())
}
