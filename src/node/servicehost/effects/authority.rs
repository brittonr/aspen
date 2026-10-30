//! Separate current operator admission for concrete collaboration effects.
//! Producer requests are public; neither source receipts nor config possession grant authority.

pub struct RouteInput<'a> {
    pub binding: &'a super::config::Binding,
    pub actual_binding: &'a crate::fabric::CanonicalPortBinding,
    pub effect: &'a crate::system_extension::TypedEffectRequest,
}

pub struct Gate {
    current: crate::node_service_host::authority::Current<crate::fabric_time::LiveClockAdapter>,
    bundle_ref: String,
    config_ref: String,
}

// r[impl molten.system_extension.native_effect_authority]
impl Gate {
    /// Opens only independently provisioned operator trust, without creating effect resources.
    pub fn open(
        root: &crate::node_state::DirectoryRoot,
        bundle_ref: &str,
        config_ref: &str,
    ) -> crate::error::Result<Self> {
        crate::preserves_rail::ContentRef::parse(config_ref)?;
        Ok(Self {
            current: crate::node_service_host::authority::Current::open(
                root,
                bundle_ref,
                crate::world_witness::live_clock()?,
            )?,
            bundle_ref: bundle_ref.to_owned(),
            config_ref: config_ref.to_owned(),
        })
    }

    pub fn admit_install(&mut self, source_ref: &str, tick: u64) -> crate::error::Result<String> {
        let request = install_request(&self.bundle_ref, &self.config_ref, source_ref, tick)?;
        self.admit(&request)
    }

    /// Rechecks current signed authority on every route, immediately before the caller queues it.
    /// This receipt is admission only: it is neither queue acceptance nor a committed effect.
    pub fn admit_route(&mut self, input: RouteInput<'_>, tick: u64) -> crate::error::Result<String> {
        let request = route_request(&self.bundle_ref, &self.config_ref, input, tick)?;
        self.admit(&request)
    }

    fn admit(&mut self, request: &crate::node_service_hosting::OperationRequest) -> crate::error::Result<String> {
        let authority_ref = crate::node_service_hosting::AuthorityPort::admit(&mut self.current, request)?;
        let request_ref = crate::node_service_host::authority::request_ref(&self.bundle_ref, request)?;
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "node-service-host-effect-admitted-v1",
            vec![
                crate::preserves_rail::string(request_ref),
                crate::preserves_rail::string(authority_ref),
                crate::preserves_rail::string("admission-not-queued-not-committed"),
            ],
        ))
    }
}

/// Exact request for independent signing through `authority::sign_credential`.
/// The source validation reference is provenance, not an effect capability.
pub fn install_request(
    bundle_ref: &str,
    config_ref: &str,
    source_ref: &str,
    tick: u64,
) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
    for reference in [bundle_ref, config_ref, source_ref] {
        crate::preserves_rail::ContentRef::parse(reference)?;
    }
    let input_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "node-service-host-install-effects-v1",
        vec![
            crate::preserves_rail::string(bundle_ref),
            crate::preserves_rail::string(config_ref),
            crate::preserves_rail::string(source_ref),
            crate::preserves_rail::u64_value(tick),
        ],
    ))?;
    Ok(crate::node_service_hosting::OperationRequest {
        operation: "install-effects".to_owned(),
        subject: config_ref.to_owned(),
        logical_tick: tick,
        state_ref: config_ref.to_owned(),
        input_ref,
    })
}

/// Pure exact route projection, usable before delivery by an independent grant producer.
/// The binding hash includes service/manifest, scope, policy, resource and installation.
pub fn route_request(
    bundle_ref: &str,
    config_ref: &str,
    input: RouteInput<'_>,
    tick: u64,
) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
    for reference in [bundle_ref, config_ref, &input.effect.request_ref] {
        crate::preserves_rail::ContentRef::parse(reference)?;
    }
    validate_route(&input)?;
    let binding_ref = super::config::binding_ref(input.binding)?;
    let effect = input.effect;
    let input_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "node-service-host-route-effect-v1",
        vec![
            crate::preserves_rail::string(bundle_ref),
            crate::preserves_rail::string(config_ref),
            crate::preserves_rail::string(binding_ref),
            crate::preserves_rail::string(&input.actual_binding.binding_ref),
            crate::preserves_rail::string(&input.binding.service_id),
            crate::preserves_rail::string(&effect.operation),
            crate::preserves_rail::string(&effect.input_schema_ref),
            crate::preserves_rail::string(&effect.output_schema_ref),
            crate::preserves_rail::string(&effect.request_ref),
            crate::preserves_rail::u64_value(effect.generation),
            crate::preserves_rail::u64_value(effect.incarnation),
            crate::preserves_rail::u64_value(effect.accounted_bytes),
            crate::preserves_rail::u64_value(tick),
        ],
    ))?;
    Ok(crate::node_service_hosting::OperationRequest {
        operation: "route-effect".to_owned(),
        subject: input.binding.service_id.clone(),
        logical_tick: tick,
        state_ref: config_ref.to_owned(),
        input_ref,
    })
}

