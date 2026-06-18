    #[test]
    fn cli_chunk_store_commands_work() {
        let dir = temp_dir("chunk-cli");
        let input = dir.join("input.bin");
        let store = dir.join("chunk-store");
        let manifest = dir.join("manifest.preserves");
        let full = dir.join("full.bin");
        let range = dir.join("range.bin");
        fs::write(&input, b"aaaabbbbcccc").expect("write input");
        run_chunk_command(ChunkCommand::Put {
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
        run_chunk_command(ChunkCommand::Verify {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            receipt_out: Some(dir.join("verify-receipt.preserves")),
        })
        .expect("chunk verify");
        run_chunk_command(ChunkCommand::Read {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            out: full.clone(),
            receipt_out: Some(dir.join("read-receipt.preserves")),
        })
        .expect("chunk read");
        assert_eq!(fs::read(&full).expect("read full"), b"aaaabbbbcccc");
        run_chunk_command(ChunkCommand::Range {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            offset: 2,
            length: 8,
            out: range.clone(),
            receipt_out: Some(dir.join("range-receipt.preserves")),
        })
        .expect("chunk range");
        assert_eq!(fs::read(&range).expect("read range"), b"aabbbbcc");
        let mirror = dir.join("chunk-store-mirror");
        run_chunk_command(ChunkCommand::Sync {
            manifest_ref: manifest_ref.clone(),
            from: store.clone(),
            store: mirror.clone(),
            receipt_out: Some(dir.join("sync-receipt.preserves")),
        })
        .expect("chunk sync");
        run_chunk_command(ChunkCommand::Read {
            manifest_ref: manifest_ref.clone(),
            store: mirror,
            out: dir.join("mirror-full.bin"),
            receipt_out: None,
        })
        .expect("read synced chunk store");
        let iroh_store = dir.join("chunk-iroh-store");
        run_chunk_command(ChunkCommand::IrohPublish {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            iroh_store: iroh_store.clone(),
            node: "node:cli".to_string(),
            receipt_out: Some(dir.join("iroh-publish-receipt.preserves")),
        })
        .expect("chunk iroh publish");
        let iroh_dest = dir.join("chunk-iroh-dest");
        run_chunk_command(ChunkCommand::IrohFetch {
            ticket: format!("iroh-local-chunk:{manifest_ref}"),
            iroh_store: iroh_store.clone(),
            store: iroh_dest.clone(),
            expected_manifest_ref: Some(manifest_ref.clone()),
            peer: "peer:cli".to_string(),
            receipt_out: Some(dir.join("iroh-fetch-receipt.preserves")),
        })
        .expect("chunk iroh fetch");
        run_chunk_command(ChunkCommand::Read {
            manifest_ref: manifest_ref.clone(),
            store: iroh_dest,
            out: dir.join("iroh-full.bin"),
            receipt_out: None,
        })
        .expect("read iroh-fetched chunk store");
        run_chunk_command(ChunkCommand::IndexStatus { store: store.clone() }).expect("chunk index status");
        run_chunk_command(ChunkCommand::IndexRebuild {
            store: store.clone(),
            receipt_out: Some(dir.join("index-rebuild-receipt.preserves")),
        })
        .expect("chunk index rebuild");
        run_chunk_command(ChunkCommand::ReceiptList { store: store.clone() }).expect("chunk receipt list");
        let receipt_ref = chunk_store::list_receipt_refs(&store)
            .expect("list receipt refs")
            .into_iter()
            .next()
            .expect("receipt ref");
        run_chunk_command(ChunkCommand::ReceiptShow {
            receipt_ref,
            store: store.clone(),
        })
        .expect("chunk receipt show");
        let lineage_out = dir.join("chunk-lineage.preserves");
        run_chunk_command(ChunkCommand::Lineage {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            lineage_out: Some(lineage_out.clone()),
        })
        .expect("chunk lineage");
        assert!(fs::read_to_string(lineage_out).expect("read lineage").contains("chunk-lineage-v1"));
        run_chunk_command(ChunkCommand::Pin {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            receipt_out: Some(dir.join("pin-receipt.preserves")),
        })
        .expect("chunk pin");
        run_chunk_command(ChunkCommand::Unpin {
            manifest_ref,
            store: store.clone(),
            receipt_out: Some(dir.join("unpin-receipt.preserves")),
        })
        .expect("chunk unpin");
        run_chunk_command(ChunkCommand::Gc {
            store,
            dry_run: false,
            apply_refs: Vec::new(),
            retention: retention_cli_args("chunk-gc"),
            receipt_out: Some(dir.join("gc-receipt.preserves")),
        })
        .expect("chunk gc");
    }
