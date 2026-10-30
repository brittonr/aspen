/// Separately verified domain authority retained with its rooted currentness source.
pub struct Authorization {
    pub(super) root: crate::node_state::DirectoryRoot,
    pub(super) grant: super::super::authority::VerifiedGrant,
}

impl Authorization {
    pub fn load(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Self> {
        let grant = super::super::authority::load(root)?;
        Ok(Self {
            root: root.clone(),
            grant,
        })
    }
    pub fn grant(&self) -> &super::super::authority::VerifiedGrant {
        &self.grant
    }
    pub fn require(&self, operation: &str) -> crate::error::Result<()> {
        self.grant.verify_current(&self.root)?;
        if !self.grant.allows(operation) {
            return Err(crate::error::Failure::invalid_harness(format!("coordination grant denies {operation}")));
        }
        Ok(())
    }
    pub(super) fn request(&self, request: &super::super::protocol::Request) -> crate::error::Result<()> {
        let operation = match request.phase {
            super::super::protocol::Phase::Setup => "setup",
            super::super::protocol::Phase::Invoke => request
                .operation
                .as_ref()
                .and_then(|value| value.get("function"))
                .and_then(|value| value.as_str())
                .ok_or_else(|| crate::error::Failure::invalid_harness("missing registry operation"))?,
            super::super::protocol::Phase::Recover => "recover",
            super::super::protocol::Phase::FinalRead => "final-read",
            super::super::protocol::Phase::Teardown => "teardown",
            super::super::protocol::Phase::Probe => "probe",
        };
        self.require(operation)?;
        if request.phase == super::super::protocol::Phase::Probe {
            return Ok(());
        }
        let invocation = request
            .authorization
            .as_ref()
            .ok_or_else(|| crate::error::Failure::invalid_harness("authenticated client invocation required"))?;
        let mut unsigned = request.clone();
        unsigned.authorization = None;
        super::super::authority::verify_request(&self.grant, &unsigned, invocation)?;
        let identity = crate::preserves_rail::record("live-client-replay-v1", vec![
            crate::preserves_rail::string(&invocation.client_scope_ref),
            preserves::IOValue::bytes(invocation.nonce),
        ]);
        let reference = crate::preserves_rail::canonical_hash(&identity)?;
        let path =
            crate::node_state::RelativePath::parse(&format!("coordination-client-{}.preserves", &reference[7..]))?;
        self.root.write_new(&path, &crate::preserves_rail::canonical_bytes(&identity)?)
    }

    pub(super) fn validate_start(&self, input: &super::launch::Startup) -> crate::error::Result<()> {
        self.require("serve")?;
        let grant = self.grant.grant();
        if grant.node_id != input.node_id
            || grant.group_ref != input.group.binding_ref
            || grant.membership_ref != input.membership.membership_ref
            || grant.generation != input.group.service_generation
            || grant.endpoint != input.host.endpoint.to_string()
            || grant.source_binding_ref != input.source_gate.binding_ref()
            || grant.transport_session_ref != input.ingress.session_ref
        {
            return Err(crate::error::Failure::invalid_harness(
                "coordination startup differs from signed domain grant",
            ));
        }
        Ok(())
    }
}
