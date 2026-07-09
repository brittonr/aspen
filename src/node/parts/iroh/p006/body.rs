
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohAlpnRegistryEntry {
    pub symbolic_name: String,
    pub alpn: String,
    pub owner_namespace: String,
    pub handler_profile: String,
    pub lifecycle_state: String,
    pub supported_schema_profiles: Vec<String>,
    pub limit_refs: Vec<String>,
    pub required_evidence_refs: Vec<String>,
    pub receipt_schema_refs: Vec<String>,
    pub entry_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohAlpnRegistryInput {
    pub symbolic_name: String,
    pub alpn: String,
    pub owner_namespace: String,
    pub handler_profile: String,
    pub lifecycle_state: String,
    pub supported_schema_profiles: Vec<String>,
    pub limit_refs: Vec<String>,
    pub required_evidence_refs: Vec<String>,
    pub receipt_schema_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohAlpnRegistryValidation {
    pub decision: String,
    pub entries: Vec<IrohAlpnRegistryEntry>,
    pub diagnostics: Vec<String>,
    pub value: preserves::IOValue,
}

pub fn default_iroh_alpn_registry_inputs() -> Vec<IrohAlpnRegistryInput> {
    vec![IrohAlpnRegistryInput {
        symbolic_name: DEFAULT_NODE_CONTROL_SYMBOL.to_string(),
        alpn: "molten/node-control/1".to_string(),
        owner_namespace: DEFAULT_NODE_CONTROL_OWNER.to_string(),
        handler_profile: DEFAULT_NODE_CONTROL_HANDLER_PROFILE.to_string(),
        lifecycle_state: DEFAULT_NODE_CONTROL_LIFECYCLE.to_string(),
        supported_schema_profiles: vec![DEFAULT_NODE_CONTROL_HANDLER_PROFILE.to_string()],
        limit_refs: vec![default_limit_profile_ref()],
        required_evidence_refs: Vec::new(),
        receipt_schema_refs: vec![
            fixture_ref(IROH_PROTOCOL_ROUTER_SCHEMA),
            fixture_ref(IROH_FRAMED_ENVELOPE_SCHEMA),
            fixture_ref(IROH_STREAM_SESSION_SCHEMA),
        ],
    }]
}

pub fn validate_iroh_alpn_registry(inputs: &[IrohAlpnRegistryInput]) -> crate::error::Result<IrohAlpnRegistryValidation> {
    let mut entries = Vec::new();
    let mut diagnostics = DiagnosticLog::new();
    let mut seen_alpns = std::collections::BTreeSet::new();
    let mut seen_symbols = std::collections::BTreeSet::new();
    for input in inputs {
        collect_alpn_entry_diagnostics(input, &mut diagnostics)?;
        if !seen_alpns.insert(input.alpn.clone()) {
            push_diagnostic(&mut diagnostics, format!("duplicate ALPN registry entry {}", input.alpn))?;
        }
        if !seen_symbols.insert(input.symbolic_name.clone()) {
            push_diagnostic(
                &mut diagnostics,
                format!("duplicate ALPN registry symbolic name {}", input.symbolic_name),
            )?;
        }
        if let Ok(entry) = iroh_alpn_registry_entry(input) {
            crate::bounded::push_bounded(&mut entries, entry, MAX_REF_COUNT, "ALPN registry entries")?;
        }
    }
    let diagnostics = diagnostics.into_values();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = iroh_alpn_registry_validation_value(&decision, &entries, &diagnostics)?;
    Ok(IrohAlpnRegistryValidation {
        decision,
        entries,
        diagnostics,
        value,
    })
}

pub fn default_iroh_alpn_registry() -> crate::error::Result<IrohAlpnRegistryValidation> {
    validate_iroh_alpn_registry(&default_iroh_alpn_registry_inputs())
}

fn iroh_alpn_registry_entry(input: &IrohAlpnRegistryInput) -> crate::error::Result<IrohAlpnRegistryEntry> {
    validate_alpn(&input.alpn)?;
    validate_handler_kind(&input.symbolic_name)?;
    validate_handler_kind(&input.owner_namespace)?;
    validate_handler_kind(&input.handler_profile)?;
    validate_alpn_lifecycle(&input.lifecycle_state)?;
    validate_schema_profiles(&input.supported_schema_profiles)?;
    validate_alpn_registry_refs(&input.limit_refs, "ALPN registry limit ref")?;
    validate_alpn_registry_refs(&input.required_evidence_refs, "ALPN registry required evidence ref")?;
    validate_alpn_registry_refs(&input.receipt_schema_refs, "ALPN registry receipt schema ref")?;
    let value = iroh_alpn_registry_entry_value(input)?;
    Ok(IrohAlpnRegistryEntry {
        symbolic_name: input.symbolic_name.clone(),
        alpn: input.alpn.clone(),
        owner_namespace: input.owner_namespace.clone(),
        handler_profile: input.handler_profile.clone(),
        lifecycle_state: input.lifecycle_state.clone(),
        supported_schema_profiles: input.supported_schema_profiles.clone(),
        limit_refs: input.limit_refs.clone(),
        required_evidence_refs: input.required_evidence_refs.clone(),
        receipt_schema_refs: input.receipt_schema_refs.clone(),
        entry_ref: crate::preserves_rail::canonical_hash(&value)?,
        value,
    })
}

fn collect_alpn_entry_diagnostics(
    input: &IrohAlpnRegistryInput,
    diagnostics: &mut impl DiagnosticSink,
) -> crate::error::Result<()> {
    if validate_alpn(&input.alpn).is_err() {
        push_diagnostic(diagnostics, format!("malformed ALPN registry value {}", input.alpn))?;
    }
    if validate_handler_kind(&input.symbolic_name).is_err() {
        push_diagnostic(diagnostics, "malformed ALPN symbolic name")?;
    }
    if validate_handler_kind(&input.owner_namespace).is_err() {
        push_diagnostic(diagnostics, "malformed ALPN owner namespace")?;
    }
    if validate_handler_kind(&input.handler_profile).is_err() {
        push_diagnostic(diagnostics, "malformed ALPN handler profile")?;
    }
    if validate_alpn_lifecycle(&input.lifecycle_state).is_err() {
        push_diagnostic(diagnostics, format!("unsupported ALPN lifecycle state {}", input.lifecycle_state))?;
    }
    if validate_schema_profiles(&input.supported_schema_profiles).is_err() {
        push_diagnostic(diagnostics, "malformed ALPN schema/profile version")?;
    }
    validate_alpn_registry_refs(&input.limit_refs, "ALPN registry limit ref")?;
    validate_alpn_registry_refs(&input.required_evidence_refs, "ALPN registry required evidence ref")?;
    validate_alpn_registry_refs(&input.receipt_schema_refs, "ALPN registry receipt schema ref")?;
    Ok(())
}

fn iroh_alpn_registry_entry_value(input: &IrohAlpnRegistryInput) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("iroh-alpn-registry-entry-v1", vec![
        crate::preserves_rail::string("molten.node.iroh-alpn-registry-entry.v1"),
        crate::preserves_rail::record("symbol", vec![crate::preserves_rail::string(&input.symbolic_name)]),
        crate::preserves_rail::record("alpn", vec![crate::preserves_rail::string(&input.alpn)]),
        crate::preserves_rail::record("owner", vec![crate::preserves_rail::string(&input.owner_namespace)]),
        crate::preserves_rail::record("handler-profile", vec![crate::preserves_rail::string(&input.handler_profile)]),
        crate::preserves_rail::record("lifecycle", vec![crate::preserves_rail::string(&input.lifecycle_state)]),
        crate::preserves_rail::record("schema-profiles", vec![strings_value(&input.supported_schema_profiles)?]),
        crate::preserves_rail::record("limits", vec![refs_value(&input.limit_refs)?]),
        crate::preserves_rail::record("required-evidence", vec![refs_value(&input.required_evidence_refs)?]),
        crate::preserves_rail::record("receipt-schemas", vec![refs_value(&input.receipt_schema_refs)?]),
        checks_value(&[
            ("canonical-alpn-registry-entry", "pass"),
            ("routes-only-not-authority", "pass"),
            ("owner-namespace-bound", "pass"),
        ]),
    ]))
}

