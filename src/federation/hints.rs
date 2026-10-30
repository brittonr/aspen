pub struct LocatorInput<'a> {
    pub peer: &'a molten_core::federation::PeerHint,
    pub subject_ref: &'a str,
    pub now: u64,
    pub evidence_refs: &'a [String],
}

pub struct HintEvidence {
    pub decision: &'static str,
    pub evidence_ref: String,
    pub value: preserves::IOValue,
}

// r[impl molten.federated_pull_runtime.discovery_hints]
pub fn observe_locator(input: LocatorInput<'_>) -> crate::error::Result<HintEvidence> {
    crate::preserves_rail::validate_content_ref(&input.peer.peer_ref)?;
    crate::preserves_rail::validate_content_ref(&input.peer.signer_ref)?;
    crate::preserves_rail::validate_content_ref(input.subject_ref)?;
    if input.evidence_refs.len() > 64 {
        return Err(crate::error::Failure::invalid_harness("locator evidence bound exceeded"));
    }
    for reference in input.evidence_refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let source = match input.peer.source {
        molten_core::federation::LocatorSource::Static => "static",
        molten_core::federation::LocatorSource::Endpoint => "endpoint",
        molten_core::federation::LocatorSource::Gossip => "gossip",
        molten_core::federation::LocatorSource::Tracker => "tracker",
        molten_core::federation::LocatorSource::Dht => "dht",
        molten_core::federation::LocatorSource::Pkarr => "pkarr",
        molten_core::federation::LocatorSource::Probe => "probe",
        molten_core::federation::LocatorSource::Catalog => "catalog",
    };
    let freshness = if input.peer.observed_at <= input.now && input.now <= input.peer.expires_at {
        "fresh"
    } else {
        "stale"
    };
    let value = crate::preserves_rail::record("federation-locator-hint-v1", vec![
        crate::preserves_rail::string(&input.peer.peer_ref),
        crate::preserves_rail::string(&input.peer.signer_ref),
        crate::preserves_rail::string(input.subject_ref),
        crate::preserves_rail::string(source),
        crate::preserves_rail::string(freshness),
        crate::preserves_rail::u64_value(input.peer.observed_at),
        crate::preserves_rail::u64_value(input.peer.expires_at),
        crate::preserves_rail::sequence(input.evidence_refs.iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::string("hint-only; no import, pin, install, execute, merge, or local trust authority"),
    ]);
    Ok(HintEvidence {
        decision: "hint-only",
        evidence_ref: crate::preserves_rail::canonical_hash(&value)?,
        value,
    })
}

// r[impl molten.federated_pull_runtime.discovery_hints]
pub fn reject_locator_import(hint: &HintEvidence) -> crate::error::Result<HintEvidence> {
    crate::preserves_rail::validate_content_ref(&hint.evidence_ref)?;
    let value = crate::preserves_rail::record("federation-locator-import-denial-v1", vec![
        crate::preserves_rail::string("deny"),
        crate::preserves_rail::string(&hint.evidence_ref),
        crate::preserves_rail::string(
            "locator evidence cannot replace receiver planning, verification or local admission",
        ),
    ]);
    Ok(HintEvidence {
        decision: "deny",
        evidence_ref: crate::preserves_rail::canonical_hash(&value)?,
        value,
    })
}
