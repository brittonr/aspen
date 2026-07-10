
#[cfg(test)]
mod tests {
    use super::*;

    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Ordering = std::sync::atomic::Ordering;
    type PathBuf = std::path::PathBuf;

    fn content_ref_from_bytes(bytes: &[u8]) -> String {
        crate::preserves_rail::content_ref_from_bytes(bytes)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    const CHUNK_SIZE: u64 = 4;
    const RANGE_OFFSET: u64 = 2;
    const RANGE_LENGTH: u64 = 5;
    const MEMBER_SIZE: u64 = 7;
    const FIRST_TEMP_ROOT_ID: u64 = 1;

    static NEXT_TEMP_ROOT_ID: AtomicU64 = AtomicU64::new(FIRST_TEMP_ROOT_ID);

    fn fixture_ref(label: &str) -> String {
        content_ref_from_bytes(label.as_bytes())
    }

    fn temp_root(label: &str) -> PathBuf {
        let id = NEXT_TEMP_ROOT_ID.fetch_add(FIRST_TEMP_ROOT_ID, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("molten-gateway-{label}-{}-{id}", std::process::id()));
        match std::fs::remove_dir_all(&root) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove stale gateway temp root {}: {error}", root.display()),
        }
        root
    }

    fn visibility() -> Visibility {
        Visibility {
            profile: PUBLIC_PROFILE.to_string(),
            visibility_policy_refs: vec![fixture_ref("visibility")],
            retention_refs: vec![fixture_ref("retention")],
            reveal_refs: Vec::new(),
            redaction_refs: vec![fixture_ref("redaction")],
            hidden_refs: Vec::new(),
            allow_sensitive_names: false,
        }
    }

