type Outcome<T> = molten::error::Result<T>;

const GATEWAY_CHUNK_SIZE: u64 = 4;
const GATEWAY_CHUNK_SIZE_USIZE: usize = 4;
const GATEWAY_RANGE_OFFSET: u64 = 1;
const GATEWAY_RANGE_LENGTH: u64 = 6;
const GATEWAY_MEMBER_SIZE: u64 = 7;
const FIRST_GATEWAY_FIXTURE_ID: u64 = 1;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
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

pub(crate) fn run_command(command: Command) -> Outcome<()> {
    match command {
        Command::RangeFixture { root, out } => range_fixture(root, out),
        Command::IndexFixture { out } => index_fixture(out),
    }
}

fn range_fixture(root: std::path::PathBuf, out: Option<std::path::PathBuf>) -> Outcome<()> {
    reset_fixture_root(&root)?;
    let body = b"operator gateway fixture";
    let put = molten::chunk_store::put_bytes(&root, "operator-gateway-artifact", body, GATEWAY_CHUNK_SIZE)?;
    let manifest = molten::chunk_store::parse_manifest_value(&put.manifest_value, Some(&put.manifest_ref))?;
    let chunk_bytes = collect_fixture_chunks(body, &manifest)?;
    let verification = molten::operator_gateway::verify_range(&molten::operator_gateway::RangeVerificationInput {
        read: molten::operator_gateway::ReadInput {
            object_ref: manifest.manifest_ref.clone(),
            member: None,
            requested_range: Some(molten::operator_gateway::Range {
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

fn reset_fixture_root(root: &std::path::Path) -> Outcome<()> {
    match std::fs::remove_dir_all(root) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(molten::error::MoltenError::from(error)),
    }
}

fn collect_fixture_chunks(
    body: &[u8],
    manifest: &molten::chunk_store::ChunkManifest,
) -> Outcome<std::collections::BTreeMap<String, Vec<u8>>> {
    ensure_fixture_chunk_count(body, manifest.chunks.len())?;
    let chunk_bytes = manifest
        .chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            let start = index * GATEWAY_CHUNK_SIZE_USIZE;
            let end = (start + GATEWAY_CHUNK_SIZE_USIZE).min(body.len());
            (chunk.chunk_ref.clone(), body[start..end].to_vec())
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    if chunk_bytes.len() != manifest.chunks.len() {
        return Err(molten::error::MoltenError::invalid_harness(
            "gateway fixture manifest contains duplicate chunk refs",
        ));
    }
    Ok(chunk_bytes)
}

fn ensure_fixture_chunk_count(body: &[u8], chunk_count: usize) -> Outcome<()> {
    let expected_chunks = body.len().div_ceil(GATEWAY_CHUNK_SIZE_USIZE);
    if chunk_count > expected_chunks {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "gateway fixture chunk count {chunk_count} exceeds expected bound {expected_chunks}"
        )));
    }
    Ok(())
}

fn index_fixture(out: Option<std::path::PathBuf>) -> Outcome<()> {
    let hidden_ref = fixture_ref("gateway-hidden");
    let index = molten::operator_gateway::decide_index(&molten::operator_gateway::IndexInput {
        bundle_ref: fixture_ref("gateway-bundle"),
        requester_ref: fixture_ref("gateway-operator"),
        visibility: molten::operator_gateway::Visibility {
            hidden_refs: vec![hidden_ref.clone()],
            ..visibility()
        },
        members: vec![
            molten::operator_gateway::Member {
                name: "release-evidence.preserves".to_string(),
                object_ref: fixture_ref("gateway-visible"),
                size: GATEWAY_MEMBER_SIZE,
                mime_hint: Some("application/preserves".to_string()),
                sensitive: false,
                visible: true,
            },
            molten::operator_gateway::Member {
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

fn visibility() -> molten::operator_gateway::Visibility {
    molten::operator_gateway::Visibility {
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

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &[u8] = b"operator gateway fixture";

    #[test]
    fn chunk_count_allows_exact_fixture_bound() {
        let expected_chunks = BODY.len().div_ceil(GATEWAY_CHUNK_SIZE_USIZE);

        let result = ensure_fixture_chunk_count(BODY, expected_chunks);

        assert!(result.is_ok());
    }

    #[test]
    fn chunk_count_rejects_over_bound() {
        let expected_chunks = BODY.len().div_ceil(GATEWAY_CHUNK_SIZE_USIZE);
        let too_many_chunks = expected_chunks + 1;

        let error =
            ensure_fixture_chunk_count(BODY, too_many_chunks).expect_err("chunk count should reject over-bound input");

        assert!(error.to_string().contains("exceeds expected bound"), "{error}");
    }
}
