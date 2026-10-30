//! Telemetry has a separate UCAN ability, policy, sink capability and replay operation.
use basalt::ucan;

#[derive(Clone)]
pub struct Scope {
    pub component: String,
    pub bundle: String,
    pub wit: String,
    pub receipt: String,
    pub sink: String,
    pub policy: String,
    pub generation: u64,
    pub deadline_tick: u64,
}
impl Scope {
    pub fn value(&self) -> preserves::IOValue {
        crate::codec::record("component-rpc-otel-scope-v1", vec![
            crate::codec::string(super::REVISION),
            crate::codec::string(super::INTERFACE),
            crate::codec::string(&self.component),
            crate::codec::string(&self.bundle),
            crate::codec::string(&self.wit),
            crate::codec::string(&self.receipt),
            crate::codec::string(&self.sink),
            crate::codec::string(&self.policy),
            crate::codec::string(self.generation.to_string()),
            crate::codec::string(self.deadline_tick.to_string()),
            crate::codec::string(super::MAX_SPANS.to_string()),
            crate::codec::string("diagnostic-only-no-raw-payloads"),
        ])
    }
}
pub struct Lease {
    scope: Scope,
    authority: String,
}
impl Lease {
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn authority(&self) -> &str {
        &self.authority
    }
    pub fn check(&self, tick: u64, generation: u64) -> super::Result<()> {
        if tick >= self.scope.deadline_tick || generation != self.scope.generation {
            return Err("current telemetry authority deadline/generation denied".into());
        }
        Ok(())
    }
}
pub fn resource(scope: &Scope) -> String {
    format!("urn:molten:component-rpc:diagnostics:{}:{}:{}", scope.component, scope.receipt, scope.sink)
}
pub fn binding(scope: &Scope) -> super::Result<ucan::OperationBinding> {
    let profile = crate::codec::record("component-rpc-otel-profile-v1", vec![
        crate::codec::string(super::REVISION),
        crate::codec::string(super::INTERFACE),
        crate::codec::string(super::ABILITY),
    ]);
    let profile = crate::codec::canonical_bytes(&profile).map_err(super::super::failure)?;
    let value = crate::codec::canonical_bytes(&scope.value()).map_err(super::super::failure)?;
    Ok(ucan::OperationBinding::new(
        ucan::OperationProfileId::from_bytes(*blake3::hash(&profile).as_bytes()),
        ucan::RequestBindingDigest::from_bytes(*blake3::hash(&value).as_bytes()),
    ))
}

pub fn admit(scope: &Scope, input: &super::super::authority::Input<'_>) -> super::Result<Lease> {
    for reference in [
        &scope.component,
        &scope.bundle,
        &scope.wit,
        &scope.receipt,
        &scope.sink,
        &scope.policy,
    ] {
        crate::codec::validate_content_ref(reference).map_err(super::super::failure)?;
    }
    if scope.generation == 0
        || input.generation != scope.generation
        || input.tick >= scope.deadline_tick
        || super::super::authority::policy_ref(input.policy_json)? != scope.policy
    {
        return Err("telemetry current policy/resource/generation context differs".into());
    }
    let authority = super::super::authority::authorize(
        super::super::authority::Operation {
            contract: super::CONTRACT,
            ability: super::ABILITY,
            resource: resource(scope),
            binding: binding(scope)?,
            value: scope.value(),
        },
        input,
    )?;
    Ok(Lease {
        scope: scope.clone(),
        authority,
    })
}
