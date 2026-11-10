const POLICY: &str = r#"{"backends":{},"contracts":{"node-service-host":{"abilities":["load","install","status","submit","deliver","supervise","readmit","upgrade","reconcile","install-effects","route-effect","scope-observe"],"description":"Native collaboration host operator","id":"node-service-host","resource_prefixes":["molten://node-service-host/"]}},"schema_version":"ucan-nickel-contracts.policy.v1"}"#;

impl super::Issuer {
    /// `root` is the independent operator capability, not the executable/source directory.
    pub fn provision_host(&self, root: &molten::node_state::DirectoryRoot, window: super::Window) -> super::Result<()> {
        let trust = molten::node_service_host::authority::wire::Trust {
            issuer: self.issuer_did()?,
            issuer_key: self.issuer_key,
            holder: self.holder_did().to_owned(),
            holder_key: self.holder_key,
            policy: POLICY.to_owned(),
            not_before: window.not_before,
            expires: window.expires,
            revoked: Vec::new(),
        };
        root.write_new(
            &molten::node_state::RelativePath::parse("service-host-trust")?,
            &molten::node_service_host::authority::wire::encode_trust(&trust)?,
        )?;
        Ok(())
    }

    /// Signs all host verbs through the same exact public OperationRequest binding.
    /// `nonce` must be unique within this operator root, including across process restarts.
    pub fn host_credential(
        &self,
        bundle: &str,
        request: &molten::node_service_hosting::OperationRequest,
        window: super::Window,
        nonce: [u8; 32],
    ) -> super::Result<molten::node_service_host::authority::wire::Credential> {
        let token = self.token(&molten::node_service_host::authority::resource(bundle), &request.operation, window)?;
        Ok(molten::node_service_host::authority::sign_credential(
            molten::node_service_host::authority::SigningInput {
                bundle_root: bundle,
                request,
                token: token.as_str(),
                audience: &self.audience,
                nonce,
            },
            &self.holder,
        )?)
    }

    pub fn write_host_credential(
        &self,
        root: &molten::node_state::DirectoryRoot,
        bundle: &str,
        request: &molten::node_service_hosting::OperationRequest,
        credential: &molten::node_service_host::authority::wire::Credential,
    ) -> super::Result<String> {
        let reference = molten::node_service_host::authority::request_ref(bundle, request)?;
        // Re-signing the same exact request replaces its credential; the nonce is the holder's.
        root.write(
            &molten::node_service_host::authority::credential_path(&reference)?,
            &molten::node_service_host::authority::wire::encode_credential(credential)?,
        )?;
        Ok(reference)
    }
}

/// Call this exact producer before signing `install-effects`; source provenance is not a grant.
pub fn install_effects_request(
    bundle: &str,
    configuration: &molten::node_service_host::effects::config::Configuration,
    source_validation: &str,
    tick: u64,
) -> super::Result<molten::node_service_hosting::OperationRequest> {
    Ok(molten::node_service_host::effects::authority::install_request(
        bundle,
        &molten::node_service_host::effects::config::reference(configuration)?,
        source_validation,
        tick,
    )?)
}

/// Route credentials include the actual canonical binding and complete concrete effect.
pub fn route_effect_request(
    bundle: &str,
    configuration: &molten::node_service_host::effects::config::Configuration,
    input: molten::node_service_host::effects::authority::RouteInput<'_>,
    tick: u64,
) -> super::Result<molten::node_service_hosting::OperationRequest> {
    Ok(molten::node_service_host::effects::authority::route_request(
        bundle,
        &molten::node_service_host::effects::config::reference(configuration)?,
        input,
        tick,
    )?)
}
