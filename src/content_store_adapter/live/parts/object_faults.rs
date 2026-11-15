#[cfg(test)]
mod fault_tests {
    use std::time::Duration;

    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::content_store_adapter::ClientConfig;
    use crate::content_store_adapter::LocalTier;

    fn reference(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn profile() -> ContentAdapterProfile {
        crate::content_store_adapter::profile(
            "object",
            reference("profile"),
            ContentAdapterClass::ObjectStorage,
            ContentResourceBounds {
                max_total_bytes: 1024,
                max_chunk_count: 8,
                max_chunk_bytes: 1024,
                max_range_bytes: 1024,
                max_concurrent_operations: 1,
                max_queued_bytes: 1024,
                max_memory_bytes: 1024,
                max_deadline_ticks: 10,
                max_retries: 0,
                max_events: 16,
                max_status_entries: 8,
            },
            vec![reference("evidence")],
        )
        .expect("profile")
    }

    async fn fetch_with(
        response: &'static [u8],
        delay: Option<Duration>,
        timeout: Option<Duration>,
    ) -> ObjectExecution {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 2048];
            let _ = socket.read(&mut buffer).await.expect("read");
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            let _ = socket.write_all(response).await;
        });
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let config = ClientConfig {
            endpoint: format!("http://{address}"),
            region: "us-east-1".to_string(),
            bucket: "bucket".to_string(),
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
            session_token: None,
        };
        let mut client = match timeout {
            Some(timeout) => CachedClient::connect_with_timeout(config, tier, timeout),
            None => CachedClient::connect(config, tier),
        }
        .expect("client");
        let profile = profile();
        let chunk_ref = crate::chunk_store::hash_chunk(b"hello", 5);
        let manifest = ContentManifestDescriptor {
            manifest_ref: reference("manifest"),
            total_length: 5,
            chunker: "fixed_v1".to_string(),
            chunk_size: 5,
            metadata_ref: reference("metadata"),
            policy_refs: vec![reference("policy")],
            evidence_refs: vec![reference("evidence")],
            chunks: vec![ContentChunkDescriptor {
                chunk_ref: chunk_ref.clone(),
                length: 5,
                position: 0,
                transform: "identity".to_string(),
            }],
        };
        let command = crate::content_store_adapter::command(&profile, crate::content_store_adapter::CommandInput {
            operation_ref: reference("operation"),
            operation: ContentOperation::Get,
            manifest: &manifest,
            range: None,
            submitted_tick: 0,
            deadline_tick: 10,
            retry_count: 0,
            cancelled: false,
            policy_refs: vec![reference("policy")],
        })
        .expect("command");
        let publication = ObjectPublication {
            manifest,
            keys: vec![format!(
                "objects/{}",
                crate::preserves_rail::content_ref_from_bytes(b"hello").trim_start_matches("blake3:")
            )],
            backend_hint_ref: reference("backend"),
        };
        let execution = fetch_object_chunks(
            FetchInput {
                profile: &profile,
                publication: &publication,
                command: &command,
                generation: 1,
                retained: None,
            },
            &mut client,
        )
        .await
        .expect("fetch");
        if execution.state.artifact.terminal != ContentTerminal::Verified {
            assert_eq!(client.local_tier_bytes(), 0, "failed observations must not enter the tier");
        }
        let fixture_value = crate::preserves_rail::record("content-store-mock-get-command-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(response)),
            crate::preserves_rail::u64_value(delay.map_or(0, |value| value.as_millis() as u64)),
            crate::preserves_rail::u64_value(timeout.map_or(30_000, |value| value.as_millis() as u64)),
            crate::preserves_rail::string(crate::content_store_adapter::bind_command(&profile, &publication.manifest, &command, 0, 0).expect("command digest").artifact_ref),
        ]);
        let value = crate::preserves_rail::record("content-store-mock-get-parity-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&fixture_value).expect("fixture digest")),
            crate::preserves_rail::string(&execution.state.artifact_ref),
            crate::preserves_rail::string(execution.state.artifact.terminal.as_str()),
            crate::preserves_rail::u64_value(client.local_tier_bytes()),
            crate::preserves_rail::string("http-mock-only-no-performance-claim"),
        ]);
        let class = execution.state.artifact.failure.map_or("verified", ContentFailure::as_str);
        super::completion_tests::retain_receipt(&format!("mock-get-{class}.preserves"), &value);
        execution
    }

    #[tokio::test]
    async fn disconnect_fails_without_verified_payload() {
        let execution = fetch_with(b"", None, None).await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Retryable);
        assert_eq!(execution.state.artifact.failure, Some(ContentFailure::TransportDisconnected));
        assert!(execution.verified_chunks.is_empty());
    }

    #[tokio::test]
    async fn truncation_fails_without_verified_payload() {
        let execution = fetch_with(
            b"HTTP/1.1 200 OK\r\ncontent-length: 10\r\nconnection: close\r\n\r\nhi",
            None,
            None,
        )
        .await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Failed);
        assert_eq!(execution.state.artifact.failure, Some(ContentFailure::TruncatedChunk));
        assert!(execution.verified_chunks.is_empty());
    }

    #[tokio::test]
    async fn timeout_fails_without_verified_payload() {
        let execution = fetch_with(b"", Some(Duration::from_millis(200)), Some(Duration::from_millis(50))).await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Uncertain);
        assert_eq!(execution.state.artifact.failure, Some(ContentFailure::Timeout));
        assert!(execution.verified_chunks.is_empty());
    }

    #[tokio::test]
    async fn provider_throttle_remains_retryable_without_cached_bytes() {
        let execution = fetch_with(b"HTTP/1.1 503 SlowDown\r\ncontent-length: 0\r\nconnection: close\r\n\r\n", None, None).await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Retryable);
        assert_eq!(execution.state.artifact.failure, Some(ContentFailure::ProviderThrottle));
        assert!(execution.verified_chunks.is_empty());
    }

    #[tokio::test]
    async fn missing_object_is_never_available_or_cached() {
        let execution = fetch_with(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n", None, None).await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Failed);
        assert!(execution.verified_chunks.is_empty());
    }

    #[tokio::test]
    async fn corruption_never_enters_cache_or_verified_payload() {
        let execution = fetch_with(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nwrong", None, None).await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Failed);
        assert_eq!(execution.state.artifact.failure, Some(ContentFailure::CorruptChunk));
        assert!(execution.verified_chunks.is_empty());
    }

    #[tokio::test]
    async fn bounded_latency_does_not_change_verified_identity() {
        let execution = fetch_with(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello",
            Some(Duration::from_millis(1)), None).await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Verified);
        assert_eq!(execution.verified_chunks[0].bytes, b"hello");
    }
}
