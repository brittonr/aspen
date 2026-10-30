#[derive(clap::Args)]
pub struct Arguments {
    #[arg(long)]
    pub cohort: std::path::PathBuf,
    #[arg(long)]
    pub source_trust: std::path::PathBuf,
    #[arg(long)]
    pub signed_output: std::path::PathBuf,
    #[arg(long)]
    pub issuer_key: std::path::PathBuf,
    #[arg(long)]
    pub holder_key: std::path::PathBuf,
    #[arg(long)]
    pub policy: std::path::PathBuf,
    #[arg(long)]
    pub request: std::path::PathBuf,
    #[arg(long)]
    pub nonce_file: std::path::PathBuf,
    #[arg(long, value_enum, default_value = "text")]
    pub request_format: Format,
}

#[derive(Clone, clap::ValueEnum)]
pub enum Format {
    Text,
    Packed,
}

pub(super) fn authorize(args: Arguments) -> molten::error::Result<()> {
    let root = molten::node_state::DirectoryRoot::open_existing(&args.cohort)?;
    let trust_bytes = super::source::read_input(&args.source_trust, 16_384)?;
    if root.read(&molten::node_state::RelativePath::parse("source-gate-trust.json")?, 16_384)? != trust_bytes {
        return Err(super::invalid("explicit independent source selection differs from authored cohort"));
    }
    let trust: serde_json::Value =
        serde_json::from_slice(&trust_bytes).map_err(|_| super::invalid("invalid source trust JSON"))?;
    if trust.get("output").and_then(serde_json::Value::as_str) != args.signed_output.to_str() {
        return Err(super::invalid("explicit signed output differs from independently selected cohort"));
    }
    let validation = molten::node_daemon::verify_selected_node_output_with_root(&root)?;
    let bytes = root.read(&molten::node_state::RelativePath::parse("admission.preserves")?, 16_777_216)?;
    let bundle_ref = molten::preserves_rail::content_ref_from_bytes(&bytes);
    let bundle = molten::node_service_host::codec::decode(&bytes, &bundle_ref)?;
    if bundle
        .services
        .iter()
        .chain(bundle.replacements.iter().map(|replacement| &replacement.service))
        .any(|service| service.executable.source_gate_ref != validation.expected_startup_validation_ref)
    {
        return Err(super::invalid("cohort source admission differs from actual verified selected output"));
    }
    let mut operator = super::operator::Authority::open(&args.issuer_key, &args.holder_key, &args.policy, &root)?;
    operator.bind_existing_trust(&root)?;
    let request = request(&args.request, args.request_format)?;
    let nonce: [u8; 32] = super::source::read_input(&args.nonce_file, 32)?
        .as_slice()
        .try_into()
        .map_err(|_| super::invalid("invocation nonce must contain exactly 32 raw bytes"))?;
    operator.authorize(&root, &bundle_ref, &request, nonce)?;
    let reference = molten::node_service_host::authority::request_ref(&bundle_ref, &request)?;
    println!(
        "{}",
        molten::preserves_rail::to_text(&molten::preserves_rail::record(
            "experimental-exact-invocation-authored-v1",
            vec![
                molten::preserves_rail::string(&bundle_ref),
                molten::preserves_rail::string(reference),
                molten::preserves_rail::string("signature-is-not-execution-evidence"),
            ]
        ))?
    );
    Ok(())
}

fn request(
    path: &std::path::Path,
    format: Format,
) -> molten::error::Result<molten::node_service_hosting::OperationRequest> {
    let bytes = super::source::read_input(path, 65_536)?;
    let value = match format {
        Format::Packed => molten::preserves_rail::strict_canonical_decode(&bytes)?.value,
        Format::Text => molten::preserves_rail::parse_text(
            std::str::from_utf8(&bytes).map_err(|_| super::invalid("preview request text is not UTF-8"))?,
        )?,
    };
    let fields = molten::preserves_rail::simple_record_fields(&value, "node-service-host-operation-request-v1", 5)?;
    let number = fields[2]
        .as_u64()
        .ok_or_else(|| super::invalid("logical tick must be u64"))?
        .map_err(|_| super::invalid("logical tick is out of range"))?;
    Ok(molten::node_service_hosting::OperationRequest {
        operation: molten::preserves_rail::required_string_field(&fields[0], "operation")?,
        subject: molten::preserves_rail::required_string_field(&fields[1], "subject")?,
        logical_tick: number,
        state_ref: molten::preserves_rail::required_content_ref_string(&fields[3], "state ref")?,
        input_ref: molten::preserves_rail::required_content_ref_string(&fields[4], "input ref")?,
    })
}
