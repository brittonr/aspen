//! Current-generation, authenticated public endpoint readiness. Experimental only.
pub mod current;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointProof {
    pub schema: String,
    pub node_id: String,
    pub endpoint: String,
    pub run_id: String,
    pub profile: super::profile::Register,
}

pub struct Input<'a> {
    pub root: &'a crate::node_state::DirectoryRoot,
    pub node_id: &'a str,
    pub source_binding_ref: &'a str,
    pub health_path: &'a str,
    pub descriptor: &'a EndpointProof,
}

#[derive(Debug, serde::Serialize)]
pub struct Verified {
    pub schema: &'static str,
    pub node_id: String,
    pub endpoint: String,
    pub grant_ref: String,
    pub generation: u64,
    pub source_binding_ref: String,
    pub startup_ref: String,
    pub health_ref: String,
    pub probe_ref: String,
    pub signed_probe: super::authority::SignedProbe,
    pub experimental_ready: bool,
    pub production_admitted: bool,
}

pub fn verify(input: Input<'_>) -> crate::error::Result<Verified> {
    let grant = super::authority::load(input.root)?;
    validate_descriptor(&grant, &input)?;
    let before = current::read(input.root, input.health_path)?;
    if before.source_binding_ref != input.source_binding_ref {
        return Err(denied("readiness source binding differs from independently expected source"));
    }
    let mut nonce = [0; 32];
    crate::fabric_time::CryptographicEntropySource::fill_secret(
        &mut crate::fabric_time::OperatingSystemEntropySource,
        &mut nonce,
    )
    .map_err(|error| denied(format!("readiness nonce entropy: {error:?}")))?;
    let challenge = super::authority::Challenge {
        nonce,
        node_id: input.node_id.into(),
        endpoint: input.descriptor.endpoint.clone(),
        source_binding_ref: before.source_binding_ref.clone(),
        startup_ref: before.startup_ref.clone(),
        health_ref: before.health_ref.clone(),
    };
    let bytes = serde_json::to_vec(&challenge).map_err(|error| denied(error.to_string()))?;
    input
        .root
        .write(&crate::node_state::RelativePath::parse("onixos-readiness-challenge.json")?, &bytes)?;
    let signed = probe(input.descriptor, &challenge)?;
    let state = super::authority::verify_probe(&grant, &signed, &challenge)?;
    if !state.ready {
        return Err(denied("actual public endpoint is not ready"));
    }
    grant.verify_current(input.root)?;
    if current::read(input.root, input.health_path)? != before {
        return Err(denied("node generation changed during probe"));
    }
    Ok(Verified {
        schema: "molten.experimental-readiness.v1",
        node_id: input.node_id.into(),
        endpoint: input.descriptor.endpoint.clone(),
        grant_ref: grant.grant_ref().into(),
        generation: state.generation,
        source_binding_ref: before.source_binding_ref,
        startup_ref: before.startup_ref,
        health_ref: before.health_ref,
        probe_ref: super::profile::canonical_ref(&signed).map_err(|_| denied("probe encoding"))?,
        signed_probe: signed,
        experimental_ready: true,
        production_admitted: false,
    })
}

fn validate_descriptor(grant: &super::authority::VerifiedGrant, input: &Input<'_>) -> crate::error::Result<()> {
    let descriptor = input.descriptor;
    if descriptor.schema != "molten.coordination-endpoint.v1"
        || descriptor.node_id != input.node_id
        || grant.node_id() != input.node_id
        || descriptor.endpoint != grant.endpoint()
        || grant.source_binding_ref() != input.source_binding_ref
        || !grant.allows("probe")
    {
        return Err(denied("endpoint descriptor differs from independently admitted grant"));
    }
    let admission = &grant.configuration()["admission"];
    if admission["run_id"].as_str() != Some(descriptor.run_id.as_str())
        || admission["profile"]
            != serde_json::to_value(&descriptor.profile).map_err(|error| denied(error.to_string()))?
    {
        return Err(denied("endpoint descriptor substitutes signed workload"));
    }
    Ok(())
}

fn probe(
    descriptor: &EndpointProof,
    challenge: &super::authority::Challenge,
) -> crate::error::Result<super::authority::SignedProbe> {
    let request = super::protocol::Request {
        schema: super::protocol::SCHEMA.into(),
        phase: super::protocol::Phase::Probe,
        run_id: descriptor.run_id.clone(),
        endpoint: descriptor.endpoint.clone(),
        operation: None,
        deadline_ms: descriptor.profile.recovery.timeout_ms.min(120_000),
        profile: Some(descriptor.profile.clone()),
        authorization: None,
        probe: Some(challenge.clone()),
    };
    let response = super::client::execute(&request);
    if response.outcome != super::protocol::Outcome::Ok {
        return Err(denied("authenticated live probe incomplete"));
    }
    serde_json::from_value(response.value).map_err(|error| denied(format!("invalid signed probe: {error}")))
}

fn denied(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
