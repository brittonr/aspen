pub struct Context {
    pub authorization: super::access::Authorization,
    pub source_gate: crate::node_runtime::ValidatedNodeSourceGate,
    pub health_path: String,
    pub instance_ref: String,
    pub native_run_ref: String,
    pub native_startup_ref: String,
    pub source_trust_ref: String,
    pub peer_probes: std::collections::BTreeMap<String, super::leader::PeerProbe>,
    pub transport_registration: crate::fabric_transport::IrohAdapter,
}

impl Context {
    pub(super) fn require_current(&self, operation: &str) -> crate::error::Result<()> {
        self.authorization.require(operation)?;
        if !self.transport_registration.state().protocols.values().any(|protocol| {
            protocol.phase == crate::fabric_transport::ProtocolRegistrationPhase::Active
                && protocol.descriptor.generation == self.authorization.grant.generation()
        }) {
            return Err(crate::error::Failure::invalid_harness("coordination transport registration is not current"));
        }
        let actual = super::super::readiness::current::read_with_admitted_source(
            &self.authorization.root,
            &self.health_path,
            &self.source_gate,
            &self.source_trust_ref,
        )?;
        if actual.source_binding_ref != self.source_gate.binding_ref()
            || actual.run_ref != self.native_run_ref
            || actual.startup_ref != self.native_startup_ref
        {
            return Err(crate::error::Failure::invalid_harness(
                "native node generation or authenticated source changed",
            ));
        }
        Ok(())
    }
}
