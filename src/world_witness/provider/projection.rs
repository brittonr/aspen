pub(super) fn checkpoint_id(
    provider: &[u8; 32],
    namespace: &release_channel_index::model::Namespace,
    snapshot: &release_channel_index::model::Snapshot,
) -> Result<String, super::types::Error> {
    let pending = snapshot.pending.as_ref().map_or_else(
        || crate::preserves_rail::record("none", vec![]),
        |pending| {
            crate::preserves_rail::record("pending", vec![
                crate::preserves_rail::string(hex(&pending.reservation)),
                claim(Some(&pending.claim)),
                crate::preserves_rail::string(match pending.status {
                    release_channel_index::model::PendingStatus::Reserved => "reserved",
                    release_channel_index::model::PendingStatus::Orphan => "orphan",
                }),
            ])
        },
    );
    let projection = crate::preserves_rail::record("world-witness-index-checkpoint-v1", vec![
        crate::preserves_rail::string(hex(provider)),
        crate::preserves_rail::record("namespace", vec![
            crate::preserves_rail::string(&namespace.channel),
            crate::preserves_rail::string(&namespace.platform),
            crate::preserves_rail::string(&namespace.track),
        ]),
        crate::preserves_rail::string(snapshot.revision.to_string()),
        claim(snapshot.highest.as_ref()),
        claim(snapshot.committed.as_ref()),
        pending,
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&projection)
        .map_err(|error| super::types::Error::invalid(error.to_string()))?;
    Ok(crate::preserves_rail::content_ref_from_bytes(&bytes))
}

fn claim(claim: Option<&release_channel_index::model::Claim>) -> preserves::IOValue {
    claim.map_or_else(
        || crate::preserves_rail::record("none", vec![]),
        |claim| {
            let predecessor = claim.predecessor.as_ref().map_or_else(
                || crate::preserves_rail::record("none", vec![]),
                |digest| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(hex(digest))]),
            );
            crate::preserves_rail::record("claim", vec![
                crate::preserves_rail::string(claim.generation.to_string()),
                crate::preserves_rail::string(hex(&claim.digest)),
                predecessor,
            ])
        },
    )
}

pub(super) fn hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in bytes {
        value.push(DIGITS[usize::from(byte >> 4)] as char);
        value.push(DIGITS[usize::from(byte & 15)] as char);
    }
    value
}
