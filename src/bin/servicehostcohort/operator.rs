pub(super) struct Authority {
    issuer: basalt::ucan::Ed25519InMemorySigner,
    holder: basalt::ucan::Ed25519InMemorySigner,
    issuer_key: [u8; 32],
    holder_key: [u8; 32],
    audience: basalt::ucan::AudienceDid,
    pub policy: String,
    not_before: u64,
    expires: u64,
    revoked: Vec<[u8; 32]>,
}

impl Authority {
    pub fn open(
        issuer_path: &std::path::Path,
        holder_path: &std::path::Path,
        policy_path: &std::path::Path,
        root: &molten::node_state::DirectoryRoot,
    ) -> molten::error::Result<Self> {
        let mut issuer_seed = seed(issuer_path)?;
        let mut holder_seed = seed(holder_path)?;
        let issuer_key = basalt::ucan::derive_ed25519_public_key(&issuer_seed)
            .map_err(|_| super::invalid("issuer public key derivation failed"))?;
        let holder_key = basalt::ucan::derive_ed25519_public_key(&holder_seed)
            .map_err(|_| super::invalid("holder public key derivation failed"))?;
        let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes(issuer_seed);
        let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes(holder_seed);
        issuer_seed.fill(0);
        holder_seed.fill(0);
        let audience = basalt::ucan::AudienceDid::from(
            basalt::ucan::TokenSigner::issuer(&holder).map_err(|_| super::invalid("holder DID derivation failed"))?,
        );
        let policy = String::from_utf8(super::source::read_input(policy_path, 262_144)?)
            .map_err(|_| super::invalid("operator policy must be UTF-8"))?;
        basalt::parse_policy_json(&policy)
            .map_err(|_| super::invalid("operator policy is not an admitted Basalt policy"))?;
        let now = unix_now()?;
        let path = molten::node_state::RelativePath::parse("operator-policy.json")?;
        if root.entry_kind(&path)?.is_some() {
            if root.read(&path, 262_144)? != policy.as_bytes() {
                return Err(super::invalid("explicit operator policy differs from existing cohort"));
            }
        } else {
            root.write_new(&path, policy.as_bytes())?;
        }
        Ok(Self {
            issuer,
            holder,
            issuer_key,
            holder_key,
            audience,
            policy,
            not_before: now.saturating_sub(1),
            expires: now + 3600,
            revoked: Vec::new(),
        })
    }

    pub fn bind_existing_trust(&mut self, root: &molten::node_state::DirectoryRoot) -> molten::error::Result<()> {
        let trust = molten::node_service_host::authority::wire::decode_trust(
            &root.read(&molten::node_state::RelativePath::parse("service-host-trust")?, 262_144)?,
        )?;
        let now = unix_now()?;
        if trust.issuer != self.issuer_did()?
            || trust.holder != self.audience.as_str()
            || trust.issuer_key != self.issuer_key
            || trust.holder_key != self.holder_key
            || trust.policy != self.policy
            || now < trust.not_before
            || now >= trust.expires
        {
            return Err(super::invalid(
                "explicit signing keys/policy do not match current independently provisioned cohort trust",
            ));
        }
        self.not_before = trust.not_before;
        self.expires = trust.expires;
        self.revoked = trust.revoked;
        Ok(())
    }

    pub fn issuer_did(&self) -> molten::error::Result<String> {
        Ok(basalt::ucan::TokenSigner::issuer(&self.issuer)
            .map_err(|_| super::invalid("issuer DID unavailable"))?
            .as_str()
            .to_string())
    }

    pub fn grant(
        &self,
        root: &molten::node_state::DirectoryRoot,
        resource: &str,
        ability: &str,
        subject_ref: &str,
    ) -> molten::error::Result<String> {
        let token = self.token(resource, ability)?;
        super::source::record(root, "experimental-operator-ucan-grant-v1", vec![
            molten::preserves_rail::string(self.issuer_did()?),
            molten::preserves_rail::string(self.audience.as_str()),
            molten::preserves_rail::string(resource),
            molten::preserves_rail::string(ability),
            molten::preserves_rail::string(subject_ref),
            molten::preserves_rail::string(token.as_str()),
            molten::preserves_rail::string("experimental-only-not-production-admission"),
        ])
    }

