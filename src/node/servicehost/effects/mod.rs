//! Explicit collaboration-only native effect runtime composition.
//!
//! Source admission and callback success grant no effect authority. Unconfigured
//! hosts retain their approved-but-unrouted effects as unknown work.

pub mod authority;
pub mod config;
mod router;

// r[impl molten.system_extension.native_effect_installation]
// r[impl molten.system_extension.native_effect_lifetime]
pub(super) fn install(
    root: &crate::node_state::DirectoryRoot,
    bundle_ref: &str,
    gate: &crate::node_runtime::ValidatedNodeSourceGate,
    logical_tick: u64,
    runtime: &mut super::runtime::Native,
) -> crate::error::Result<()> {
    let selector = crate::node_state::RelativePath::parse(config::SELECTOR)?;
    if !root.try_exists(&selector)? {
        return Ok(());
    }
    let bytes = root.read(&selector, config::MAX_CONFIG_BYTES as u64)?;
    let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
    let configuration = config::decode(&bytes, &reference)?;
    let mut authority = authority::Gate::open(root, bundle_ref, &reference)?;
    authority.admit_install(gate.validation_ref(), logical_tick)?;
    for binding in configuration.bindings {
        let effect_authority = authority::Gate::open(root, bundle_ref, &reference)?;
        let service = runtime.service(&binding.service_id)?;
        validate_supervisor(service, &binding)?;
        let recovery_group =
            service.host().manifest().manifest().recovery_group.as_ref().map(|group| group.group_id.clone());
        let bytes = root.read(&crate::node_state::RelativePath::parse(&binding.runtime_artifact)?, 16_777_216)?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != binding.runtime_ref {
            return Err(super::invalid("scope runtime artifact differs from independently admitted configuration"));
        }
        let plan = crate::collaboration_scope::native_runtime::admit(&bytes, service.host())?;
        if plan.installation_ref() != binding.installation_ref
            || plan.group_ref() != binding.group_ref
            || plan.policy_ref() != binding.policy_ref
            || plan.resource_ref() != binding.resource_ref
        {
            return Err(super::invalid(
                "actual scope installation/group/policy/resource differs from effect configuration",
            ));
        }
        // The genuine install-effects grant and every exact active binding check
        // precede construction of scope storage, replica, transport and worker.
        let scope = crate::collaboration_scope::native_runtime::Worker::open(root, plan)?;
        let member = binding.service_id.clone();
        let router = router::Scope::new(binding, effect_authority, scope, recovery_group);
        runtime.register_effect_router(&member, Box::new(router))?;
    }
    Ok(())
}

fn validate_supervisor(
    service: &crate::system_extension::Supervisor<super::runtime::Port, crate::system_extension::Durable>,
    binding: &config::Binding,
) -> crate::error::Result<()> {
    let instance = service.instance().map_err(|_| super::invalid("native effect installation instance unavailable"))?;
    let manifest = service.host().manifest();
    let key = crate::fabric::FabricPortKey {
        port_id: binding.port_id.clone(),
        version: binding.port_version.clone(),
    };
    let actual = manifest
        .binding_for(&key)
        .ok_or_else(|| super::invalid("scope effect port was not admitted by actual native host"))?;
    if instance.service_id != binding.service_id
        || instance.manifest_ref != binding.manifest_ref
        || instance.lifecycle.generation != binding.generation
        || instance.lifecycle.phase != crate::system_extension::LifecyclePhase::Running
        || actual.binding_ref != binding.port_binding_ref
        || !manifest.manifest().policy_refs.contains(&binding.policy_ref)
        || service.executable().executable.resource_ref != binding.resource_ref
    {
        return Err(super::invalid(
            "effect configuration differs from actual active service/port/generation/policy/resource",
        ));
    }
    Ok(())
}
