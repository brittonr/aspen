#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; mutating methods are explicit consumer-owned Core registry/context transitions, never mutations of borrowed Prepared or guest subject values; proof checking remains the declared trusted external boundary"
)]
impl crate::companion::Core {
    /// A fresh core with no contracts, evidence, or observations.
    pub fn new(limits: crate::Limits) -> Self {
        Self {
            limits,
            registry: crate::companion::registry::Registry::new(limits),
            policy: 0,
            revision: 0,
            semantic_revision: noble_kernel::untrusted::SEMANTIC_REVISION,
            host_contract: 0,
            environment_fact: 0,
        }
    }

    /// Select the consumer policy. Adopting a different policy bumps the
    /// context revision, so evidence admitted under the old policy reports
    /// `StaleContext` until it is revalidated (VC-VALUE-03).
    pub fn set_policy(&mut self, policy: u32) {
        if policy != self.policy {
            self.policy = policy;
            self.revision = self.revision.saturating_add(1);
        }
    }

    /// Invalidate applicability when the consumer changes its semantic model.
    /// This does not extend the fragment supported by the proof checker.
    pub fn set_semantic_revision(&mut self, revision: u32) {
        if revision != self.semantic_revision {
            self.semantic_revision = revision;
            self.revision = self.revision.saturating_add(1);
        }
    }

    /// Select the consumer's immutable host-contract identity.
    pub fn set_host_contract(&mut self, identity: u64) {
        if identity != self.host_contract {
            self.host_contract = identity;
            self.revision = self.revision.saturating_add(1);
        }
    }

    /// Select the consumer's immutable current-environment fact identity.
    pub fn set_environment_fact(&mut self, identity: u64) {
        if identity != self.environment_fact {
            self.environment_fact = identity;
            self.revision = self.revision.saturating_add(1);
        }
    }
}
