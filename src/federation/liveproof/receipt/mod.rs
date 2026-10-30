mod checks;
pub(super) mod offline;

/// Public verification material. Trust this value out of band, never from a bundle.
pub struct PublicContext {
    pub provider_key: String,
    pub provider_ref: String,
    pub receiver_key: String,
    pub receiver_ref: String,
    pub provider_generation: u64,
    pub receiver_generation: u64,
    pub service_generation: u64,
    pub context_ref: String,
    pub policy_ref: String,
    pub observed_at: u64,
    pub max_age: u64,
}
impl PublicContext {
    pub fn value(&self, profile: &crate::fabric_crypto_identity::Profile) -> preserves::IOValue {
        crate::preserves_rail::record("federation-public-context-v1", vec![
            profile.value.clone(),
            crate::preserves_rail::string(&self.provider_key),
            crate::preserves_rail::string(&self.provider_ref),
            crate::preserves_rail::string(&self.receiver_key),
            crate::preserves_rail::string(&self.receiver_ref),
            crate::preserves_rail::u64_value(self.provider_generation),
            crate::preserves_rail::u64_value(self.receiver_generation),
            crate::preserves_rail::u64_value(self.service_generation),
            crate::preserves_rail::string(&self.context_ref),
            crate::preserves_rail::string(&self.policy_ref),
            crate::preserves_rail::u64_value(self.observed_at),
            crate::preserves_rail::u64_value(self.max_age),
        ])
    }
    fn verification<'a>(
        &'a self,
        profile: &'a crate::fabric_crypto_identity::Profile,
        purpose: molten_core::federation::RecordPurpose,
        provider: bool,
    ) -> crate::mesh::envelope::signatures::VerificationInput<'a> {
        crate::mesh::envelope::signatures::VerificationInput {
            profile,
            public_key: if provider {
                &self.provider_key
            } else {
                &self.receiver_key
            },
            signer_ref: if provider {
                &self.provider_ref
            } else {
                &self.receiver_ref
            },
            generation: if provider {
                self.provider_generation
            } else {
                self.receiver_generation
            },
            currentness: crate::fabric_crypto_identity::KeyCurrentness::Current,
            context_ref: &self.context_ref,
            policy_ref: &self.policy_ref,
            purpose,
            now: self.observed_at,
            max_age: self.max_age,
        }
    }
}

pub struct Evidence {
    pub inventory: crate::mesh::envelope::signatures::SignedRecord,
    pub request: crate::mesh::envelope::signatures::SignedRecord,
    pub response: crate::mesh::envelope::signatures::SignedRecord,
    pub transport: Vec<preserves::IOValue>,
    pub dag: preserves::IOValue,
    pub content: preserves::IOValue,
    pub admission: Vec<preserves::IOValue>,
    pub import: preserves::IOValue,
    /// Canonical artifact read back through the receiver's ledger capability.
    pub registry_readback: preserves::IOValue,
    pub supervised: Vec<preserves::IOValue>,
}

pub struct VerifiedBundle {
    pub bundle_ref: String,
    pub context_ref: String,
    pub artifact_ref: String,
    pub import_ref: String,
}

/// Seals the exact detached evidence, not a claim of production readiness.
pub fn seal(
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    context: &PublicContext,
    evidence: &Evidence,
    signing: &crate::mesh::envelope::signatures::SigningInput<'_>,
) -> crate::error::Result<Vec<u8>> {
    if signing.handle.public_key_ref != context.receiver_ref
        || signing.context_ref != context.context_ref
        || signing.policy_ref != context.policy_ref
        || signing.purpose != molten_core::federation::RecordPurpose::Receipt
    {
        return Err(invalid("bundle seal must use the receiver's bound production identity"));
    }
    let body = body(context, adapter.profile(), evidence)?;
    let signature =
        crate::mesh::envelope::signatures::sign_record(adapter, &crate::mesh::envelope::signatures::SigningInput {
            payload: &body,
            ..*signing
        })?;
    let signature =
        crate::preserves_rail::parse_canonical_bytes(&crate::mesh::envelope::wire::encode(&signature, MAXIMUM_BYTES)?)?;
    let bundle = crate::preserves_rail::record("federation-detached-bundle-v1", vec![body, signature]);
    let bytes = crate::preserves_rail::canonical_bytes(&bundle)?;
    verify(adapter, context, &bytes)?;
    Ok(bytes)
}

pub const MAXIMUM_BYTES: u64 = 4 * 1024 * 1024;

