//! Canonical collaboration runtime configuration is data, never admission authority.

pub const SELECTOR: &str = "service-host-effects";
pub const MAX_BINDINGS: usize = 32;
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 4096;
const CONFIG_SCHEMA: &str = "service-host-effects-v1";
const BINDING_SCHEMA: &str = "service-host-collaboration-effect-binding-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    pub bindings: Vec<Binding>,
}

/// Exact installation inputs; current authority and actual runtime checks are separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub service_id: String,
    pub manifest_ref: String,
    pub port_binding_ref: String,
    pub port_id: String,
    pub port_version: String,
    pub operation: String,
    pub group_ref: String,
    pub generation: u64,
    pub policy_ref: String,
    pub resource_ref: String,
    pub installation_ref: String,
    pub runtime_artifact: String,
    pub runtime_ref: String,
}

/// Produces the one canonical wire representation, without opening any resources.
pub fn encode(configuration: &Configuration) -> crate::error::Result<Vec<u8>> {
    validate(configuration)?;
    let bindings = configuration.bindings.iter().map(binding_value).collect();
    let value = crate::preserves_rail::record(CONFIG_SCHEMA, vec![crate::preserves_rail::sequence(bindings)]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(invalid("effect configuration byte bound exceeded"));
    }
    Ok(bytes)
}

pub fn reference(configuration: &Configuration) -> crate::error::Result<String> {
    Ok(crate::preserves_rail::content_ref_from_bytes(&encode(configuration)?))
}

/// Rejects noncanonical bytes, unknown records/fields and any exact-hash mismatch.
pub fn decode(bytes: &[u8], expected_ref: &str) -> crate::error::Result<Configuration> {
    crate::preserves_rail::ContentRef::parse(expected_ref)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(invalid("effect configuration byte bound exceeded"));
    }
    if crate::preserves_rail::content_ref_from_bytes(bytes) != expected_ref {
        return Err(invalid("effect configuration content reference mismatch"));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    let fields = value
        .collect_simple_record(CONFIG_SCHEMA, Some(1))
        .ok_or_else(|| invalid("expected exact effect configuration record"))?;
    let sequence = crate::preserves_rail::value_to_iovalue(&fields[0]);
    let items = sequence.collect_sequence().ok_or_else(|| invalid("expected effect binding sequence"))?;
    if items.len() > MAX_BINDINGS {
        return Err(invalid("effect configuration binding bound exceeded"));
    }
    let bindings = items
        .iter()
        .map(|item| parse_binding(&crate::preserves_rail::value_to_iovalue(item)))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let configuration = Configuration { bindings };
    validate(&configuration)?;
    Ok(configuration)
}

/// Independently computable exact binding identity; it grants no effect permission.
pub fn binding_ref(binding: &Binding) -> crate::error::Result<String> {
    validate_binding(binding)?;
    Ok(crate::preserves_rail::content_ref_from_bytes(&crate::preserves_rail::canonical_bytes(
        &binding_value(binding),
    )?))
}

fn binding_value(binding: &Binding) -> preserves::IOValue {
    crate::preserves_rail::record(BINDING_SCHEMA, vec![
        crate::preserves_rail::string(&binding.service_id),
        crate::preserves_rail::string(&binding.manifest_ref),
        crate::preserves_rail::string(&binding.port_binding_ref),
        crate::preserves_rail::string(&binding.port_id),
        crate::preserves_rail::string(&binding.port_version),
        crate::preserves_rail::string(&binding.operation),
        crate::preserves_rail::string(&binding.group_ref),
        preserves::IOValue::new(binding.generation),
        crate::preserves_rail::string(&binding.policy_ref),
        crate::preserves_rail::string(&binding.resource_ref),
        crate::preserves_rail::string(&binding.installation_ref),
        crate::preserves_rail::string(&binding.runtime_artifact),
        crate::preserves_rail::string(&binding.runtime_ref),
    ])
}

fn parse_binding(value: &preserves::IOValue) -> crate::error::Result<Binding> {
    let fields = value
        .collect_simple_record(BINDING_SCHEMA, Some(13))
        .ok_or_else(|| invalid("expected exact collaboration effect binding record"))?;
    let fields: [preserves::IOValue; 13] =
        std::array::from_fn(|index| crate::preserves_rail::value_to_iovalue(&fields[index]));
    let binding = Binding {
        service_id: text(&fields[0])?,
        manifest_ref: text(&fields[1])?,
        port_binding_ref: text(&fields[2])?,
        port_id: text(&fields[3])?,
        port_version: text(&fields[4])?,
        operation: text(&fields[5])?,
        group_ref: text(&fields[6])?,
        generation: fields[7]
            .as_u64()
            .ok_or_else(|| invalid("expected generation integer"))?
            .map_err(|_| invalid("generation overflow"))?,
        policy_ref: text(&fields[8])?,
        resource_ref: text(&fields[9])?,
        installation_ref: text(&fields[10])?,
        runtime_artifact: text(&fields[11])?,
        runtime_ref: text(&fields[12])?,
    };
    Ok(binding)
}

