pub struct LocalPolicy {
    pub schema: crate::preserves_rail::BoundarySchemaSpec,
    pub allowed_heads: std::collections::BTreeSet<String>,
    pub allowed_signers: std::collections::BTreeSet<String>,
    pub maximum_bytes: u64,
    pub expires_at: u64,
    pub retention_policy_ref: String,
    pub provenance_policy_ref: String,
    pub resource_profile_ref: String,
}

pub struct PolicyPort<'a> {
    policy: &'a LocalPolicy,
    policy_ref: String,
    proofset: &'a crate::capability_tokens::CapabilityProofset,
    origins: &'a std::cell::RefCell<Vec<crate::mesh::transfer::quarantine::QuarantinedObject>>,
    remaining_bytes: u64,
    evidence_values: Vec<preserves::IOValue>,
}

pub struct PolicyInput<'a, 'm> {
    pub manifest: &'m super::Manifest,
    pub policy: &'a LocalPolicy,
    pub proofset: &'a crate::capability_tokens::CapabilityProofset,
    pub origins: &'a std::cell::RefCell<Vec<crate::mesh::transfer::quarantine::QuarantinedObject>>,
}

impl<'a> PolicyPort<'a> {
    pub fn admit(input: PolicyInput<'a, '_>) -> crate::error::Result<Self> {
        let value = policy_value(input.policy)?;
        let policy_ref = crate::preserves_rail::canonical_hash(&value)?;
        if policy_ref != input.manifest.local_policy_ref()
            || input.policy.allowed_heads.len() > 4096
            || input.policy.allowed_signers.len() > 256
            || input.origins.borrow().len() > 4096
            || input.policy.maximum_bytes > input.manifest.bounds().fetch_bytes
            || input.policy.resource_profile_ref != input.manifest.resource_profile_ref()
        {
            return Err(invalid("local federation policy does not match admitted manifest or bounds"));
        }
        let proofset_ref = crate::preserves_rail::canonical_hash(
            &crate::capability_tokens::capability_proofset_value(input.proofset)?,
        )?;
        if !input.manifest.extension().manifest().capability_refs.contains(&proofset_ref) {
            return Err(invalid("federation local capability proofset is not manifest-admitted"));
        }
        Ok(Self {
            policy: input.policy,
            policy_ref,
            proofset: input.proofset,
            origins: input.origins,
            remaining_bytes: input.policy.maximum_bytes,
            evidence_values: Vec::new(),
        })
    }

    pub fn evidence_values(&self) -> &[preserves::IOValue] {
        &self.evidence_values
    }

    pub fn take_evidence(&mut self) -> Vec<preserves::IOValue> {
        std::mem::take(&mut self.evidence_values)
    }

    fn evidence(
        &mut self,
        gate: crate::mesh::transfer::import::Gate,
        subject: &crate::mesh::transfer::import::LocalSubject<'_>,
    ) -> crate::error::Result<String> {
        if subject.now > self.policy.expires_at || !self.policy.allowed_heads.contains(subject.head_ref) {
            return Err(invalid("local federation import policy expired or object is not allowed"));
        }
        match gate {
            crate::mesh::transfer::import::Gate::Schema => {
                let bytes = crate::preserves_rail::canonical_bytes(subject.artifact)?;
                let report = crate::preserves_rail::validate_boundary_bytes(&bytes, &self.policy.schema)?;
                if report.decision != "pass" || report.decoded_value_ref.as_str() != subject.head_ref {
                    return Err(invalid("local federation schema gate denied"));
                }
                Ok(report.schema_ref.as_str().to_string())
            }
            crate::mesh::transfer::import::Gate::Capability => self.capability(subject),
            crate::mesh::transfer::import::Gate::Provenance => self.provenance(subject),
            crate::mesh::transfer::import::Gate::Resource => {
                let length = u64::try_from(crate::preserves_rail::canonical_bytes(subject.artifact)?.len())
                    .map_err(|_| invalid("local import size overflow"))?;
                self.remaining_bytes = self
                    .remaining_bytes
                    .checked_sub(length)
                    .ok_or_else(|| invalid("local import resource reservation exhausted"))?;
                Ok(self.policy.resource_profile_ref.clone())
            }
            crate::mesh::transfer::import::Gate::Retention => {
                crate::preserves_rail::validate_content_ref(&self.policy.retention_policy_ref)?;
                Ok(self.policy.retention_policy_ref.clone())
            }
            crate::mesh::transfer::import::Gate::Policy | crate::mesh::transfer::import::Gate::Import => {
                Ok(self.policy_ref.clone())
            }
        }
    }

    fn capability(&self, subject: &crate::mesh::transfer::import::LocalSubject<'_>) -> crate::error::Result<String> {
        let receipt =
            crate::capability_tokens::admit_capability(self.proofset, &crate::capability_tokens::CapabilityRequest {
                holder_ref: self.proofset.holder_ref.clone(),
                session_ref: self.proofset.session_ref.clone(),
                context_ref: self.proofset.context_ref.clone(),
                resource_ref: subject.head_ref.to_string(),
                ability: "federation:import".to_string(),
                scope: "local-registry".to_string(),
                at_tick: subject.now,
                required_policy_refs: self.proofset.policy_refs.clone(),
                required_resource_refs: self.proofset.resource_refs.clone(),
                required_token_kind: None,
                caveat_context: Vec::new(),
            })?;
        if receipt.decision != "pass" {
            return Err(invalid("local federation capability admission denied"));
        }
        Ok(receipt.receipt_ref)
    }