    fn token(&self, resource: &str, ability: &str) -> molten::error::Result<basalt::ucan::CompactToken> {
        let policy = basalt::parse_policy_json(&self.policy).map_err(|_| super::invalid("operator policy denied"))?;
        let request = basalt::EnforcementRequest::new("node-service-host", resource, ability)
            .with_capabilities(vec![basalt::CapabilityGrant::new(resource, ability)]);
        if !basalt::enforce(&policy, &request)
            .map_err(|_| super::invalid("operator policy evaluation failed"))?
            .is_allowed()
        {
            return Err(super::invalid("independent operator policy does not authorize this experimental capability"));
        }
        let capability = basalt::ucan::CapabilityDocument::new(resource.to_owned(), ability.to_owned())
            .map_err(|_| super::invalid("invalid exact capability"))?;
        let capabilities =
            basalt::ucan::CapabilitySet::new(vec![capability]).map_err(|_| super::invalid("capability set denied"))?;
        let token = basalt::ucan::issue_token_with_signer(
            &self.issuer,
            &self.audience,
            &capabilities,
            &basalt::ucan::ProofReferences::empty(),
            basalt::ucan::TokenTimeBounds::new(self.not_before, self.expires)
                .map_err(|_| super::invalid("operator token validity denied"))?,
        )
        .map_err(|_| super::invalid("actual operator token signing failed"))?;
        let proof = basalt::ucan::proof_reference(&token);
        if self.revoked.iter().any(|reference| proof.as_bytes() == reference) {
            return Err(super::invalid("current operator trust has revoked this capability proof"));
        }
        Ok(token)
    }

    pub fn publish(
        &mut self,
        root: &molten::node_state::DirectoryRoot,
        bundle_ref: &str,
        validation: &molten::node_daemon::SelectedNodeOutputValidation,
    ) -> molten::error::Result<()> {
        let trust = molten::node_service_host::authority::wire::Trust {
            issuer: self.issuer_did()?,
            issuer_key: self.issuer_key,
            holder: self.audience.as_str().to_string(),
            holder_key: self.holder_key,
            policy: self.policy.clone(),
            not_before: self.not_before,
            expires: self.expires,
            revoked: Vec::new(),
        };
        root.write_new(
            &molten::node_state::RelativePath::parse("service-host-trust")?,
            &molten::node_service_host::authority::wire::encode_trust(&trust)?,
        )?;
        let request = molten::node_service_host::command::load_request(
            bundle_ref,
            &validation.expected_startup_validation_ref,
            1,
        )?;
        self.authorize(root, bundle_ref, &request, [1; 32])?;
        let authored = molten::node_service_hosting::OperationRequest {
            operation: "author-cohort".to_string(),
            subject: bundle_ref.to_string(),
            logical_tick: 0,
            state_ref: validation.selected_output_validation_ref.clone(),
            input_ref: bundle_ref.to_string(),
        };
        self.authorize(root, bundle_ref, &authored, [2; 32])
    }

    pub fn authorize(
        &self,
        root: &molten::node_state::DirectoryRoot,
        bundle_ref: &str,
        request: &molten::node_service_hosting::OperationRequest,
        nonce: [u8; 32],
    ) -> molten::error::Result<()> {
        let resource = molten::node_service_host::authority::resource(bundle_ref);
        let token = self.token(&resource, &request.operation)?;
        let credential = molten::node_service_host::authority::sign_credential(
            molten::node_service_host::authority::SigningInput {
                bundle_root: bundle_ref,
                request,
                token: token.as_str(),
                audience: &self.audience,
                nonce,
            },
            &self.holder,
        )?;
        let reference = molten::node_service_host::authority::request_ref(bundle_ref, request)?;
        root.write(
            &molten::node_service_host::authority::credential_path(&reference)?,
            &molten::node_service_host::authority::wire::encode_credential(&credential)?,
        )
    }
}

fn seed(path: &std::path::Path) -> molten::error::Result<[u8; 32]> {
    let mut bytes = super::source::read_input(path, 32)?;
    let result = bytes
        .as_slice()
        .try_into()
        .map_err(|_| super::invalid("operator key must contain exactly 32 raw seed bytes"));
    bytes.fill(0);
    result
}

/// Operator verification time, observed only through the admitted fabric-time live wall clock.
fn unix_now() -> molten::error::Result<u64> {
    molten::world_witness::WallClock::unix_seconds(&mut molten::world_witness::live_clock()?)
        .map_err(|_| super::invalid("operator verification clock unavailable"))
}
