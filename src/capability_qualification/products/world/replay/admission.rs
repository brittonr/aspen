//! Current replay admission and the signed execution token it admits.

use ed25519_dalek::Signer;
use ed25519_dalek::Verifier;

use crate::world_replay::WorldReplayAdmissionPort;

const AUTHORITY_PATH: &str = "storage/world-replay/current-authority.preserves";
const AUTHORITY_RECORD: &str = "aspen-qualification-replay-authority-v1";
const TOKEN_KIND: &str = "world-replay-execution";
const REPLAY_ABILITY: &str = "world/replay/execute";
const REPLAY_SCOPE: &str = "runtime-spine-committed-turn-v1";
const MAX_REPLAY_TURNS: usize = 16;
const MAX_AUTHORITY_BYTES: u64 = 65_536;

/// Current replay admission, observed when the replay service asks for it:
/// authority from capability admission of an Ed25519-signed token against the
/// current revocation document, plus artifact, schema, resource, runtime and
/// effect facts of this process and this capsule.
pub(super) struct CurrentAuthority {
    pub(super) destination: super::ports::Shared,
    pub(super) issuer: ed25519_dalek::VerifyingKey,
    pub(super) token: crate::capability_tokens::CapabilityToken,
    pub(super) signature: ed25519_dalek::Signature,
    pub(super) boundary: molten_core::world_replay::WorldReplayObservationBoundary,
    pub(super) receipts: Vec<preserves::IOValue>,
}

impl CurrentAuthority {
    fn revocations(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<(u64, Vec<String>)> {
        use preserves::ValueImpl as _;
        let bytes = root.read(&crate::node_state::RelativePath::parse(AUTHORITY_PATH)?, MAX_AUTHORITY_BYTES)?;
        let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
        let fields = crate::preserves_rail::simple_record_fields(&value, AUTHORITY_RECORD, 2)?;
        let generation = fields[0]
            .as_u64()
            .ok_or_else(|| super::super::invalid("current authority generation missing"))?
            .map_err(|error| super::super::invalid(format!("current authority generation: {error}")))?;
        let revoked = fields[1]
            .collect_sequence()
            .ok_or_else(|| super::super::invalid("current revocations must be a sequence"))?
            .as_slice()
            .iter()
            .map(|reference| crate::preserves_rail::required_string_field(reference, "revocation ref"))
            .collect::<crate::error::Result<Vec<_>>>()?;
        Ok((generation, revoked))
    }

    pub(super) fn publish(
        root: &crate::node_state::DirectoryRoot,
        generation: u64,
        revoked: &[String],
    ) -> crate::error::Result<()> {
        let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(AUTHORITY_RECORD, vec![
            crate::preserves_rail::u64_value(generation),
            crate::preserves_rail::sequence(revoked.iter().map(crate::preserves_rail::string).collect()),
        ]))?;
        root.create_dir_all(&crate::node_state::RelativePath::parse("storage/world-replay")?)?;
        root.write(&crate::node_state::RelativePath::parse(AUTHORITY_PATH)?, &bytes)
    }

    fn signature_verified(&self) -> crate::error::Result<bool> {
        let bytes =
            crate::preserves_rail::canonical_bytes(&crate::capability_tokens::capability_token_value(&self.token)?)?;
        Ok(self.token.issuer_ref == super::super::runtime::key_ref(&self.issuer)
            && self.issuer.verify(&bytes, &self.signature).is_ok())
    }
}