    fn manifest_fixture() -> (PathBuf, ChunkManifest, Map<String, Vec<u8>>) {
        let root = temp_root("range");
        let body = b"abcdefghi";
        let put = crate::chunk_store::put_bytes(&root, "artifact", body, CHUNK_SIZE).expect("put");
        let manifest =
            crate::chunk_store::parse_manifest_value(&put.manifest_value, Some(&put.manifest_ref)).expect("manifest");
        let chunk_size = usize::try_from(CHUNK_SIZE).expect("fixture chunk size fits usize");
        let chunks = manifest
            .chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| {
                let start = index * chunk_size;
                let end = (start + chunk_size).min(body.len());
                (chunk.chunk_ref.clone(), body[start..end].to_vec())
            })
            .collect::<Map<_, _>>();
        (root, manifest, chunks)
    }

    #[test]
    fn readback_decision_normalizes_range_and_requires_chunks_before_io() {
        let (_root, manifest, _chunks) = manifest_fixture();
        let read = decide_readback(&ReadInput {
            object_ref: manifest.manifest_ref.clone(),
            member: None,
            requested_range: Some(Range {
                offset: RANGE_OFFSET,
                length: RANGE_LENGTH,
            }),
            requester_ref: fixture_ref("operator"),
            manifest: Some(&manifest),
            visibility: visibility(),
        })
        .expect("read decision");
        assert_eq!(read.decision, "pass");
        assert_eq!(read.normalized_range.expect("range").length, RANGE_LENGTH);
        assert!(!read.required_chunk_refs.is_empty());
    }

    #[test]
    fn malformed_ref_denies_before_lookup() {
        let read = decide_readback(&ReadInput {
            object_ref: "not-a-ref".to_string(),
            member: None,
            requested_range: None,
            requester_ref: fixture_ref("operator"),
            manifest: None,
            visibility: visibility(),
        })
        .expect("malformed deny");
        assert_eq!(read.decision, "deny");
        assert!(read.diagnostics.iter().any(|diagnostic| diagnostic.contains("invalid object")));
    }

    #[test]
    fn verified_range_returns_bytes_and_denies_corrupt_chunks() {
        let (_root, manifest, chunks) = manifest_fixture();
        let input = RangeVerificationInput {
            read: ReadInput {
                object_ref: manifest.manifest_ref.clone(),
                member: None,
                requested_range: Some(Range {
                    offset: RANGE_OFFSET,
                    length: RANGE_LENGTH,
                }),
                requester_ref: fixture_ref("operator"),
                manifest: Some(&manifest),
                visibility: visibility(),
            },
            chunk_bytes: chunks.clone(),
        };
        let pass = verify_range(&input).expect("range pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(pass.bytes, b"cdefg");

        let mut corrupt = chunks;
        let first = manifest.chunks.first().expect("first chunk").chunk_ref.clone();
        corrupt.insert(first, b"xxxx".to_vec());
        let deny = verify_range(&RangeVerificationInput {
            chunk_bytes: corrupt,
            ..input
        })
        .expect("range deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.bytes.is_empty());
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("corrupt chunk")));
    }

    #[test]
    fn protected_object_denies_without_reveal_evidence() {
        let (_root, mut manifest, chunks) = manifest_fixture();
        manifest.transforms = ChunkTransforms::confidential_protected(fixture_ref("commitment"));
        let deny = verify_range(&RangeVerificationInput {
            read: ReadInput {
                object_ref: manifest.manifest_ref.clone(),
                member: None,
                requested_range: None,
                requester_ref: fixture_ref("operator"),
                manifest: Some(&manifest),
                visibility: visibility(),
            },
            chunk_bytes: chunks,
        })
        .expect("protected deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("protected object")));
    }

    #[test]
    fn index_omits_hidden_and_redacts_sensitive_members() {
        let hidden_ref = fixture_ref("hidden");
        let visible_ref = fixture_ref("visible");
        let decision = decide_index(&IndexInput {
            bundle_ref: fixture_ref("bundle"),
            requester_ref: fixture_ref("operator"),
            visibility: Visibility {
                hidden_refs: vec![hidden_ref.clone()],
                ..visibility()
            },
            members: vec![
                Member {
                    name: "secret-name".to_string(),
                    object_ref: visible_ref,
                    size: MEMBER_SIZE,
                    mime_hint: Some("application/preserves".to_string()),
                    sensitive: true,
                    visible: true,
                },
                Member {
                    name: "hidden".to_string(),
                    object_ref: hidden_ref,
                    size: MEMBER_SIZE,
                    mime_hint: None,
                    sensitive: false,
                    visible: true,
                },
            ],
        })
        .expect("index");
        assert_eq!(decision.decision, "pass");
        assert_eq!(decision.entries.len(), MIN_CHUNK_SIZE);
        assert!(decision.entries[0].redacted);
        let text = to_text(&decision.receipt_value).expect("text");
        assert!(text.contains("hidden-members-omitted"));
    }

    #[test]
    fn receipt_never_authorizes_mutation() {
        let decision = decide_index(&IndexInput {
            bundle_ref: fixture_ref("bundle"),
            requester_ref: fixture_ref("operator"),
            visibility: visibility(),
            members: Vec::new(),
        })
        .expect("index");
        assert!(!receipt_authorizes_mutation(&decision.receipt_value));
    }

    #[test]
    fn http3_iroh_adapter_delegates_to_canonical_gateway_receipt() {
        let (_root, manifest, chunks) = manifest_fixture();
        let decision = handle_http3_iroh_readback(&Http3IrohReadbackInput {
            method: HTTP3_METHOD_GET,
            route: "/artifact/range",
            session_ref: &fixture_ref("http3-session"),
            requester_ref: &fixture_ref("operator"),
            object_ref: &manifest.manifest_ref,
            requested_range: Some(Range {
                offset: RANGE_OFFSET,
                length: RANGE_LENGTH,
            }),
            manifest: Some(&manifest),
            chunk_bytes: chunks,
            visibility: visibility(),
        })
        .expect("http3 adapter pass");
        assert_eq!(decision.decision, "pass");
        assert_eq!(decision.status, http3_status_for_decision("pass"));
        assert!(decision.gateway_receipt_value.is_some());
        let text = to_text(&decision.receipt_value).expect("adapter text");
        assert!(text.contains("http-transport-is-not-authority"));
    }

    #[test]
    fn http3_iroh_adapter_rejects_mutating_methods_and_hidden_refs() {
        let (_root, manifest, chunks) = manifest_fixture();
        let method_deny = handle_http3_iroh_readback(&Http3IrohReadbackInput {
            method: "DELETE",
            route: "/artifact/range",
            session_ref: &fixture_ref("http3-session"),
            requester_ref: &fixture_ref("operator"),
            object_ref: &manifest.manifest_ref,
            requested_range: None,
            manifest: Some(&manifest),
            chunk_bytes: chunks.clone(),
            visibility: visibility(),
        })
        .expect("http3 method deny");
        assert_eq!(method_deny.decision, "deny");
        assert!(method_deny.bytes.is_empty());
        assert!(method_deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("read-only")));

        let hidden_deny = handle_http3_iroh_readback(&Http3IrohReadbackInput {
            method: HTTP3_METHOD_GET,
            route: "/artifact/range",
            session_ref: &fixture_ref("http3-session"),
            requester_ref: &fixture_ref("operator"),
            object_ref: &manifest.manifest_ref,
            requested_range: Some(Range {
                offset: RANGE_OFFSET,
                length: RANGE_LENGTH,
            }),
            manifest: Some(&manifest),
            chunk_bytes: chunks,
            visibility: Visibility {
                hidden_refs: vec![manifest.manifest_ref.clone()],
                ..visibility()
            },
        })
        .expect("http3 hidden deny");
        assert_eq!(hidden_deny.decision, "deny");
        assert!(hidden_deny.bytes.is_empty());
    }
}
