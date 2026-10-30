pub fn encode(session: &molten_core::federation::Session) -> preserves::IOValue {
    crate::preserves_rail::record("federation-session-state-v1", vec![
        text(&session.session_ref),
        number(session.generation),
        number(session.attempts.into()),
        text(match session.phase {
            molten_core::federation::SessionPhase::Planned => "planned",
            molten_core::federation::SessionPhase::Fetching => "fetching",
            molten_core::federation::SessionPhase::Verified => "verified",
            molten_core::federation::SessionPhase::Admitted => "admitted",
            molten_core::federation::SessionPhase::Committed => "committed",
            molten_core::federation::SessionPhase::Denied => "denied",
        }),
        text(&session.plan.peer_ref),
        text(&session.plan.signer_ref),
        number(session.plan.inventory_issued_at),
        number(session.plan.inventory_expires_at),
        number(session.plan.deadline),
        number(session.plan.bytes),
        number(session.plan.dag_nodes),
        number(session.plan.chunks),
        sequence(session.plan.missing.iter().map(entry).collect()),
        sequence(
            session
                .plan
                .conflicts
                .iter()
                .map(|conflict| {
                    crate::preserves_rail::record("conflict", vec![
                        text(&conflict.name),
                        text(&conflict.local_ref),
                        entry(&conflict.remote),
                    ])
                })
                .collect(),
        ),
        refs(&session.verified),
        refs(&session.admitted),
        refs(&session.committed),
        refs(&session.evidence_refs),
    ])
}