fn validate_route(input: &RouteInput<'_>) -> crate::error::Result<()> {
    let configured = input.binding;
    let actual = input.actual_binding;
    let effect = input.effect;
    let crate::system_extension::EffectTarget::FabricPort(key) = &effect.target else {
        return Err(denied());
    };
    if actual.binding_ref != configured.port_binding_ref
        || actual.binding.key != *key
        || key.port_id != configured.port_id
        || key.version != configured.port_version
        || effect.operation != configured.operation
        || effect.generation != configured.generation
        || key.port_id != crate::collaboration_scope::COMMIT_PORT_ID
        || key.version != crate::collaboration_scope::COMMIT_PORT_VERSION
        || effect.operation != crate::collaboration_scope::COMMIT_OPERATION
        || effect.input_schema_ref != crate::collaboration_scope::COMMIT_INPUT_SCHEMA
        || effect.output_schema_ref != crate::collaboration_scope::COMMIT_OUTPUT_SCHEMA
    {
        return Err(denied());
    }
    validate_metadata(actual)
}

// A CanonicalPortBinding contains public mutable raw metadata as well as its claimed
// digest. Verify both, rather than accepting a correct reference alongside substituted fields.
fn validate_metadata(actual: &crate::fabric::CanonicalPortBinding) -> crate::error::Result<()> {
    if crate::preserves_rail::canonical_hash(&actual.binding_value)? != actual.binding_ref
        || crate::preserves_rail::canonical_hash(&actual.descriptor_value)? != actual.descriptor_ref
        || crate::preserves_rail::canonical_hash(&actual.registry_value)? != actual.registry_ref
    {
        return Err(denied());
    }
    let fields = crate::preserves_rail::simple_record_fields(&actual.binding_value, "fabric-port-binding-v1", 10)?;
    if crate::preserves_rail::required_string_field(&fields[0], "binding schema")?
        != crate::fabric::FABRIC_PORT_BINDING_SCHEMA
    {
        return Err(denied());
    }
    let binding = &actual.binding;
    for (position, name, expected) in [
        (1, "port-id", binding.key.port_id.as_str()),
        (2, "version", binding.key.version.as_str()),
        (3, "class", binding.class.as_str()),
        (4, "implementation-profile", binding.implementation_profile.as_str()),
        (5, "descriptor-ref", actual.descriptor_ref.as_str()),
        (6, "registry-ref", actual.registry_ref.as_str()),
    ] {
        let value = crate::preserves_rail::value_to_iovalue(&fields[position]);
        let field = crate::preserves_rail::simple_record_fields(&value, name, 1)?;
        if crate::preserves_rail::required_string_field(&field[0], name)? != expected {
            return Err(denied());
        }
    }
    validate_strings(&fields[7], "conformance-refs", binding.conformance_refs.iter().map(String::as_str))?;
    validate_strings(&fields[8], "non-claims", binding.non_claims.iter().map(|claim| claim.as_str()))?;
    Ok(())
}

fn validate_strings<'a>(
    value: &preserves::Value<preserves::IOValue>,
    name: &str,
    expected: impl Iterator<Item = &'a str>,
) -> crate::error::Result<()> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, name, 1)?;
    let actual = crate::preserves_rail::value_to_iovalue(&fields[0]);
    let expected = crate::preserves_rail::sequence(expected.map(crate::preserves_rail::string).collect());
    if actual != expected {
        return Err(denied());
    }
    Ok(())
}

fn denied() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("service host concrete effect authority denied")
}
