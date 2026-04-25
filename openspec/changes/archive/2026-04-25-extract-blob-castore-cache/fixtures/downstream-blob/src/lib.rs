#[cfg(test)]
mod tests {
    use aspen_blob::BlobRead;
    use aspen_blob::BlobWrite;
    use aspen_blob::InMemoryBlobStore;
    use aspen_blob::is_blob_ref;

    const FIXTURE_BYTES: &[u8] = b"downstream blob fixture payload";

    #[tokio::test]
    async fn canonical_memory_store_roundtrips_bytes() {
        let store = InMemoryBlobStore::new();

        let add_result = store.add_bytes(FIXTURE_BYTES).await.expect("add fixture blob");
        let hash = add_result.blob_ref.hash;
        let loaded = store.get_bytes(&hash).await.expect("load fixture blob");

        assert!(add_result.was_new);
        assert_eq!(loaded.as_deref(), Some(FIXTURE_BYTES));
        assert!(store.has(&hash).await.expect("check fixture blob existence"));
    }

    #[test]
    fn blob_ref_prefix_helper_rejects_plain_hash() {
        let plain_hash = "not-a-blob-ref";

        assert!(!is_blob_ref(plain_hash));
    }
}
