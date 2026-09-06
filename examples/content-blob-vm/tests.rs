use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "molten-handoff-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("fresh owned test root");
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove owned fixture");
    }
}

fn wire(manifest: &preserves::IOValue, locators: Vec<preserves::IOValue>, provider: iroh::EndpointId) -> Vec<u8> {
    rail::canonical_bytes(&rail::record(
        "live-content-handoff-v1",
        vec![
            manifest.clone(),
            rail::string(reference("backend")),
            rail::string(provider.to_string()),
            rail::sequence(locators),
        ],
    ))
    .unwrap()
}

// r[verify molten.content_live.handoff]
#[test]
fn handoff_admission_checks_membership_bounds_and_independent_expectations_without_network() {
    let root = Fixture::new();
    let put = chunks::put_bytes(&root.0, "artifact", b"aaaabbbbcccc", 4).unwrap();
    let manifest = chunks::read_manifest(&root.0, &put.manifest_ref).unwrap();
    let provider = iroh::SecretKey::from_bytes(&[1; 32]).public();
    let wrong_provider = iroh::SecretKey::from_bytes(&[2; 32]).public();
    let address: SocketAddr = "127.0.0.1:17888".parse().unwrap();
    let entries: Vec<_> = manifest
        .chunks
        .iter()
        .zip([b"aaaa", b"bbbb", b"cccc"])
        .enumerate()
        .map(|(index, (chunk, bytes))| {
            let ticket = iroh_blobs::ticket::BlobTicket::new(
                iroh::EndpointAddr::new(provider).with_ip_addr(address),
                iroh_blobs::Hash::new(bytes),
                iroh_blobs::BlobFormat::Raw,
            );
            rail::record(
                "live-content-locator-v1",
                vec![
                    rail::string(index.to_string()),
                    rail::string(&chunk.chunk_ref),
                    rail::string(ticket.to_string()),
                ],
            )
        })
        .collect();
    let policy: Policy = serde_json::from_slice(POLICY).unwrap();
    let profile = profile(&policy).unwrap();
    let admit = |bytes: &[u8], manifest_ref: &str, provider, address| {
        admit_live_handoff(
            &profile,
            bytes,
            LiveHandoffExpectation {
                manifest_ref,
                provider,
                address,
            },
        )
    };
    let valid = wire(&manifest.value, entries.clone(), provider);
    assert_eq!(
        admit(&valid, &manifest.manifest_ref, provider, address).unwrap().manifest().manifest_ref,
        manifest.manifest_ref
    );
    assert!(admit(&valid, &reference("wrong-manifest"), provider, address).is_err());
    assert!(admit(&valid, &manifest.manifest_ref, wrong_provider, address).is_err());
    assert!(admit(&valid, &manifest.manifest_ref, provider, "0.0.0.0:0".parse().unwrap()).is_err());
    for invalid in [Vec::new(), b"truncated".to_vec(), vec![0; MAX_LIVE_HANDOFF_BYTES + 1]] {
        assert!(admit(&invalid, &manifest.manifest_ref, provider, address).is_err());
    }
    let mut missing = entries.clone();
    missing.pop();
    let mut extra = entries.clone();
    extra.push(entries[0].clone());
    let mut reordered = entries.clone();
    reordered.swap(0, 1);
    let mut duplicate = entries.clone();
    duplicate[1] = entries[0].clone();
    let oversized = rail::record(
        "live-content-locator-v1",
        vec![
            rail::string("0"),
            rail::string(&manifest.chunks[0].chunk_ref),
            rail::string("x".repeat(2049)),
        ],
    );
    let mut long_ticket = entries.clone();
    long_ticket[0] = oversized;
    for invalid in [missing, extra, reordered, duplicate, long_ticket] {
        assert!(admit(&wire(&manifest.value, invalid, provider), &manifest.manifest_ref, provider, address).is_err());
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    assert!(admit(&trailing, &manifest.manifest_ref, provider, address).is_err());
    for (key, format) in [
        (wrong_provider, iroh_blobs::BlobFormat::Raw),
        (provider, iroh_blobs::BlobFormat::HashSeq),
    ] {
        let ticket = iroh_blobs::ticket::BlobTicket::new(
            iroh::EndpointAddr::new(key).with_ip_addr(address),
            iroh_blobs::Hash::new(b"aaaa"),
            format,
        );
        let mut changed = entries.clone();
        changed[0] = rail::record(
            "live-content-locator-v1",
            vec![
                rail::string("0"),
                rail::string(&manifest.chunks[0].chunk_ref),
                rail::string(ticket.to_string()),
            ],
        );
        assert!(admit(&wire(&manifest.value, changed, provider), &manifest.manifest_ref, provider, address).is_err());
    }
    let mut small = profile.clone();
    small.bounds.max_total_bytes = 1;
    assert!(
        admit_live_handoff(
            &small,
            &valid,
            LiveHandoffExpectation {
                manifest_ref: &manifest.manifest_ref,
                provider,
                address
            }
        )
        .is_err()
    );
}

#[test]
fn archive_identity_and_output_are_not_replaceable() {
    let root = Fixture::new();
    let data = b"public fixture bytes";
    let expected = blake3::hash(data).to_hex().to_string();
    assert!(check_archive(data, &expected).is_ok());
    assert!(check_archive(b"changed", &expected).is_err());
    assert!(check_archive(data, "").is_err());
    let output = root.0.join("archive");
    create_output(&output, data).unwrap();
    assert!(create_output(&output, b"replacement").is_err());
    assert_eq!(fs::read(output).unwrap(), data);
}
