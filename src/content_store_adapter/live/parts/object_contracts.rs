#[cfg(test)]
mod completion_tests {
    use super::*;
    use crate::content_store_adapter::ClientConfig;
    use crate::content_store_adapter::LocalTier;
    use crate::content_store_adapter::bind_command;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    pub(super) fn retain_receipt(name: &str, value: &preserves::IOValue) {
        if let Ok(root) = std::env::var("MOLTEN_CONTENT_RECEIPT_DIR") {
            let namespace = crate::node_state::Scope::open_existing(crate::node_state::NamespaceKind::Receipts, std::path::Path::new(&root))
                .expect("explicit receipt authority");
            namespace.write(&crate::node_state::RelativePath::parse(name).expect("receipt locator"),
                &crate::preserves_rail::canonical_bytes(value).expect("canonical receipt")).expect("retain actual observation");
        }
    }

    fn reference(label: &str) -> String { crate::preserves_rail::content_ref_from_bytes(label.as_bytes()) }

    fn fixture(operation: ContentOperation) -> (ContentAdapterProfile, ObjectPublication, ContentCommand) {
        let profile = crate::content_store_adapter::profile("object", reference("profile"), ContentAdapterClass::ObjectStorage,
            ContentResourceBounds { max_total_bytes: 1024, max_chunk_count: 8, max_chunk_bytes: 1024,
                max_range_bytes: 1024, max_concurrent_operations: 1, max_queued_bytes: 1024, max_memory_bytes: 1024,
                max_deadline_ticks: 10, max_retries: 0, max_events: 16, max_status_entries: 8 }, vec![reference("evidence")]
        ).expect("profile");
        let publication = ObjectPublication {
            manifest: ContentManifestDescriptor {
                manifest_ref: reference("manifest"), total_length: 5, chunker: "fixed_v1".to_string(), chunk_size: 5,
                metadata_ref: reference("metadata"), policy_refs: vec![reference("policy")], evidence_refs: vec![reference("evidence")],
                chunks: vec![ContentChunkDescriptor { chunk_ref: crate::chunk_store::hash_chunk(b"hello", 5), length: 5,
                    position: 0, transform: "identity".to_string() }],
            },
            keys: vec![format!("objects/{}", &reference("hello")[7..])], backend_hint_ref: reference("backend"),
        };
        let command = crate::content_store_adapter::command(&profile, crate::content_store_adapter::CommandInput {
            operation_ref: reference("operation"), operation, manifest: &publication.manifest, range: None,
            submitted_tick: 0, deadline_tick: 10, retry_count: 0, cancelled: false, policy_refs: vec![reference("policy")],
        }).expect("command");
        (profile, publication, command)
    }

    fn client(endpoint: String, tier: LocalTier) -> CachedClient {
        CachedClient::connect(ClientConfig { endpoint, region: "us-east-1".to_string(), bucket: "bucket".to_string(),
            access_key: "access".to_string(), secret_key: "secret".to_string(), session_token: None }, tier).expect("client")
    }

