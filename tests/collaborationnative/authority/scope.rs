pub struct TrustInput<'a> {
    pub policies: &'a [&'a str],
    /// Full compact tokens, not proof-reference strings: the runtime hashes these itself.
    pub revoked_tokens: &'a [&'a str],
    pub logical_position: u64,
    pub unix_seconds: u64,
}

pub struct Credential {
    pub token: String,
    pub nonce: [u8; 32],
    pub signature: [u8; 64],
}

impl super::Issuer {
    pub fn scope_trust(&self, input: &TrustInput<'_>) -> super::Result<preserves::IOValue> {
        if input.policies.is_empty()
            || input.policies.len() > 6
            || input.revoked_tokens.len() > molten::collaboration_scope::MAX_OPERATIONS
        {
            return Err("scope trust collection bound".into());
        }
        for policy in input.policies {
            molten::collaboration_scope::policy_ref(policy)?;
        }
        for token in input.revoked_tokens {
            basalt::ucan::CompactToken::new(token).map_err(|error| format!("{error:?}"))?;
        }
        Ok(molten::preserves_rail::record("collaboration-operator-trust-v1", vec![
            molten::preserves_rail::string(self.issuer_did()?),
            molten::preserves_rail::string(super::hex(&self.issuer_key)),
            molten::preserves_rail::string(self.holder_did()),
            molten::preserves_rail::string(super::hex(&self.holder_key)),
            molten::preserves_rail::sequence(input.policies.iter().map(molten::preserves_rail::string).collect()),
            molten::preserves_rail::sequence(input.revoked_tokens.iter().map(molten::preserves_rail::string).collect()),
            molten::preserves_rail::u64_value(input.logical_position),
            molten::preserves_rail::u64_value(input.unix_seconds),
        ]))
    }

    /// Publishes the independently current revocation root; never rewrites replay reservations.
    /// Call again with revoked tokens or a changed position to exercise current-root changes.
    pub fn provision_scope(
        &self,
        root: &molten::node_state::DirectoryRoot,
        input: &TrustInput<'_>,
    ) -> super::Result<molten_core::nominal::RevocationRef> {
        let value = self.scope_trust(input)?;
        root.create_dir_all(&molten::node_state::RelativePath::parse("operator-scope-authority/credentials")?)?;
        root.write(
            &molten::node_state::RelativePath::parse("operator-scope-authority/current.preserves")?,
            &molten::preserves_rail::canonical_bytes(&value)?,
        )?;
        let reference = molten::preserves_rail::canonical_hash(&value)?;
        molten_core::nominal::RevocationRef::new(reference).map_err(|error| format!("{error:?}").into())
    }

    /// Signs the request whose currentness the caller expects a real quorum read to establish.
    /// This creates no decision, current observation, scope store entry, or committed command.
    pub fn scope_credential(
        &self,
        request: &molten::collaboration_scope::MutationRequest,
        window: super::Window,
        nonce: [u8; 32],
    ) -> super::Result<Credential> {
        if molten::collaboration_scope::mutation_request_ref(request)? != request.reference
            || request.actor != self.subject()?
        {
            return Err("scope request does not bind signer or canonical intent".into());
        }
        let resource = format!(
            "molten://collaboration/{}",
            molten::collaboration_scope::mutation_resource(&request.mutation)?.as_str()
        );
        let ability = request.mutation.ability().as_str();
        let token = self.token(&resource, ability, window)?;
        let invocation = basalt::ucan::HolderInvocationSigningRequest::new(
            token.as_str(),
            &self.audience,
            &resource,
            ability,
            molten::collaboration_scope::operation_binding(&request.reference)?,
            basalt::ucan::HolderInvocationNonce::from_bytes(nonce),
        );
        let limits = basalt::ucan::VerificationLimits::new(16, 64, 65_536).with_clock_skew_tolerance_seconds(0);
        let holder_limits =
            basalt::ucan::HolderInvocationLimits::try_new(512, 4_096, 256).map_err(|error| format!("{error:?}"))?;
        let signature = basalt::ucan::sign_holder_invocation(&invocation, &self.holder, limits, holder_limits)
            .map_err(|error| format!("{error:?}"))?;
        Ok(Credential {
            token: token.as_str().to_owned(),
            nonce,
            signature: signature.as_bytes().try_into().map_err(|_| "holder signature width")?,
        })
    }
}

/// The lookup is keyed by the concrete staged effect, not by a historic decision or token.
pub fn write_credential(
    root: &molten::node_state::DirectoryRoot,
    effect_ref: &str,
    known_read: &molten::collaboration_scope::RequestRef,
    credential: &Credential,
) -> super::Result<()> {
    molten::preserves_rail::validate_content_ref(effect_ref)?;
    let digest = effect_ref.strip_prefix("blake3:").ok_or("effect digest prefix")?;
    let value = molten::preserves_rail::record("collaboration-operator-credential-v1", vec![
        molten::preserves_rail::string(effect_ref),
        molten::preserves_rail::string(known_read.as_str()),
        molten::preserves_rail::string(&credential.token),
        molten::preserves_rail::string(super::hex(&credential.nonce)),
        molten::preserves_rail::string(super::hex(&credential.signature)),
    ]);
    // Publication is content-addressed and idempotent: republishing the same exact credential
    // must not fail, and a different credential for the same effect lands under its own digest.
    root.write(
        &molten::node_state::RelativePath::parse(&format!("operator-scope-authority/credentials/{digest}.preserves"))?,
        &molten::preserves_rail::canonical_bytes(&value)?,
    )?;
    Ok(())
}