fn validate(configuration: &Configuration) -> crate::error::Result<()> {
    if configuration.bindings.len() > MAX_BINDINGS {
        return Err(invalid("effect configuration binding bound exceeded"));
    }
    let mut services = std::collections::BTreeSet::new();
    for binding in &configuration.bindings {
        if !services.insert(&binding.service_id) {
            return Err(invalid("duplicate effect service configuration"));
        }
        validate_binding(binding)?;
    }
    Ok(())
}

fn validate_binding(binding: &Binding) -> crate::error::Result<()> {
    for value in [
        &binding.service_id,
        &binding.manifest_ref,
        &binding.port_binding_ref,
        &binding.port_id,
        &binding.port_version,
        &binding.operation,
        &binding.group_ref,
        &binding.policy_ref,
        &binding.resource_ref,
        &binding.installation_ref,
        &binding.runtime_artifact,
        &binding.runtime_ref,
    ] {
        validate_text(value)?;
    }
    for reference in [
        &binding.manifest_ref,
        &binding.port_binding_ref,
        &binding.group_ref,
        &binding.policy_ref,
        &binding.resource_ref,
        &binding.installation_ref,
        &binding.runtime_ref,
    ] {
        crate::preserves_rail::ContentRef::parse(reference)?;
    }
    if binding.port_id != crate::collaboration_scope::COMMIT_PORT_ID
        || binding.port_version != crate::collaboration_scope::COMMIT_PORT_VERSION
        || binding.operation != crate::collaboration_scope::COMMIT_OPERATION
    {
        return Err(invalid("unsupported collaboration effect protocol"));
    }
    molten_node_host::node_state::RelativePath::parse(&binding.runtime_artifact)
        .map_err(|error| invalid(format!("invalid owned runtime artifact path: {error}")))?;
    Ok(())
}

fn text(value: &preserves::IOValue) -> crate::error::Result<String> {
    let value = value.as_string().ok_or_else(|| invalid("expected effect configuration text"))?;
    validate_text(&value)?;
    Ok(value.to_string())
}

fn validate_text(value: &str) -> crate::error::Result<()> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(invalid("effect configuration text must be nonempty, bounded and control-free"));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

#[cfg(test)]
mod tests {
    #[test]
    fn changed_binding_requires_its_own_exact_configuration_and_binding_hash() {
        let original = super::Configuration {
            bindings: vec![binding()],
        };
        let bytes = super::encode(&original).expect("encode");
        let reference = super::reference(&original).expect("reference");
        assert_eq!(super::decode(&bytes, &reference).expect("decode"), original);
        let mut changed = original.clone();
        changed.bindings[0].generation += 1;
        assert_ne!(
            super::binding_ref(&original.bindings[0]).expect("binding hash"),
            super::binding_ref(&changed.bindings[0]).expect("changed binding hash")
        );
        assert!(super::decode(&super::encode(&changed).expect("encode changed"), &reference).is_err());
    }

    #[test]
    fn bounds_duplicates_paths_and_protocol_are_fail_closed() {
        let mut configuration = super::Configuration {
            bindings: vec![binding(); super::MAX_BINDINGS],
        };
        for (index, binding) in configuration.bindings.iter_mut().enumerate() {
            binding.service_id = format!("service-{index}");
        }
        let bytes = super::encode(&configuration).expect("maximum bindings");
        let reference = super::reference(&configuration).expect("reference");
        assert_eq!(super::decode(&bytes, &reference).expect("decode maximum"), configuration);
        configuration.bindings.push(binding());
        assert!(super::encode(&configuration).is_err());
        assert!(
            super::encode(&super::Configuration {
                bindings: vec![binding(), binding()]
            })
            .is_err()
        );
        for path in ["../runtime", "/runtime", "runtime/../other"] {
            let mut invalid = binding();
            invalid.runtime_artifact = path.to_string();
            assert!(super::binding_ref(&invalid).is_err());
        }
        let mut invalid = binding();
        invalid.port_version = "v2".to_string();
        assert!(super::binding_ref(&invalid).is_err());
        invalid = binding();
        invalid.runtime_ref = "not-a-content-reference".to_string();
        assert!(super::binding_ref(&invalid).is_err());
    }

    fn binding() -> super::Binding {
        let reference = crate::preserves_rail::content_ref_from_bytes(b"configuration-test");
        super::Binding {
            service_id: "collaboration-test".to_string(),
            manifest_ref: reference.clone(),
            port_binding_ref: reference.clone(),
            port_id: crate::collaboration_scope::COMMIT_PORT_ID.to_string(),
            port_version: crate::collaboration_scope::COMMIT_PORT_VERSION.to_string(),
            operation: crate::collaboration_scope::COMMIT_OPERATION.to_string(),
            group_ref: reference.clone(),
            generation: 1,
            policy_ref: reference.clone(),
            resource_ref: reference.clone(),
            installation_ref: reference.clone(),
            runtime_artifact: "effects/runtime.preserves".to_string(),
            runtime_ref: reference,
        }
    }
}