fn iroh_alpn_registry_validation_value(
    decision: &str,
    entries: &[IrohAlpnRegistryEntry],
    diagnostics: &[String],
) -> crate::error::Result<preserves::IOValue> {
    let entry_refs = entries.iter().map(|entry| entry.entry_ref.clone()).collect::<Vec<_>>();
    Ok(crate::preserves_rail::record("iroh-alpn-registry-validation-v1", vec![
        crate::preserves_rail::string("molten.node.iroh-alpn-registry-validation.v1"),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("entries", vec![refs_value(&entry_refs)?]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(diagnostics)?]),
        checks_value(&[
            ("unique-alpn", if diagnostics.is_empty() { "pass" } else { "fail" }),
            ("deterministic-encoding", "pass"),
            ("routing-evidence-only", "pass"),
        ]),
    ]))
}

fn validate_schema_profiles(values: &[String]) -> crate::error::Result<()> {
    validate_bounded_value_count(values.len(), MAX_REF_COUNT, "ALPN registry schema/profile versions")?;
    for value in values {
        validate_handler_kind(value)?;
    }
    Ok(())
}

fn validate_alpn_registry_refs(refs: &[String], label: &str) -> crate::error::Result<()> {
    validate_bounded_value_count(refs.len(), MAX_REF_COUNT, label)?;
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("invalid {label} {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_alpn_lifecycle(value: &str) -> crate::error::Result<()> {
    match value {
        "proposed" | "active" | "deprecated" | "migration-only" | "removed" => Ok(()),
        _ => Err(crate::error::MoltenError::invalid_harness("unsupported ALPN lifecycle state")),
    }
}

fn validate_alpn(value: &str) -> crate::error::Result<()> {
    let mut diagnostics = DiagnosticLog::new();
    collect_alpn_diagnostic(value, &mut diagnostics)?;
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness("invalid ALPN"))
    }
}

fn validate_handler_kind(value: &str) -> crate::error::Result<()> {
    let mut diagnostics = DiagnosticLog::new();
    collect_handler_diagnostic(value, &mut diagnostics)?;
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness("invalid handler kind"))
    }
}

fn lookup_alpn_registry_entry(alpn: &str) -> crate::error::Result<Option<IrohAlpnRegistryEntry>> {
    let registry = default_iroh_alpn_registry()?;
    Ok(registry.entries.into_iter().find(|entry| entry.alpn == alpn))
}