/// No network, key generation, secret reads, or local registry mutation.
/// The caller supplies an already admitted production crypto adapter and trusted public context.
pub fn verify(
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    context: &PublicContext,
    bytes: &[u8],
) -> crate::error::Result<VerifiedBundle> {
    if u64::try_from(bytes.len()).ok().is_none_or(|length| length > MAXIMUM_BYTES) {
        return Err(invalid("detached bundle exceeds byte bound"));
    }
    let bundle = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    let outer = bundle
        .collect_simple_record("federation-detached-bundle-v1", Some(2))
        .ok_or_else(|| invalid("malformed detached bundle"))?;
    let body = crate::preserves_rail::value_to_iovalue(&outer[0]);
    let fields = body
        .collect_simple_record("federation-detached-evidence-v1", Some(11))
        .ok_or_else(|| invalid("malformed detached evidence"))?;
    let fields = fields.fields_iter().cloned().collect::<Vec<preserves::Value<preserves::IOValue>>>();
    let expected_context = context.value(adapter.profile());
    if crate::preserves_rail::value_to_iovalue(&fields[0]) != expected_context {
        return Err(invalid("detached public context differs from trusted context"));
    }
    let seal_input = context.verification(adapter.profile(), molten_core::federation::RecordPurpose::Receipt, false);
    let signature = decode(&outer[1], &seal_input)?;
    crate::mesh::envelope::signatures::verify_record(adapter, &signature, &seal_input)?;
    if signature.payload != body {
        return Err(invalid("detached seal does not bind complete evidence"));
    }
    let inventory = verified(adapter, &fields[1], context, molten_core::federation::RecordPurpose::Inventory, true)?;
    let request = verified(adapter, &fields[2], context, molten_core::federation::RecordPurpose::Request, false)?;
    let response = verified(adapter, &fields[3], context, molten_core::federation::RecordPurpose::Response, true)?;
    let artifact = crate::preserves_rail::value_to_iovalue(&fields[9]);
    let artifact_ref = crate::preserves_rail::canonical_hash(&artifact)?;
    checks::correlate(&inventory, &request, &response, &artifact_ref)?;
    validate_evidence(&fields, &artifact_ref, context.service_generation)?;
    Ok(VerifiedBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(&bundle)?,
        context_ref: crate::preserves_rail::canonical_hash(&expected_context)?,
        artifact_ref,
        import_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::value_to_iovalue(&fields[8]))?,
    })
}

fn body(
    context: &PublicContext,
    profile: &crate::fabric_crypto_identity::Profile,
    evidence: &Evidence,
) -> crate::error::Result<preserves::IOValue> {
    let record = |signed| {
        crate::preserves_rail::parse_canonical_bytes(&crate::mesh::envelope::wire::encode(signed, MAXIMUM_BYTES)?)
    };
    Ok(crate::preserves_rail::record("federation-detached-evidence-v1", vec![
        context.value(profile),
        record(&evidence.inventory)?,
        record(&evidence.request)?,
        record(&evidence.response)?,
        crate::preserves_rail::sequence(evidence.transport.clone()),
        evidence.dag.clone(),
        evidence.content.clone(),
        crate::preserves_rail::sequence(evidence.admission.clone()),
        evidence.import.clone(),
        evidence.registry_readback.clone(),
        crate::preserves_rail::sequence(evidence.supervised.clone()),
    ]))
}
fn decode(
    value: &preserves::Value<preserves::IOValue>,
    input: &crate::mesh::envelope::signatures::VerificationInput<'_>,
) -> crate::error::Result<crate::mesh::envelope::signatures::SignedRecord> {
    crate::mesh::envelope::wire::decode(
        &crate::preserves_rail::canonical_bytes(&crate::preserves_rail::value_to_iovalue(value))?,
        input,
        MAXIMUM_BYTES,
    )
}
fn verified(
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    value: &preserves::Value<preserves::IOValue>,
    context: &PublicContext,
    purpose: molten_core::federation::RecordPurpose,
    provider: bool,
) -> crate::error::Result<crate::mesh::envelope::signatures::SignedRecord> {
    let input = context.verification(adapter.profile(), purpose, provider);
    let record = decode(value, &input)?;
    crate::mesh::envelope::signatures::verify_record(adapter, &record, &input)?;
    Ok(record)
}
fn validate_evidence(
    fields: &[preserves::Value<preserves::IOValue>],
    artifact_ref: &str,
    generation: u64,
) -> crate::error::Result<()> {
    let imported = crate::preserves_rail::value_to_iovalue(&fields[8]);
    let imported = imported
        .collect_simple_record("federation-import-complete-v1", Some(5))
        .ok_or_else(|| invalid("bundle lacks completed local import receipt"))?;
    let imported = imported.fields_iter().cloned().collect::<Vec<preserves::Value<preserves::IOValue>>>();
    let dag = crate::preserves_rail::value_to_iovalue(&fields[5]);
    if imported[2].as_string().as_deref() != Some(artifact_ref)
        || imported[1].as_u64().and_then(|number| number.ok()) != Some(generation)
        || imported[3].as_string().as_deref() != Some(checks::dag_ref(&dag)?.as_str())
    {
        return Err(invalid("import receipt does not bind artifact and DAG receipt"));
    }
    let wire = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let wire = wire
        .collect_simple_record("federation-wire-v1", Some(6))
        .ok_or_else(|| invalid("missing signed fetch"))?;
    let fetch = crate::preserves_rail::value_to_iovalue(&wire[4]);
    let fetch = fetch
        .collect_simple_record("federation-fetch-v1", Some(4))
        .ok_or_else(|| invalid("missing fetch session"))?;
    if fetch[0] != imported[0] {
        return Err(invalid("detached fetch and local import sessions differ"));
    }
    let gates = fields[7].collect_sequence().ok_or_else(|| invalid("bundle lacks local admission evidence"))?;
    let gates = gates.iter().cloned().collect::<Vec<preserves::Value<preserves::IOValue>>>();
    let refs = imported[4].collect_sequence().ok_or_else(|| invalid("import lacks admission references"))?;
    checks::gates(&gates, &imported)?;
    checks::content(fields)?;
    for gate in gates {
        let reference = crate::preserves_rail::canonical_hash(&crate::preserves_rail::value_to_iovalue(&gate))?;
        if !refs.iter().any(|value| value.as_string().as_deref() == Some(reference.as_str())) {
            return Err(invalid("local admission evidence is not bound by import receipt"));
        }
    }
    checks::transport(fields)?;
    checks::supervision(fields, generation)?;
    Ok(())
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
