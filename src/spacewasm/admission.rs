#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Blocker {
    ProfileAbsent,
    EngineMismatch,
    Profile(String),
    ArtifactMismatch,
    Module(String),
    StaleAuthority,
    StaleResources,
    HostcallDenied(String),
    MissingEvidence,
    EvidenceArtifactMismatch,
}

/// Current scheduler facts, not evidence submitted by the guest.
pub struct Authority<'a> {
    actor: &'a str,
    generation: u64,
    authority_ref: &'a str,
    resource_ref: &'a str,
    allowed_hostcalls: Vec<String>,
}

pub struct Request<'a> {
    pub engine: &'a str,
    pub profile: Option<&'a super::profile::Profile>,
    pub bytes: &'a [u8],
    pub artifact_ref: &'a str,
    pub actor: &'a str,
    pub generation: u64,
    pub authority_ref: &'a str,
    pub resource_ref: &'a str,
    pub current: Authority<'a>,
    pub evidence: Option<&'a super::evidence::Verified>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub(super) profile: super::profile::Profile,
    pub(super) profile_ref: String,
    pub(super) artifact_ref: String,
    pub(super) actor: String,
    pub(super) generation: u64,
    pub(super) authority_ref: String,
    pub(super) resource_ref: String,
    pub(super) evidence_ref: String,
    pub(super) fixture_name: String,
    pub(super) input_ref: String,
    pub(super) hostcalls: Vec<String>,
    pub(super) memory_pages: u32,
    pub(super) table_elements: u32,
}