pub fn decode(
    value: &preserves::IOValue,
    bounds: &molten_core::federation::PullBounds,
) -> crate::error::Result<molten_core::federation::Session> {
    let fields = value.collect_simple_record("federation-session-state-v1", Some(18)).ok_or_else(invalid)?;
    let generation = integer(&fields[1])?;
    let phase = match string(&fields[3])?.as_str() {
        "planned" => molten_core::federation::SessionPhase::Planned,
        "fetching" => molten_core::federation::SessionPhase::Fetching,
        "verified" => molten_core::federation::SessionPhase::Verified,
        "admitted" => molten_core::federation::SessionPhase::Admitted,
        "committed" => molten_core::federation::SessionPhase::Committed,
        "denied" => molten_core::federation::SessionPhase::Denied,
        _ => return Err(invalid()),
    };
    let missing = items(&fields[12], bounds.inventory_entries)?
        .iter()
        .map(decode_entry)
        .collect::<crate::error::Result<Vec<_>>>()?;
    let conflicts = items(&fields[13], bounds.inventory_entries)?
        .iter()
        .map(|value| {
            let parts = value.collect_simple_record("conflict", Some(3)).ok_or_else(invalid)?;
            Ok(molten_core::federation::Conflict {
                name: string(&parts[0])?,
                local_ref: reference(&parts[1])?,
                remote: decode_entry(&parts[2])?,
            })
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    let session = molten_core::federation::Session {
        session_ref: reference(&fields[0])?,
        generation,
        attempts: u32::try_from(integer(&fields[2])?).map_err(|_| invalid())?,
        phase,
        plan: molten_core::federation::PullPlan {
            peer_ref: reference(&fields[4])?,
            signer_ref: reference(&fields[5])?,
            inventory_issued_at: integer(&fields[6])?,
            inventory_expires_at: integer(&fields[7])?,
            generation,
            deadline: integer(&fields[8])?,
            bytes: integer(&fields[9])?,
            dag_nodes: integer(&fields[10])?,
            chunks: integer(&fields[11])?,
            missing,
            conflicts,
        },
        verified: decode_refs(&fields[14], bounds.inventory_entries)?,
        admitted: decode_refs(&fields[15], bounds.inventory_entries)?,
        committed: decode_refs(&fields[16], bounds.inventory_entries)?,
        evidence_refs: decode_refs(&fields[17], bounds.retained_status)?,
    };
    if session.plan.bytes > bounds.fetch_bytes
        || session.plan.dag_nodes > bounds.dag_nodes
        || session.plan.chunks > bounds.content_chunks
        || session.attempts > bounds.retries.saturating_add(1)
        || session
            .verified
            .iter()
            .any(|head| !session.plan.missing.iter().any(|entry| &entry.head_ref == head))
        || session.admitted.iter().any(|head| !session.verified.contains(head))
        || session.committed.iter().any(|head| !session.admitted.contains(head))
    {
        return Err(invalid());
    }
    Ok(session)
}

fn entry(entry: &molten_core::federation::InventoryEntry) -> preserves::IOValue {
    crate::preserves_rail::record("entry", vec![
        text(&entry.name),
        text(&entry.head_ref),
        text(&entry.ancestry_ref),
        text(&entry.policy_ref),
        number(entry.bytes),
        number(entry.dag_nodes),
        number(entry.chunks),
    ])
}
fn decode_entry(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<molten_core::federation::InventoryEntry> {
    let fields = value.collect_simple_record("entry", Some(7)).ok_or_else(invalid)?;
    let name = string(&fields[0])?;
    if name.is_empty() || name.len() > 256 {
        return Err(invalid());
    }
    Ok(molten_core::federation::InventoryEntry {
        name,
        head_ref: reference(&fields[1])?,
        ancestry_ref: reference(&fields[2])?,
        policy_ref: reference(&fields[3])?,
        bytes: integer(&fields[4])?,
        dag_nodes: integer(&fields[5])?,
        chunks: integer(&fields[6])?,
    })
}
pub(super) fn items(
    value: &preserves::Value<preserves::IOValue>,
    bound: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let values = value.collect_sequence().ok_or_else(invalid)?;
    if values.len() > bound {
        return Err(invalid());
    }
    Ok(values.into_owned())
}
fn decode_refs(value: &preserves::Value<preserves::IOValue>, bound: usize) -> crate::error::Result<Vec<String>> {
    items(value, bound)?.iter().map(reference).collect()
}
fn refs(values: &[String]) -> preserves::IOValue {
    sequence(values.iter().map(|value| text(value)).collect())
}
fn text(value: &str) -> preserves::IOValue {
    crate::preserves_rail::string(value)
}
fn number(value: u64) -> preserves::IOValue {
    crate::preserves_rail::u64_value(value)
}
fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values)
}
pub(super) fn string(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    value.as_string().map(|value| value.to_string()).ok_or_else(invalid)
}
pub(super) fn integer(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    value.as_u64().ok_or_else(invalid)?.map_err(|_| invalid())
}
fn reference(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    let reference = string(value)?;
    crate::preserves_rail::validate_content_ref(&reference)?;
    Ok(reference)
}
fn invalid() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("invalid bounded federation checkpoint")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(digit: char) -> String {
        format!("blake3:{}", digit.to_string().repeat(64))
    }
    fn bounds() -> molten_core::federation::PullBounds {
        molten_core::federation::PullBounds {
            inventory_entries: 4,
            inventory_bytes: 4096,
            fetch_bytes: 1024,
            dag_nodes: 4,
            content_chunks: 4,
            concurrent_sessions: 2,
            query_interval: 10,
            freshness: 20,
            session_ticks: 30,
            retries: 1,
            retained_status: 16,
        }
    }
    fn session() -> molten_core::federation::Session {
        molten_core::federation::Session {
            session_ref: reference('1'),
            generation: 7,
            attempts: 2,
            phase: molten_core::federation::SessionPhase::Fetching,
            plan: molten_core::federation::PullPlan {
                peer_ref: reference('2'),
                signer_ref: reference('3'),
                generation: 7,
                inventory_issued_at: 10,
                inventory_expires_at: 40,
                deadline: 50,
                bytes: 64,
                dag_nodes: 1,
                chunks: 1,
                missing: vec![molten_core::federation::InventoryEntry {
                    name: "document".to_string(),
                    head_ref: reference('4'),
                    ancestry_ref: reference('5'),
                    policy_ref: reference('6'),
                    bytes: 64,
                    dag_nodes: 1,
                    chunks: 1,
                }],
                conflicts: Vec::new(),
            },
            verified: vec![reference('4')],
            admitted: Vec::new(),
            committed: Vec::new(),
            evidence_refs: vec![reference('7')],
        }
    }

    #[test]
    fn distributed_federation_resume_preserves_retry_and_generation_fences() {
        let original = session();
        let bytes = crate::preserves_rail::canonical_bytes(&encode(&original)).unwrap();
        let value = crate::preserves_rail::parse_canonical_bytes(&bytes).unwrap();
        let recovered = decode(&value, &bounds()).unwrap();
        assert_eq!(recovered, original);
        assert_eq!(
            molten_core::federation::advance_session(
                &recovered,
                &molten_core::federation::SessionEvent::BeginFetch,
                7,
                30,
                &bounds()
            ),
            Err(molten_core::federation::SessionIssue::Retry)
        );
        assert_eq!(
            molten_core::federation::advance_session(
                &recovered,
                &molten_core::federation::SessionEvent::BeginFetch,
                8,
                30,
                &bounds()
            ),
            Err(molten_core::federation::SessionIssue::Fence)
        );
    }

    #[test]
    fn distributed_federation_resume_rejects_unverified_commit_and_resource_drift() {
        let mut corrupt = session();
        corrupt.committed.push(reference('4'));
        assert!(decode(&encode(&corrupt), &bounds()).is_err());
        corrupt = session();
        corrupt.plan.bytes = 1025;
        assert!(decode(&encode(&corrupt), &bounds()).is_err());
        corrupt = session();
        corrupt.verified.push(reference('8'));
        assert!(decode(&encode(&corrupt), &bounds()).is_err());
    }
}