impl WorldReplayAdmissionPort for CurrentAuthority {
    fn observe_current(
        &mut self,
        trace: &molten_core::world_replay::WorldTransitionTrace,
        capsule: &molten_core::world_replay::WorldReplayCapsule,
    ) -> crate::error::Result<crate::world_replay::WorldReplayAdmissionObservation> {
        let destination = self.destination.borrow();
        let (generation, revoked) = Self::revocations(&destination.node.root)?;
        let proofset = crate::capability_tokens::CapabilityProofset {
            holder_ref: self.token.holder_ref.clone(),
            session_ref: self.token.session_ref.clone(),
            context_ref: self.token.context_ref.clone(),
            tokens: vec![self.token.clone()],
            policy_refs: vec![super::super::runtime::policy_ref()],
            resource_refs: Vec::new(),
            revocation_refs: revoked,
            evidence_refs: Vec::new(),
        };
        let request = crate::capability_tokens::CapabilityRequest {
            holder_ref: self.token.holder_ref.clone(),
            session_ref: self.token.session_ref.clone(),
            context_ref: capsule.capsule_ref.clone(),
            resource_ref: trace.trace_ref.clone(),
            ability: REPLAY_ABILITY.to_string(),
            scope: REPLAY_SCOPE.to_string(),
            at_tick: generation,
            required_policy_refs: vec![super::super::runtime::policy_ref()],
            required_resource_refs: Vec::new(),
            required_token_kind: Some(TOKEN_KIND.to_string()),
            caveat_context: Vec::new(),
        };
        let admission = crate::capability_tokens::admit_capability(&proofset, &request)?;
        let is_authority_admitted = admission.decision == "pass" && self.signature_verified()?;
        let observed_cohort = destination.node.adapter()?.observed_cohort().cohort_ref.clone();
        let is_artifact_admitted = observed_cohort == destination.descriptor.cohort.cohort_ref;
        let is_schema_admitted =
            trace.steps.iter().all(|step| step.input.schema_ref == super::history::step_schema_ref());
        let is_resource_admitted = trace.steps.len() <= MAX_REPLAY_TURNS;
        let is_runtime_admitted = trace.profile.profile_ref.as_str()
            == crate::world_snapshot::local_logical_profile_ref()?
            && trace.profile.cohort_ref.is_none();
        let is_effect_admitted = self.boundary.ledger.reached_sources.iter().all(|source_id: &String| {
            self.boundary.inventory.source(source_id).is_some_and(|source| !source.is_external_mutation)
        });
        let facts = [
            is_authority_admitted,
            is_artifact_admitted,
            is_schema_admitted,
            is_resource_admitted,
            is_runtime_admitted,
            is_effect_admitted,
        ];
        let observation_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "aspen-qualification-replay-admission-v1",
            vec![
                crate::preserves_rail::string(&trace.trace_ref),
                crate::preserves_rail::string(&capsule.capsule_ref),
                crate::preserves_rail::string(&admission.receipt_ref),
                crate::preserves_rail::u64_value(generation),
                crate::preserves_rail::sequence(
                    facts.iter().map(|held| crate::preserves_rail::bool_value(*held)).collect(),
                ),
            ],
        ))?;
        self.receipts.push(admission.value);
        Ok(crate::world_replay::WorldReplayAdmissionObservation {
            observation_ref,
            trace_ref: trace.trace_ref.clone(),
            capsule_ref: capsule.capsule_ref.clone(),
            profile_ref: trace.profile.profile_ref.as_str().to_string(),
            generation,
            authority_admitted: is_authority_admitted,
            artifact_admitted: is_artifact_admitted,
            schema_admitted: is_schema_admitted,
            resource_admitted: is_resource_admitted,
            runtime_admitted: is_runtime_admitted,
            effect_admitted: is_effect_admitted,
        })
    }
}

/// Sign the replay execution token the current authority admits.
pub(super) fn execution_token(
    issuer: &ed25519_dalek::SigningKey,
    request: &molten_core::world_replay::WorldReplayPlanRequest,
) -> crate::error::Result<(crate::capability_tokens::CapabilityToken, ed25519_dalek::Signature)> {
    let token = crate::capability_tokens::CapabilityToken {
        token_kind: TOKEN_KIND.to_string(),
        issuer_ref: super::super::runtime::key_ref(&issuer.verifying_key()),
        holder_ref: super::super::runtime::reference("aspen qualification replay executor"),
        session_ref: super::super::runtime::reference("aspen qualification replay session"),
        context_ref: request.capsule.capsule_ref.clone(),
        resource_ref: request.trace.trace_ref.clone(),
        ability: REPLAY_ABILITY.to_string(),
        scope: REPLAY_SCOPE.to_string(),
        attenuation: "exact".to_string(),
        caveats: Vec::new(),
        expires_at_tick: super::AUTHORITY_GENERATION,
        revoked_refs: Vec::new(),
        policy_refs: vec![super::super::runtime::policy_ref()],
        resource_refs: Vec::new(),
        delegation_refs: Vec::new(),
        evidence_refs: Vec::new(),
    };
    let bytes = crate::preserves_rail::canonical_bytes(&crate::capability_tokens::capability_token_value(&token)?)?;
    let signature = issuer.sign(&bytes);
    Ok((token, signature))
}