    #[tokio::test]
    async fn command_local_hit_makes_no_remote_request() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let directory = tempfile::tempdir().expect("tier");
        let mut client = client(format!("http://{}", listener.local_addr().expect("address")),
            LocalTier::open(directory.path(), 5, false).expect("tier"));
        client.cache_verified(b"hello").expect("cache");
        let (profile, publication, command) = fixture(ContentOperation::Get);
        let execution = fetch_object_chunks(FetchInput { profile: &profile, publication: &publication, command: &command,
            generation: 1, retained: None }, &mut client).await.expect("local hit");
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Verified);
        assert_eq!(execution.verified_chunks[0].bytes, b"hello");
        assert_eq!(client.local_tier_bytes(), 5);
        assert!(tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept()).await.is_err());
    }

    #[tokio::test]
    async fn put_faults_use_shared_terminal_classifier() {
        for (response, expected) in [
            (&b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"[..], ContentTerminal::Durable),
            (&b"HTTP/1.1 503 SlowDown\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"[..], ContentTerminal::Retryable),
            (&b""[..], ContentTerminal::Uncertain),
            (&b"HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"[..], ContentTerminal::Failed),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
            let endpoint = format!("http://{}", listener.local_addr().expect("address"));
            let response = response.to_vec();
            let fixture_ref = crate::preserves_rail::content_ref_from_bytes(&response);
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.expect("accept");
                let mut buffer = [0; 4096];
                let _ = socket.read(&mut buffer).await.expect("read");
                socket.write_all(&response).await.expect("response");
            });
            let directory = tempfile::tempdir().expect("tier");
            let mut client = client(endpoint, LocalTier::open(directory.path(), 5, false).expect("tier"));
            let (profile, publication, command) = fixture(ContentOperation::Put);
            let result = execute_object_put(FetchInput { profile: &profile, publication: &publication, command: &command,
                generation: 1, retained: None }, &[b"hello".to_vec()], &mut client).await.expect("put outcome");
            assert_eq!(result.state.artifact.terminal, expected);
            assert!(result.verified_chunks.is_empty());
            if expected != ContentTerminal::Durable { assert_eq!(client.local_tier_bytes(), 0); }
            let value = crate::preserves_rail::record("content-store-mock-put-parity-v1", vec![
                crate::preserves_rail::string(&fixture_ref),
                crate::preserves_rail::string(bind_command(&profile, &publication.manifest, &command, 0, 0).expect("command digest").artifact_ref),
                crate::preserves_rail::string(&result.state.artifact_ref),
                crate::preserves_rail::string(result.state.artifact.terminal.as_str()),
                crate::preserves_rail::u64_value(client.local_tier_bytes()),
                crate::preserves_rail::string("http-mock-not-live-provider-or-multipart-wire-evidence"),
            ]);
            retain_receipt(&format!("mock-put-{}.preserves", result.state.artifact.terminal.as_str()), &value);
            server.await.expect("server");
        }
    }

    #[tokio::test]
    async fn put_timeout_remains_uncertain_without_local_growth() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let endpoint = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0; 4096];
            let _ = socket.read(&mut buffer).await.expect("read put");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            drop(socket);
        });
        let directory = tempfile::tempdir().expect("tier");
        let mut client = CachedClient::connect_with_timeout(ClientConfig {
            endpoint, region: "us-east-1".to_string(), bucket: "bucket".to_string(),
            access_key: "access".to_string(), secret_key: "secret".to_string(), session_token: None,
        }, LocalTier::open(directory.path(), 5, false).expect("tier"), std::time::Duration::from_millis(20)).expect("client");
        let (profile, publication, command) = fixture(ContentOperation::Put);
        let result = execute_object_put(FetchInput { profile: &profile, publication: &publication, command: &command,
            generation: 1, retained: None }, &[b"hello".to_vec()], &mut client).await.expect("put outcome");
        assert_eq!(result.state.artifact.terminal, ContentTerminal::Uncertain);
        assert_eq!(result.state.artifact.failure, Some(ContentFailure::Timeout));
        assert!(result.verified_chunks.is_empty());
        assert_eq!(client.local_tier_bytes(), 0);
        let value = crate::preserves_rail::record("content-store-mock-put-timeout-v1", vec![
            crate::preserves_rail::string(&result.state.artifact_ref),
            crate::preserves_rail::string(bind_command(&profile, &publication.manifest, &command, 0, 0).expect("command digest").artifact_ref),
            crate::preserves_rail::u64_value(20), crate::preserves_rail::string("timeout-uncertain-no-implicit-retry"),
        ]);
        retain_receipt("mock-put-timeout.preserves", &value);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn remote_lifecycle_loss_of_protected_chunk_fails_even_with_local_copy() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let endpoint = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0; 4096];
            let _ = socket.read(&mut buffer).await.expect("request");
            socket.write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n").await.expect("missing");
        });
        let directory = tempfile::tempdir().expect("tier");
        let mut client = client(endpoint, LocalTier::open(directory.path(), 5, false).expect("tier"));
        client.cache_verified(b"hello").expect("cache");
        let (_, publication, _) = fixture(ContentOperation::Get);
        let chunk_ref = &publication.manifest.chunks[0].chunk_ref;
        let mut retention = Retention::default(); retention.protect(chunk_ref);
        assert!(audit_protected_object(&client, &retention, &publication.keys[0], chunk_ref, 5).await.is_err());
        assert!(client.contains(&reference("hello")).expect("local copy exists"));
        server.await.expect("server");
    }

    #[tokio::test]
    #[ignore = "requires pinned local MOLTEN_S3_ENDPOINT, MOLTEN_S3_BUCKET and MOLTEN_CONTENT_RECEIPT_DIR"]
    async fn live_command_conformance_emits_canonical_receipt() {
        let endpoint = std::env::var("MOLTEN_S3_ENDPOINT").expect("local endpoint");
        let bucket = std::env::var("MOLTEN_S3_BUCKET").expect("bucket");
        assert_eq!(endpoint, "http://127.0.0.1:28433", "destructive fault fixture requires the pinned disposable endpoint");
        assert_eq!(bucket, "molten-live", "destructive fault fixture requires its disposable bucket");
        let directory = tempfile::tempdir().expect("tier");
        let scope = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Storage, directory.path()).expect("scope");
        let budget = ContentTierRecoveryBudget { max_local_tier_bytes: 5, minimum_verified_bytes_per_second: 5, max_rebuild_seconds: 1 };
        let config = ClientConfig { endpoint, region: "us-east-1".to_string(), bucket,
            access_key: "molten".to_string(), secret_key: "molten".to_string(), session_token: None };
        let fault_client = crate::content_store_adapter::Client::connect(config.clone()).expect("disposable fault injector");
        let mut client = CachedClient::connect(config,
            LocalTier::from_recovery_scope(scope, budget).expect("recovery budget")).expect("client");
        let (profile, publication, put_command) = fixture(ContentOperation::Put);
        let put = execute_object_put(FetchInput { profile: &profile, publication: &publication, command: &put_command,
            generation: 1, retained: None }, &[b"hello".to_vec()], &mut client).await.expect("put");
        assert_eq!(put.state.artifact.terminal, ContentTerminal::Durable);
        assert!(put.verified_chunks.is_empty());
        assert_eq!(client.head(&publication.keys[0]).await.expect("availability"), Some(5));
        client.evict_local(&reference("hello")).expect("cold get");
        let (_, _, get_command) = fixture(ContentOperation::Get);
        let get = fetch_object_chunks(FetchInput { profile: &profile, publication: &publication, command: &get_command,
            generation: 1, retained: None }, &mut client).await.expect("get");
        assert_eq!(get.state.artifact.terminal, ContentTerminal::Verified);
        assert_eq!(get.verified_chunks[0].bytes, b"hello");
        let mut recorder = crate::content_store_adapter::LatencyRecorder::default();
        for _ in 0..128 {
            client.evict_local(&reference("hello")).expect("force remote sample");
            let remote = client.get_timed(&publication.keys[0], &reference("hello")).await.expect("remote read");
            assert_eq!(remote.source, crate::content_store_adapter::ReadSource::Remote);
            recorder.record(&remote).expect("remote sample");
            let local = client.get_timed(&publication.keys[0], &reference("hello")).await.expect("local read");
            assert_eq!(local.source, crate::content_store_adapter::ReadSource::Local);
            recorder.record(&local).expect("local sample");
        }
        let summary = recorder.summary();
        assert_eq!((summary.remote_count, summary.local_count), (128, 128));
        let mut retention = Retention::default();
        let chunk_ref = &publication.manifest.chunks[0].chunk_ref;
        retention.protect(chunk_ref);
        assert!(delete_chunk(&mut client, &retention, &publication.keys[0], chunk_ref, None).await.is_err());
        audit_protected_object(&client, &retention, &publication.keys[0], chunk_ref, 5).await.expect("protected remote presence");
        // Deliberate out-of-band lifecycle fault against the exact disposable
        // endpoint, not a synthesized canonical deletion admission.
        fault_client.delete(&publication.keys[0]).await.expect("inject remote lifecycle loss");
        assert!(client.contains(&reference("hello")).expect("valid local copy remains"));
        assert!(audit_protected_object(&client, &retention, &publication.keys[0], chunk_ref, 5).await.is_err(),
            "a local hit must not conceal protected remote loss");
        retention.unprotect(chunk_ref);
        assert!(delete_chunk(&mut client, &retention, &publication.keys[0], chunk_ref, None).await.is_err(), "unprotect is not deletion authority");
        let value = crate::preserves_rail::record("content-store-object-conformance-v1", vec![
            crate::preserves_rail::string(&put.state.artifact_ref), crate::preserves_rail::string(&get.state.artifact_ref),
            crate::preserves_rail::string(bind_command(&profile, &publication.manifest, &put_command, 0, 0).expect("put digest").artifact_ref),
            crate::preserves_rail::string(bind_command(&profile, &publication.manifest, &get_command, 0, 0).expect("get digest").artifact_ref),
            crate::preserves_rail::u64_value(summary.remote_count as u64), crate::preserves_rail::u64_value(summary.local_count as u64),
            crate::preserves_rail::u64_value(summary.remote_p50_micros), crate::preserves_rail::u64_value(summary.remote_p99_micros),
            crate::preserves_rail::u64_value(summary.local_p50_micros), crate::preserves_rail::u64_value(summary.local_p99_micros),
            crate::preserves_rail::u64_value(client.local_tier_bytes()), crate::preserves_rail::u64_value(client.local_tier_capacity()),
            crate::preserves_rail::u64_value(budget.minimum_verified_bytes_per_second),
            crate::preserves_rail::u64_value(budget.max_rebuild_seconds),
            crate::preserves_rail::string(client.remote_policy().endpoint_resolution),
            crate::preserves_rail::string(client.remote_policy().credential_refresh),
            crate::preserves_rail::string("protected-remote-lifecycle-loss-observed-with-local-copy-retained"),
            crate::preserves_rail::string("local-s3-fixture-only-no-speedup-or-production-durability-claim"),
        ]);
        let root = std::env::var("MOLTEN_CONTENT_RECEIPT_DIR").expect("receipt directory");
        let namespace = crate::node_state::Scope::open_existing(crate::node_state::NamespaceKind::Receipts, std::path::Path::new(&root)).expect("receipt scope");
        namespace.write(&crate::node_state::RelativePath::parse("object-conformance.preserves").expect("locator"),
            &crate::preserves_rail::canonical_bytes(&value).expect("canonical receipt")).expect("persist actual receipt");
    }
}
