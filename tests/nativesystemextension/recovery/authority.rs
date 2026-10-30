use crate::support::*;

pub(super) struct Operator {
    issuer: basalt::ucan::Ed25519InMemorySigner,
    holder: basalt::ucan::Ed25519InMemorySigner,
    audience: basalt::ucan::AudienceDid,
    nonce: u64,
    not_before: u64,
    expires: u64,
}

impl Operator {
    pub(super) fn provision(root: &molten::node_state::DirectoryRoot) -> TestResult<Self> {
        let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([71; 32]);
        let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([73; 32]);
        let audience =
            basalt::ucan::AudienceDid::from(basalt::ucan::TokenSigner::issuer(&holder).or_fail("operator holder DID")?);
        let now = molten::world_witness::WallClock::unix_seconds(&mut molten::world_witness::live_clock()?)?;
        let not_before = now.saturating_sub(1);
        let expires = now + 3600;
        let trust = molten::node_service_host::authority::wire::Trust {
            issuer: basalt::ucan::TokenSigner::issuer(&issuer).or_fail("operator issuer DID")?.as_str().to_string(),
            issuer_key: basalt::ucan::derive_ed25519_public_key(&[71; 32]).or_fail("operator issuer public key")?,
            holder: audience.as_str().to_string(),
            holder_key: basalt::ucan::derive_ed25519_public_key(&[73; 32]).or_fail("operator holder public key")?,
            policy: r#"{"backends":{},"contracts":{"node-service-host":{"abilities":["load","install","status","supervise","submit","readmit","readmit-migrated","deliver","upgrade","reconcile"],"description":"isolated real-node conformance operator","id":"node-service-host","resource_prefixes":["molten://node-service-host/"]}},"schema_version":"ucan-nickel-contracts.policy.v1"}"#.to_string(),
            not_before, expires, revoked: Vec::new(),
        };
        root.write(
            &molten::node_state::RelativePath::parse("service-host-trust")?,
            &molten::node_service_host::authority::wire::encode_trust(&trust)?,
        )?;
        Ok(Self {
            issuer,
            holder,
            audience,
            nonce: 0,
            not_before,
            expires,
        })
    }

    pub(super) fn authorize(
        &mut self,
        root: &molten::node_state::DirectoryRoot,
        bundle_ref: &str,
        request: &molten::node_service_hosting::OperationRequest,
    ) -> TestResult<()> {
        self.nonce += 1;
        let capabilities = basalt::ucan::CapabilitySet::new(vec![
            basalt::ucan::CapabilityDocument::new(
                molten::node_service_host::authority::resource(bundle_ref),
                request.operation.clone(),
            )
            .or_fail("exact operation capability")?,
        ])
        .or_fail("operator capability set")?;
        let token = basalt::ucan::issue_token_with_signer(
            &self.issuer,
            &self.audience,
            &capabilities,
            &basalt::ucan::ProofReferences::empty(),
            basalt::ucan::TokenTimeBounds::new(self.not_before, self.expires).or_fail("token validity")?,
        )
        .or_fail("actual issuer-signed capability")?;
        let mut nonce = [0_u8; 32];
        nonce[..8].copy_from_slice(&self.nonce.to_le_bytes());
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
        )?;
        Ok(())
    }
}
