/// Untrusted producer artifacts. None of these refs assert authorization or commit.
pub struct Prepared {
    pub request: molten::collaboration_scope::MutationRequest,
    pub staged_bytes: Vec<u8>,
    pub effect_ref: String,
}

impl super::Issuer {
    /// `currentness` must come from the expected real known-read round, including its
    /// exact receipt ref; the native route independently acquires and compares it.
    pub fn mutation_request(
        &self,
        operation: molten_core::nominal::OperationRef,
        before: molten::collaboration_scope::StateRef,
        currentness: molten::collaboration_scope::Currentness,
        mutation: molten::collaboration_scope::Mutation,
    ) -> super::Result<molten::collaboration_scope::MutationRequest> {
        let value = molten::preserves_rail::record("scope-mutation-request-v1", vec![
            molten::preserves_rail::string(operation.as_str()),
            molten::preserves_rail::string(before.as_str()),
            molten::preserves_rail::string(self.subject()?.as_str()),
            molten::collaboration_scope::currentness_value(&currentness),
            molten::collaboration_scope::mutation_value(&mutation),
        ]);
        Ok(molten::collaboration_scope::parse_mutation_request(&value)?)
    }
}

/// Empty historical decisions are deliberate: only fresh runtime stage_mutation can
/// create authority evidence. Publish command_bytes and staged_bytes as actual values.
pub fn prepare(request: molten::collaboration_scope::MutationRequest) -> super::Result<Prepared> {
    if molten::collaboration_scope::mutation_request_ref(&request)? != request.reference {
        return Err("noncanonical intended request".into());
    }
    let command_value = molten::collaboration_scope::intent_value(&request);
    let command_bytes = molten::preserves_rail::canonical_bytes(&command_value)?;
    let command = molten::fabric_consistency::raft::Command {
        request_ref: request.reference.as_str().to_owned(),
        command_ref: molten::preserves_rail::content_ref_from_bytes(&command_bytes),
        command_schema_ref: molten::collaboration_scope::command_schema_ref()?,
    };
    let staged_bytes =
        molten::preserves_rail::canonical_bytes(&molten::collaboration_scope::staged_command_value(&command))?;
    let effect_ref = molten::preserves_rail::content_ref_from_bytes(&staged_bytes);
    Ok(Prepared {
        request,
        staged_bytes,
        effect_ref,
    })
}