    fn provenance(&self, subject: &crate::mesh::transfer::import::LocalSubject<'_>) -> crate::error::Result<String> {
        for object in self.origins.borrow().iter() {
            let origin = object.origin();
            let record = origin.record();
            let fields = record
                .payload
                .collect_simple_record("federation-object-v1", Some(4))
                .ok_or_else(|| invalid("provenance response payload is malformed"))?;
            if fields[1].as_string().as_deref() == Some(subject.head_ref)
                && self.policy.allowed_signers.contains(&record.signature.metadata.signer_public_ref)
                && subject.now <= record.expires_at
            {
                crate::preserves_rail::validate_content_ref(&self.policy.provenance_policy_ref)?;
                return Ok(origin.verification().outcome_ref.clone());
            }
        }
        Err(invalid("local provenance gate requires current verified response from allowed origin"))
    }
}

impl crate::mesh::transfer::import::LocalImportAdmissionPort for PolicyPort<'_> {
    fn admit(
        &mut self,
        gate: crate::mesh::transfer::import::Gate,
        subject: &crate::mesh::transfer::import::LocalSubject<'_>,
    ) -> crate::error::Result<crate::mesh::transfer::import::LocalGateObservation> {
        if self.evidence_values.len() >= 4096 {
            return Err(invalid("local admission evidence retention exhausted"));
        }
        let source_ref = self.evidence(gate, subject)?;
        let value = crate::preserves_rail::record("federation-local-admission-v1", vec![
            crate::preserves_rail::string(gate_name(gate)),
            crate::preserves_rail::string(subject.session_ref),
            crate::preserves_rail::u64_value(subject.generation),
            crate::preserves_rail::string(subject.head_ref),
            crate::preserves_rail::u64_value(subject.now),
            crate::preserves_rail::string(&self.policy_ref),
            crate::preserves_rail::string(source_ref),
        ]);
        let evidence_ref = crate::preserves_rail::canonical_hash(&value)?;
        self.evidence_values.push(value);
        Ok(crate::mesh::transfer::import::LocalGateObservation {
            gate,
            session_ref: subject.session_ref.to_string(),
            generation: subject.generation,
            head_ref: subject.head_ref.to_string(),
            admitted: true,
            evidence_ref,
        })
    }
}

pub fn policy_value(policy: &LocalPolicy) -> crate::error::Result<preserves::IOValue> {
    if policy.allowed_heads.len() > 4096 || policy.allowed_signers.len() > 256 || policy.maximum_bytes == 0 {
        return Err(invalid("local federation policy exceeds bounded profile"));
    }
    for reference in [
        &policy.retention_policy_ref,
        &policy.provenance_policy_ref,
        &policy.resource_profile_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    validate(policy)?;
    let schema_ref = crate::preserves_rail::boundary_schema_ref(&policy.schema)?;
    Ok(crate::preserves_rail::record("federation-local-policy-v1", vec![
        crate::preserves_rail::string(schema_ref.as_str()),
        crate::preserves_rail::sequence(policy.allowed_heads.iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::sequence(policy.allowed_signers.iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::u64_value(policy.maximum_bytes),
        crate::preserves_rail::u64_value(policy.expires_at),
        crate::preserves_rail::string(&policy.retention_policy_ref),
        crate::preserves_rail::string(&policy.provenance_policy_ref),
        crate::preserves_rail::string(&policy.resource_profile_ref),
    ]))
}
fn gate_name(gate: crate::mesh::transfer::import::Gate) -> &'static str {
    match gate {
        crate::mesh::transfer::import::Gate::Schema => "schema",
        crate::mesh::transfer::import::Gate::Policy => "policy",
        crate::mesh::transfer::import::Gate::Capability => "capability",
        crate::mesh::transfer::import::Gate::Provenance => "provenance",
        crate::mesh::transfer::import::Gate::Resource => "resource",
        crate::mesh::transfer::import::Gate::Retention => "retention",
        crate::mesh::transfer::import::Gate::Import => "import",
    }
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

pub fn retention_rule_ref(policy: &LocalPolicy) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("federation-retention-rule-v1", vec![
        crate::preserves_rail::string("retain-durable-value; no deletion or remote retention claim"),
        crate::preserves_rail::u64_value(policy.expires_at),
        crate::preserves_rail::u64_value(policy.maximum_bytes),
        crate::preserves_rail::sequence(policy.allowed_heads.iter().map(crate::preserves_rail::string).collect()),
    ]))
}
pub fn provenance_rule_ref(policy: &LocalPolicy) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("federation-provenance-rule-v1", vec![
        crate::preserves_rail::string("current verified response for exact head from allowed signer"),
        crate::preserves_rail::sequence(policy.allowed_signers.iter().map(crate::preserves_rail::string).collect()),
    ]))
}
pub fn resource_rule_ref(maximum_bytes: u64) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("federation-import-resource-rule-v1", vec![
        crate::preserves_rail::u64_value(maximum_bytes),
        crate::preserves_rail::string(
            "checked cumulative bytes; reservation exhaustion denies before registry mutation",
        ),
    ]))
}

fn validate(policy: &LocalPolicy) -> crate::error::Result<()> {
    if policy.retention_policy_ref != retention_rule_ref(policy)?
        || policy.provenance_policy_ref != provenance_rule_ref(policy)?
        || policy.resource_profile_ref != resource_rule_ref(policy.maximum_bytes)?
    {
        return Err(crate::error::Failure::invalid_harness(
            "local federation policy references do not identify the actual evaluated retention, provenance and resource rules",
        ));
    }
    Ok(())
}
