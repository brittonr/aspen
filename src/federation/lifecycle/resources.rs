const MAX_HISTORY_BYTES: u64 = 4 * 1024 * 1024;

pub(super) fn load(
    scope: &crate::node_state::Scope,
) -> crate::error::Result<Vec<molten_core::federation::ResourceHistory>> {
    if !scope.try_exists(&path()?)? {
        return Ok(Vec::new());
    }
    let value = crate::preserves_rail::parse_canonical_bytes(&scope.read(&path()?, MAX_HISTORY_BYTES)?)?;
    decode(&value)
}
pub(super) fn save(
    scope: &crate::node_state::Scope,
    history: &[molten_core::federation::ResourceHistory],
) -> crate::error::Result<()> {
    let bytes = crate::preserves_rail::canonical_bytes(&encode(history)?)?;
    if u64::try_from(bytes.len()).ok().is_none_or(|length| length > MAX_HISTORY_BYTES) {
        return Err(invalid());
    }
    scope.write(&path()?, &bytes)
}
pub(in crate::mesh) fn encode(
    history: &[molten_core::federation::ResourceHistory],
) -> crate::error::Result<preserves::IOValue> {
    validate(history)?;
    Ok(crate::preserves_rail::record("federation-resource-history-v1", vec![
        crate::preserves_rail::sequence(
            history
                .iter()
                .map(|item| {
                    crate::preserves_rail::record("resource", vec![
                        crate::preserves_rail::string(&item.head_ref),
                        crate::preserves_rail::u64_value(item.window_started),
                        crate::preserves_rail::u64_value(item.last_query),
                        crate::preserves_rail::u64_value(item.attempts.into()),
                        crate::preserves_rail::u64_value(item.bytes),
                        crate::preserves_rail::u64_value(item.dag_nodes),
                        crate::preserves_rail::u64_value(item.chunks),
                    ])
                })
                .collect(),
        ),
    ]))
}
pub(in crate::mesh) fn decode(
    value: &preserves::IOValue,
) -> crate::error::Result<Vec<molten_core::federation::ResourceHistory>> {
    let fields = value.collect_simple_record("federation-resource-history-v1", Some(1)).ok_or_else(invalid)?;
    let history = super::checkpoint::items(&fields[0], molten_core::federation::MAX_FEDERATION_ENTRIES)?
        .iter()
        .map(|value| {
            let parts = value.collect_simple_record("resource", Some(7)).ok_or_else(invalid)?;
            Ok(molten_core::federation::ResourceHistory {
                head_ref: super::checkpoint::string(&parts[0])?,
                window_started: super::checkpoint::integer(&parts[1])?,
                last_query: super::checkpoint::integer(&parts[2])?,
                attempts: u32::try_from(super::checkpoint::integer(&parts[3])?).map_err(|_| invalid())?,
                bytes: super::checkpoint::integer(&parts[4])?,
                dag_nodes: super::checkpoint::integer(&parts[5])?,
                chunks: super::checkpoint::integer(&parts[6])?,
            })
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    validate(&history)?;
    Ok(history)
}
fn validate(history: &[molten_core::federation::ResourceHistory]) -> crate::error::Result<()> {
    if history.len() > molten_core::federation::MAX_FEDERATION_ENTRIES
        || history.windows(2).any(|pair| pair[0].head_ref >= pair[1].head_ref)
        || history.iter().any(|item| {
            !molten_core::federation::canonical_ref(&item.head_ref)
                || item.window_started > item.last_query
                || item.attempts == 0
        })
    {
        return Err(invalid());
    }
    Ok(())
}
fn path() -> crate::error::Result<crate::node_state::RelativePath> {
    crate::node_state::RelativePath::parse("federation-resources.preserves")
}
fn invalid() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("invalid bounded federation resource history")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distributed_federation_resource_checkpoint_preserves_cross_peer_exhaustion() {
        let head = format!("blake3:{}", "1".repeat(64));
        let history = vec![molten_core::federation::ResourceHistory {
            head_ref: head.clone(),
            window_started: 10,
            last_query: 20,
            attempts: 2,
            bytes: 128,
            dag_nodes: 2,
            chunks: 2,
        }];
        let bytes = crate::preserves_rail::canonical_bytes(&encode(&history).unwrap()).unwrap();
        let restored = decode(&crate::preserves_rail::parse_canonical_bytes(&bytes).unwrap()).unwrap();
        assert_eq!(restored, history);
        let bounds = molten_core::federation::PullBounds {
            inventory_entries: 1,
            inventory_bytes: 4096,
            fetch_bytes: 1024,
            dag_nodes: 4,
            content_chunks: 4,
            concurrent_sessions: 2,
            query_interval: 10,
            freshness: 40,
            session_ticks: 40,
            retries: 1,
            retained_status: 16,
        };
        let plan = molten_core::federation::PullPlan {
            peer_ref: format!("blake3:{}", "2".repeat(64)),
            signer_ref: format!("blake3:{}", "3".repeat(64)),
            generation: 2,
            inventory_issued_at: 20,
            inventory_expires_at: 60,
            deadline: 60,
            bytes: 64,
            dag_nodes: 1,
            chunks: 1,
            conflicts: Vec::new(),
            missing: vec![molten_core::federation::InventoryEntry {
                name: "artifact".to_string(),
                head_ref: head.clone(),
                ancestry_ref: head.clone(),
                policy_ref: head,
                bytes: 64,
                dag_nodes: 1,
                chunks: 1,
            }],
        };
        assert_eq!(
            molten_core::federation::reserve_resources(&restored, &plan, &bounds, 30),
            Err(molten_core::federation::PullIssue::ResourceRetry)
        );
        let mut duplicate = restored.clone();
        duplicate.push(restored[0].clone());
        assert!(encode(&duplicate).is_err());
    }
}