impl Plan {
    pub fn profile(&self) -> &super::profile::Profile {
        &self.profile
    }
    pub fn profile_ref(&self) -> &str {
        &self.profile_ref
    }
    pub fn artifact_ref(&self) -> &str {
        &self.artifact_ref
    }
    pub fn actor(&self) -> &str {
        &self.actor
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn authority_ref(&self) -> &str {
        &self.authority_ref
    }
    pub fn resource_ref(&self) -> &str {
        &self.resource_ref
    }
    pub fn evidence_ref(&self) -> &str {
        &self.evidence_ref
    }
    pub fn fixture_name(&self) -> &str {
        &self.fixture_name
    }
    pub fn input_ref(&self) -> &str {
        &self.input_ref
    }
    pub fn hostcalls(&self) -> &[String] {
        &self.hostcalls
    }
    pub fn memory_pages(&self) -> u32 {
        self.memory_pages
    }
    pub fn table_elements(&self) -> u32 {
        self.table_elements
    }
}

pub fn admit(request: Request<'_>) -> Result<Plan, Vec<Blocker>> {
    let mut blockers = Vec::new();
    if request.engine != super::profile::ID {
        blockers.push(Blocker::EngineMismatch);
    }
    let Some(profile) = request.profile else {
        blockers.push(Blocker::ProfileAbsent);
        return Err(blockers);
    };
    if let Err(error) = profile.validate() {
        blockers.push(Blocker::Profile(error));
    }
    let inspection = super::inspection::inspect(request.bytes, profile);
    let observed = match inspection {
        Ok(observed) => Some(observed),
        Err(error) => {
            blockers.push(Blocker::Module(error));
            None
        }
    };
    if observed.as_ref().is_some_and(|facts| facts.artifact_ref != request.artifact_ref) {
        blockers.push(Blocker::ArtifactMismatch);
    }
    check_current(&request, &mut blockers);
    if let Some(facts) = &observed {
        for name in &facts.hostcalls {
            if !request.current.allowed_hostcalls.contains(name) {
                blockers.push(Blocker::HostcallDenied(name.clone()));
            }
        }
    }
    match request.evidence {
        None => blockers.push(Blocker::MissingEvidence),
        Some(evidence) if evidence.artifact_ref() != request.artifact_ref => {
            blockers.push(Blocker::EvidenceArtifactMismatch);
        }
        Some(_) => {}
    }
    blockers.sort();
    blockers.dedup();
    if !blockers.is_empty() {
        return Err(blockers);
    }
    let facts = observed.ok_or_else(|| vec![Blocker::Module("inspection unavailable".into())])?;
    let evidence = request.evidence.ok_or_else(|| vec![Blocker::MissingEvidence])?;
    Ok(Plan {
        profile: profile.clone(),
        profile_ref: profile.content_ref().map_err(|error| vec![Blocker::Profile(error)])?,
        artifact_ref: facts.artifact_ref,
        actor: request.actor.into(),
        generation: request.generation,
        authority_ref: request.authority_ref.into(),
        resource_ref: request.resource_ref.into(),
        evidence_ref: evidence.evidence_ref().into(),
        hostcalls: facts.hostcalls,
        fixture_name: evidence.fixture_name().into(),
        input_ref: evidence.input_ref().into(),
        memory_pages: facts.memory_pages,
        table_elements: facts.table_elements,
    })
}

fn check_current(request: &Request<'_>, blockers: &mut Vec<Blocker>) {
    if request.actor.is_empty()
        || request.actor != request.current.actor
        || request.generation != request.current.generation
        || !is_ref(request.authority_ref)
        || request.authority_ref != request.current.authority_ref
    {
        blockers.push(Blocker::StaleAuthority);
    }
    if !is_ref(request.resource_ref) || request.resource_ref != request.current.resource_ref {
        blockers.push(Blocker::StaleResources);
    }
}

fn is_ref(value: &str) -> bool {
    value.strip_prefix("blake3:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

pub struct Current<'a> {
    pub actor: &'a str,
    pub generation: u64,
    pub authority_ref: &'a str,
    pub resource_ref: &'a str,
    pub expected_issuer: &'a str,
    pub expected_holder: &'a str,
}

pub struct Credential<'a> {
    pub hostcall: &'a str,
    pub token: &'a str,
    pub policy_json: &'a str,
}

/// Verifies live tokens and local policy; submitted receipt refs cannot mint grants.
pub fn verify_authority<'a>(
    current: Current<'a>,
    credentials: &[Credential<'_>],
    services: &basalt::ucan_enforcement::UcanEnforcementBoundaries<'_>,
) -> Result<Authority<'a>, String> {
    if credentials.len() > 2 || !is_ref(current.authority_ref) || !is_ref(current.resource_ref) {
        return Err("invalid current authority envelope".into());
    }
    let resource = format!("molten://spacewasm/{}/{}", current.actor, current.generation);
    let mut allowed_hostcalls = Vec::new();
    for credential in credentials {
        if !matches!(credential.hostcall, "assert" | "send")
            || credential.token.len() > 65536
            || credential.policy_json.len() > 65536
        {
            return Err("invalid bounded hostcall credential".into());
        }
        let verified = basalt::ucan::verify_compact_token_str_with_resolvers_and_revocations(
            credential.token,
            services.verification_time,
            services.key_resolver,
            services.proof_store,
            services.revocations,
            services.verification_limits,
        )
        .map_err(|_| "invalid UCAN")?;
        if verified.issuer().as_str() != current.expected_issuer
            || verified.audience().as_str() != current.expected_holder
        {
            return Err("UCAN issuer or holder differs from trusted scheduler context".into());
        }
        let matched = match verified.authorize_with_policies(&resource, credential.hostcall, services.caveat_policies) {
            basalt::ucan::AuthorizationDecision::Allowed { matched_capability } => matched_capability,
            basalt::ucan::AuthorizationDecision::Denied { .. } => return Err("UCAN hostcall denied".into()),
        };
        let policy = basalt::parse_policy_json(credential.policy_json).map_err(|_| "invalid Basalt policy")?;
        let request = basalt::EnforcementRequest::new(super::profile::ID, &resource, credential.hostcall)
            .with_capabilities(vec![basalt::CapabilityGrant::new(
                matched.resource.clone(),
                matched.ability.clone(),
            )]);
        if !basalt::enforce(&policy, &request).map_err(|_| "Basalt evaluation failed")?.is_allowed() {
            return Err("Basalt hostcall denied".into());
        }
        allowed_hostcalls.push(credential.hostcall.into());
    }
    allowed_hostcalls.sort();
    allowed_hostcalls.dedup();
    Ok(Authority {
        actor: current.actor,
        generation: current.generation,
        authority_ref: current.authority_ref,
        resource_ref: current.resource_ref,
        allowed_hostcalls,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn never_falls_back_when_profile_absent() {
        let current = super::Authority {
            actor: "actor",
            generation: 1,
            authority_ref: "",
            resource_ref: "",
            allowed_hostcalls: Vec::new(),
        };
        let result = super::admit(super::Request {
            engine: "wasmtime",
            profile: None,
            bytes: &[],
            artifact_ref: "",
            actor: "actor",
            generation: 1,
            authority_ref: "",
            resource_ref: "",
            current,
            evidence: None,
        });
        assert_eq!(result, Err(vec![super::Blocker::EngineMismatch, super::Blocker::ProfileAbsent]));
    }

    #[test]
    fn aggregates_currentness_and_missing_external_evidence_deterministically() {
        let profile = super::super::profile::Profile::default();
        let evaluate = || {
            let current = super::Authority {
                actor: "actor",
                generation: 2,
                authority_ref: "",
                resource_ref: "",
                allowed_hostcalls: Vec::new(),
            };
            super::admit(super::Request {
                engine: super::super::profile::ID,
                profile: Some(&profile),
                bytes: &[],
                artifact_ref: "",
                actor: "actor",
                generation: 1,
                authority_ref: "",
                resource_ref: "",
                current,
                evidence: None,
            })
        };
        let first = evaluate();
        assert_eq!(first, evaluate());
        let blockers = first.expect_err("must deny");
        assert!(blockers.contains(&super::Blocker::StaleAuthority));
        assert!(blockers.contains(&super::Blocker::StaleResources));
        assert!(blockers.contains(&super::Blocker::MissingEvidence));
    }
}
