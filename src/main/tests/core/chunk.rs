    #[test]
    fn cli_chunk_store_commands_work() {
        let dir = temp_dir("chunk-cli");
        let fixture = create_chunk_manifest(&dir);
        read_chunk_ranges(&dir, &fixture);
        mirror_chunk_store(&dir, &fixture);
        exchange_chunks_locally(&dir, &fixture);
        inspect_chunk_index(&dir, &fixture);
        pin_and_collect_chunks(&dir, fixture);
    }

    struct ChunkFixture {
        store: PathBuf,
        manifest_ref: String,
    }

    fn create_chunk_manifest(dir: &Path) -> ChunkFixture {
        let input = dir.join("input.bin");
        let store = dir.join("chunk-store");
        let manifest = dir.join("manifest.preserves");
        fs::write(&input, b"aaaabbbbcccc").expect("write input");
        run(Top::Put {
            input,
            store: store.clone(),
            kind: "artifact".to_string(),
            chunk_size: 4,
            manifest_out: Some(manifest.clone()),
            receipt_out: Some(dir.join("put-receipt.preserves")),
        })
        .expect("chunk put");
        let manifest_value = read_preserves_file(&manifest).expect("read manifest");
        let manifest_ref = molten::preserves_rail::canonical_hash(&manifest_value).expect("manifest ref");
        run(Top::Verify {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            receipt_out: Some(dir.join("verify-receipt.preserves")),
        })
        .expect("chunk verify");
        ChunkFixture { store, manifest_ref }
    }

    fn read_chunk_ranges(dir: &Path, fixture: &ChunkFixture) {
        let full = dir.join("full.bin");
        let range = dir.join("range.bin");
        run(Top::Read {
            manifest_ref: fixture.manifest_ref.clone(),
            store: fixture.store.clone(),
            out: full.clone(),
            receipt_out: Some(dir.join("read-receipt.preserves")),
        })
        .expect("chunk read");
        assert_eq!(fs::read(&full).expect("read full"), b"aaaabbbbcccc");
        run(Top::Range {
            manifest_ref: fixture.manifest_ref.clone(),
            store: fixture.store.clone(),
            offset: 2,
            length: 8,
            out: range.clone(),
            receipt_out: Some(dir.join("range-receipt.preserves")),
        })
        .expect("chunk range");
        assert_eq!(fs::read(&range).expect("read range"), b"aabbbbcc");
    }

    fn mirror_chunk_store(dir: &Path, fixture: &ChunkFixture) {
        let mirror = dir.join("chunk-store-mirror");
        run(Top::Sync {
            manifest_ref: fixture.manifest_ref.clone(),
            from: fixture.store.clone(),
            store: mirror.clone(),
            receipt_out: Some(dir.join("sync-receipt.preserves")),
        })
        .expect("chunk sync");
        run(Top::Read {
            manifest_ref: fixture.manifest_ref.clone(),
            store: mirror,
            out: dir.join("mirror-full.bin"),
            receipt_out: None,
        })
        .expect("read synced chunk store");
    }

    fn exchange_chunks_locally(dir: &Path, fixture: &ChunkFixture) {
        let iroh_store = dir.join("chunk-iroh-store");
        run(Top::IrohPublish {
            manifest_ref: fixture.manifest_ref.clone(),
            store: fixture.store.clone(),
            iroh_store: iroh_store.clone(),
            node: "node:cli".to_string(),
            receipt_out: Some(dir.join("iroh-publish-receipt.preserves")),
        })
        .expect("chunk iroh publish");
        let iroh_dest = dir.join("chunk-iroh-dest");
        run(Top::IrohFetch {
            ticket: format!("iroh-local-chunk:{}", fixture.manifest_ref),
            iroh_store,
            store: iroh_dest.clone(),
            expected_manifest_ref: Some(fixture.manifest_ref.clone()),
            peer: "peer:cli".to_string(),
            receipt_out: Some(dir.join("iroh-fetch-receipt.preserves")),
        })
        .expect("chunk iroh fetch");
        run(Top::Read {
            manifest_ref: fixture.manifest_ref.clone(),
            store: iroh_dest,
            out: dir.join("iroh-full.bin"),
            receipt_out: None,
        })
        .expect("read iroh-fetched chunk store");
    }

    fn inspect_chunk_index(dir: &Path, fixture: &ChunkFixture) {
        run(Top::IndexStatus {
            store: fixture.store.clone(),
        })
        .expect("chunk index status");
        run(Top::IndexRebuild {
            store: fixture.store.clone(),
            receipt_out: Some(dir.join("index-rebuild-receipt.preserves")),
        })
        .expect("chunk index rebuild");
        run(Top::ReceiptList {
            store: fixture.store.clone(),
        })
        .expect("chunk receipt list");
        let receipt_ref = molten::chunk_store::list_receipt_refs(&fixture.store)
            .expect("list receipt refs")
            .into_iter()
            .next()
            .expect("receipt ref");
        run(Top::ReceiptShow {
            receipt_ref,
            store: fixture.store.clone(),
        })
        .expect("chunk receipt show");
        let lineage_out = dir.join("chunk-lineage.preserves");
        run(Top::Lineage {
            manifest_ref: fixture.manifest_ref.clone(),
            store: fixture.store.clone(),
            lineage_out: Some(lineage_out.clone()),
        })
        .expect("chunk lineage");
        assert!(fs::read_to_string(lineage_out).expect("read lineage").contains("chunk-lineage-v1"));
    }

    fn pin_and_collect_chunks(dir: &Path, fixture: ChunkFixture) {
        run(Top::Pin {
            manifest_ref: fixture.manifest_ref.clone(),
            store: fixture.store.clone(),
            receipt_out: Some(dir.join("pin-receipt.preserves")),
        })
        .expect("chunk pin");
        run(Top::Unpin {
            manifest_ref: fixture.manifest_ref,
            store: fixture.store.clone(),
            receipt_out: Some(dir.join("unpin-receipt.preserves")),
        })
        .expect("chunk unpin");
        run(Top::Gc {
            store: fixture.store,
            dry_run: false,
            apply_refs: Vec::new(),
            retention: retention_cli_args("chunk-gc"),
            receipt_out: Some(dir.join("gc-receipt.preserves")),
        })
        .expect("chunk gc");
    }
