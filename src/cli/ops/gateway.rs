type Outcome<T> = molten::error::Result<T>;

const GATEWAY_CHUNK_SIZE: u64 = 4;
const GATEWAY_RANGE_OFFSET: u64 = 1;
const GATEWAY_RANGE_LENGTH: u64 = 6;
const GATEWAY_MEMBER_SIZE: u64 = 7;
const FIRST_GATEWAY_FIXTURE_ID: u64 = 1;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum GatewayCommand {
    RangeFixture {
        #[arg(long)]
        root: std::path::PathBuf,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    IndexFixture {
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
}

pub(crate) fn run_gateway_command(command: GatewayCommand) -> Outcome<()> {
    match command {
        GatewayCommand::RangeFixture { root, out } => range_fixture(root, out),
        GatewayCommand::IndexFixture { out } => index_fixture(out),
    }
}

fn range_fixture(root: std::path::PathBuf, out: Option<std::path::PathBuf>) -> Outcome<()> {
    let _ = std::fs::remove_dir_all(&root);
    let body = b"operator gateway fixture";
    let put = molten::chunk_store::put_bytes(&root, "operator-gateway-artifact", body, GATEWAY_CHUNK_SIZE)?;
    let manifest = molten::chunk_store::parse_manifest_value(&put.manifest_value, Some(&put.manifest_ref))?;
    let mut chunk_bytes = std::collections::BTreeMap::new();
    for (index, chunk) in manifest.chunks.iter().enumerate() {
        let start = index * GATEWAY_CHUNK_SIZE as usize;
        let end = (start + GATEWAY_CHUNK_SIZE as usize).min(body.len());
        chunk_bytes.insert(chunk.chunk_ref.clone(), body[start..end].to_vec());
    }
    let verification =
        molten::operator_gateway::verify_gateway_range(&molten::operator_gateway::GatewayRangeVerificationInput {
            read: molten::operator_gateway::GatewayReadInput {
                object_ref: manifest.manifest_ref.clone(),
                member: None,
                requested_range: Some(molten::operator_gateway::GatewayRange {
                    offset: GATEWAY_RANGE_OFFSET,
                    length: GATEWAY_RANGE_LENGTH,
                }),
                requester_ref: fixture_ref("gateway-operator"),
                manifest: Some(&manifest),
                visibility: visibility(),
            },
            chunk_bytes,
        })?;
    emit(out.as_ref(), "operator-gateway-range-fixture", &verification.receipt_value)
}

fn index_fixture(out: Option<std::path::PathBuf>) -> Outcome<()> {
    let hidden_ref = fixture_ref("gateway-hidden");
    let index = molten::operator_gateway::decide_index(&molten::operator_gateway::GatewayIndexInput {
        bundle_ref: fixture_ref("gateway-bundle"),
        requester_ref: fixture_ref("gateway-operator"),
        visibility: molten::operator_gateway::GatewayVisibility {
            hidden_refs: vec![hidden_ref.clone()],
            ..visibility()
        },
        members: vec![
            molten::operator_gateway::GatewayMember {
                name: "release-evidence.preserves".to_string(),
                object_ref: fixture_ref("gateway-visible"),
                size: GATEWAY_MEMBER_SIZE,
                mime_hint: Some("application/preserves".to_string()),
                sensitive: false,
                visible: true,
            },
            molten::operator_gateway::GatewayMember {
                name: "secret.txt".to_string(),
                object_ref: hidden_ref,
                size: GATEWAY_MEMBER_SIZE,
                mime_hint: Some("text/plain".to_string()),
                sensitive: true,
                visible: true,
            },
        ],
    })?;
    emit(out.as_ref(), "operator-gateway-index-fixture", &index.receipt_value)
}

fn visibility() -> molten::operator_gateway::GatewayVisibility {
    molten::operator_gateway::GatewayVisibility {
        profile: "public".to_string(),
        visibility_policy_refs: vec![fixture_ref("gateway-visibility")],
        retention_refs: vec![fixture_ref("gateway-retention")],
        reveal_refs: Vec::new(),
        redaction_refs: vec![fixture_ref("gateway-redaction")],
        hidden_refs: Vec::new(),
        allow_sensitive_names: false,
    }
}

fn emit(out: Option<&std::path::PathBuf>, label: &str, value: &preserves::IOValue) -> Outcome<()> {
    let text = molten::preserves_rail::to_text(value)?;
    let reference = molten::preserves_rail::canonical_hash(value)?;
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
        }
        std::fs::write(path, text).map_err(molten::error::MoltenError::from)?;
        println!("{label} {reference} written to {}", path.display());
    } else {
        println!("{text}");
        eprintln!("{label} {reference}");
    }
    Ok(())
}

fn fixture_ref(label: &str) -> String {
    let scoped = format!("{label}-{FIRST_GATEWAY_FIXTURE_ID}");
    molten::preserves_rail::content_ref_from_bytes(scoped.as_bytes())
}
