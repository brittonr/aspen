pub fn verify(
    root: &crate::node_state::DirectoryRoot,
    trusted_context_ref: &str,
) -> crate::error::Result<super::VerifiedBundle> {
    crate::preserves_rail::validate_content_ref(trusted_context_ref)?;
    let public = root.receipts()?;
    let context = crate::preserves_rail::parse_canonical_bytes(
        &public.read(&crate::node_state::RelativePath::parse("public-context.preserves")?, 8192)?,
    )?;
    if crate::preserves_rail::canonical_hash(&context)? != trusted_context_ref {
        return Err(super::super::invalid("offline context is not the independently selected public context"));
    }
    let identity = root.identity()?;
    let adapter = super::super::setup::profiles::crypto(&identity)?;
    let context = parse_context(&context, adapter.profile())?;
    let bytes =
        public.read(&crate::node_state::RelativePath::parse("detached-bundle.preserves")?, super::MAXIMUM_BYTES)?;
    super::verify(&adapter, &context, &bytes)
}
fn parse_context(
    value: &preserves::IOValue,
    profile: &crate::fabric_crypto_identity::Profile,
) -> crate::error::Result<super::PublicContext> {
    let fields = value
        .collect_simple_record("federation-public-context-v1", Some(12))
        .ok_or_else(|| super::super::invalid("malformed trusted public context"))?;
    let string = |index: usize| {
        fields[index]
            .as_string()
            .map(|text| text.into_owned())
            .ok_or_else(|| super::super::invalid("invalid context string"))
    };
    let number = |index: usize| {
        fields[index]
            .as_u64()
            .and_then(|number| number.ok())
            .ok_or_else(|| super::super::invalid("invalid context number"))
    };
    if crate::preserves_rail::value_to_iovalue(&fields[0]) != profile.value {
        return Err(super::super::invalid("trusted admitted crypto profile mismatch"));
    }
    Ok(super::PublicContext {
        provider_key: string(1)?,
        provider_ref: string(2)?,
        receiver_key: string(3)?,
        receiver_ref: string(4)?,
        provider_generation: number(5)?,
        receiver_generation: number(6)?,
        service_generation: number(7)?,
        context_ref: string(8)?,
        policy_ref: string(9)?,
        observed_at: number(10)?,
        max_age: number(11)?,
    })
}
pub fn readback(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<super::VerifiedBundle> {
    let public = root.receipts()?;
    let context = crate::preserves_rail::parse_canonical_bytes(
        &public.read(&crate::node_state::RelativePath::parse("public-context.preserves")?, 8192)?,
    )?;
    let verified = verify(root, &crate::preserves_rail::canonical_hash(&context)?)?;
    let artifact = crate::ledger::read_artifact_with_root(&root.ledger_store()?, &verified.artifact_ref)?;
    if artifact != super::super::setup::profiles::artifact() {
        return Err(super::super::invalid("receiver registry differs from admitted fixture artifact"));
    }
    Ok(verified)
}
pub fn public_result(verified: &super::VerifiedBundle, mode: &str) -> preserves::IOValue {
    crate::preserves_rail::record("federation-live-proof-v1", vec![
        crate::preserves_rail::string(mode),
        crate::preserves_rail::string(&verified.bundle_ref),
        crate::preserves_rail::string(&verified.context_ref),
        crate::preserves_rail::string(&verified.artifact_ref),
        crate::preserves_rail::string(&verified.import_ref),
        crate::preserves_rail::sequence(
            [
                "not production readiness",
                "not permanent convergence",
                "not remote correctness",
            ]
            .iter()
            .map(crate::preserves_rail::string)
            .collect(),
        ),
        crate::preserves_rail::u64_value(super::super::setup::transport::NOW),
    ])
}
