use super::*;

fn reference<D: molten_core::nominal::ReferenceDomain>(
    value: &preserves::IOValue,
) -> crate::error::Result<molten_core::nominal::CanonicalRef<D>> {
    molten_core::nominal::CanonicalRef::new(decode::string(value)?).map_err(|_| denied(Denial::Domain))
}

pub fn parse_audience(value: &preserves::IOValue) -> crate::error::Result<AudienceRequest> {
    let f = decode::fields(value, "scope-audience-request-v1", 15)?;
    let mut audience = std::collections::BTreeSet::new();
    for value in decode::items(&f[5], MAX_MEMBERS)? {
        if !audience.insert(reference(&value)?) {
            return Err(denied(Denial::Conflict));
        }
    }
    if audience.is_empty() {
        return Err(denied(Denial::Bound));
    }
    Ok(AudienceRequest {
        reference: RequestRef::new(crate::preserves_rail::canonical_hash(value)?)
            .map_err(|_| denied(Denial::Domain))?,
        scope: reference(&f[0])?,
        scope_revision: decode::integer(&f[1])?,
        membership_epoch: decode::integer(&f[2])?,
        binding: reference(&f[3])?,
        resource: reference(&f[4])?,
        audience,
        ability: parse_ability(&f[6])?,
        target: decode::option(&f[7])?.map(ScopeRef::new).transpose().map_err(|_| denied(Denial::Domain))?,
        target_revision: decode::expiration(&f[8])?,
        target_membership_epoch: decode::expiration(&f[9])?,
        scope_policy: reference(&f[10])?,
        resource_policy: reference(&f[11])?,
        organization_floor: decode::option(&f[12])?
            .map(molten_core::nominal::PolicyRef::new)
            .transpose()
            .map_err(|_| denied(Denial::Domain))?,
        consent: decode::option(&f[13])?.map(DecisionRef::new).transpose().map_err(|_| denied(Denial::Domain))?,
        currentness: parse_currentness(&f[14])?,
    })
}

/// Parsing a snapshot establishes only its content-free shape and exact identity.
/// Protected consumers still need the current shell admission boundary.
// r[impl molten.collaboration_scope.safe_projection]
pub fn decode_snapshot(bytes: &[u8]) -> crate::error::Result<ScopeAdmissionSnapshot> {
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(denied(Denial::Bound));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    let f = decode::fields(&value, "scope-admission-snapshot-v1", 5)?;
    if decode::string(&f[4])? != "one-request-evidence-not-bearer-authority" {
        return Err(denied(Denial::Domain));
    }
    let request = parse_audience(&f[0])?;
    if request.reference != reference(&f[1])? {
        return Err(denied(Denial::Domain));
    }
    let snapshot = ScopeAdmissionSnapshot {
        request,
        state: reference(&f[2])?,
        decisions: decode::items(&f[3], MAX_MEMBERS * 5)?
            .iter()
            .map(parse_decision)
            .collect::<crate::error::Result<Vec<_>>>()?,
    };
    if crate::preserves_rail::canonical_bytes(&snapshot_value(&snapshot))? != bytes {
        return Err(denied(Denial::Domain));
    }
    Ok(snapshot)
}

pub enum Consumer {
    Lattice,
    Animus,
    Tile,
}

impl Consumer {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Lattice => "lattice-effect-attempt",
            Self::Animus => "animus-context-refs",
            Self::Tile => "tile-read-only-status",
        }
    }
}

// r[impl molten.collaboration_scope.safe_projection]
pub fn consumer_value(
    snapshot: &ScopeAdmissionSnapshot,
    consumer: Consumer,
) -> crate::error::Result<preserves::IOValue> {
    let audience = crate::preserves_rail::record("collaboration-audience-v1", vec![crate::preserves_rail::sequence(
        snapshot
            .request
            .audience
            .iter()
            .map(|subject| crate::preserves_rail::string(subject.as_str()))
            .collect(),
    )]);
    Ok(crate::preserves_rail::record("collaboration-consumer-view-v1", vec![
        crate::preserves_rail::string(consumer.as_str()),
        crate::preserves_rail::string(snapshot_ref(snapshot)?.as_str()),
        crate::preserves_rail::string(snapshot.request.scope.as_str()),
        crate::preserves_rail::string(snapshot.request.resource.as_str()),
        crate::preserves_rail::string(snapshot.request.binding.as_str()),
        crate::preserves_rail::string(snapshot.request.reference.as_str()),
        crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&audience)?),
        crate::preserves_rail::u64_value(snapshot.request.scope_revision),
        crate::preserves_rail::u64_value(snapshot.request.membership_epoch),
        crate::preserves_rail::string(snapshot.request.ability.as_str()),
        crate::preserves_rail::string("revalidate-before-effect-not-bearer-authority"),
    ]))
}
