use molten_core::content_store_adapter::*;

use crate::content_store_adapter::Artifact;
use crate::content_store_adapter::VerifiedChunkPayload;
use crate::content_store_adapter::manifest_descriptor;

use super::CachedClient;
use super::EventInput;
use super::bind_event;
use super::event;
use super::failure_from_issues;
use super::partial_state;
use super::UNACKNOWLEDGED_PUT;

#[derive(Debug, Clone)]
pub struct ObjectPublication {
    pub manifest: ContentManifestDescriptor,
    pub keys: Vec<String>,
    pub backend_hint_ref: String,
}

#[derive(Debug, Clone)]
pub struct ObjectExecution {
    pub state: Artifact<ContentPartialState>,
    pub events: Vec<Artifact<ContentEvent>>,
    pub verified_chunks: Vec<VerifiedChunkPayload>,
    pub backend_hint_ref: String,
}

/// Publishes every verified chunk under one locator key and returns the ordered key set.
pub async fn publish_object_chunks(
    profile: &ContentAdapterProfile,
    root: &crate::chunk_store::CapabilityChunkRoot,
    manifest_ref: &str,
    client: &mut CachedClient,
    key_prefix: &str,
) -> crate::error::Result<ObjectPublication> {
    if profile.class != ContentAdapterClass::ObjectStorage {
        return Err(crate::error::Failure::invalid_harness(
            "object-storage publication requires an object-storage adapter profile",
        ));
    }
    validate_key_prefix(key_prefix)?;
    let issues = validate_content_profile(profile);
    if !issues.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "object-storage profile denied: {issues:?}"
        )));
    }
    let source_manifest = crate::chunk_store::read_manifest_with_root(root, manifest_ref)?;
    let manifest = manifest_descriptor(&source_manifest);
    let issues = validate_manifest_descriptor(&manifest);
    if !issues.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "object-storage manifest denied: {issues:?}"
        )));
    }
    if manifest.total_length > profile.bounds.max_total_bytes || manifest.chunks.len() > profile.bounds.max_chunk_count
    {
        return Err(crate::error::Failure::invalid_harness(
            "object-storage publication exceeds adapter bounds",
        ));
    }
    let chunk_size = usize::try_from(source_manifest.chunk_size).map_err(|_| {
        crate::error::Failure::invalid_harness("object-storage chunk size does not fit usize")
    })?;
    let mut keys = Vec::with_capacity(source_manifest.chunks.len());
    for chunk in &source_manifest.chunks {
        let bytes = crate::chunk_store::read_verified_chunk(root, chunk, chunk_size)?;
        let object_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        let key = format!("{key_prefix}/{}", object_ref.trim_start_matches("blake3:"));
        client.put(&key, &bytes).await?;
        keys.push(key);
    }
    let backend_hint_ref = backend_hint_ref(ContentAdapterClass::ObjectStorage, key_prefix)?;
    Ok(ObjectPublication { manifest, keys, backend_hint_ref })
}

pub struct FetchInput<'a> {
    pub profile: &'a ContentAdapterProfile,
    pub publication: &'a ObjectPublication,
    pub command: &'a ContentCommand,
    pub generation: u64,
    pub retained: Option<&'a ContentPartialState>,
}

pub async fn fetch_object_chunks(
    input: FetchInput<'_>,
    client: &mut CachedClient,
) -> crate::error::Result<ObjectExecution> {
    let FetchInput { profile, publication, command, generation, retained } = input;
    if profile.class != ContentAdapterClass::ObjectStorage {
        return Err(crate::error::Failure::invalid_harness(
            "object-storage fetch requires an object-storage adapter profile",
        ));
    }
    let preflight = preflight_content_operation(profile, &publication.manifest, command, 0, 0);
    if preflight.terminal != ContentTerminal::Accepted || !preflight.issues.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "object-storage preflight denied: {:?}",
            preflight.issues
        )));
    }
    let mut state = begin_partial_state(profile, &publication.manifest, command, generation, retained)
        .map_err(|issues| crate::error::Failure::invalid_harness(format!(
            "object-storage partial state denied: {issues:?}"
        )))?;
    let mut events = Vec::with_capacity(publication.keys.len());
    let mut verified_chunks = Vec::with_capacity(publication.keys.len());
    for (position, key) in publication.keys.iter().enumerate() {
        let Some(descriptor) = publication.manifest.chunks.get(position) else {
            return terminate(profile, publication, command, state, events, verified_chunks);
        };
        let bytes = match client.get_raw(key).await {
            Ok(bytes) => bytes,
            Err(_) => {
                state = classify_content_failure(profile, &state, ContentFailure::AdapterFailure, UNACKNOWLEDGED_PUT)
                    .map_err(transition_error)?;
                return terminate(profile, publication, command, state, events, verified_chunks);
            }
        };
        let chunk_size = usize::try_from(publication.manifest.chunk_size)
            .map_err(|_| crate::error::Failure::invalid_harness("object-storage chunk size does not fit usize"))?;
        if crate::chunk_store::hash_chunk(&bytes, chunk_size) != descriptor.chunk_ref {
            state = classify_content_failure(
                profile,
                &state,
                ContentFailure::CorruptChunk,
                UNACKNOWLEDGED_PUT,
            )
            .map_err(transition_error)?;
            return terminate(profile, publication, command, state, events, verified_chunks);
        }
        client.cache_verified(&bytes)?;
        let observed_length = u64::try_from(bytes.len())
            .map_err(|_| crate::error::Failure::invalid_harness("object-storage chunk length does not fit u64"))?;
        let observation = ContentChunkObservation {
            operation_ref: command.operation_ref.clone(),
            manifest_ref: publication.manifest.manifest_ref.clone(),
            sequence: next_sequence(&state)?,
            chunk_ref: descriptor.chunk_ref.clone(),
            position: descriptor.position,
            observed_content_ref: crate::chunk_store::hash_chunk(&bytes, chunk_size),
            observed_length,
        };
        match apply_chunk_observation(profile, &publication.manifest, &state, &observation) {
            Ok(next) => {
                state = next;
                events.push(bind_event(
                    profile,
                    &event(EventInput {
                        command,
                        sequence: observation.sequence,
                        terminal: state.terminal,
                        chunk_ref: Some(descriptor.chunk_ref.clone()),
                        observed_bytes: observed_length,
                        failure: None,
                        evidence_refs: &publication.manifest.evidence_refs,
                    }),
                )?);
                verified_chunks.push(VerifiedChunkPayload {
                    chunk_ref: descriptor.chunk_ref.clone(),
                    position: descriptor.position,
                    bytes,
                });
            }
            Err(issues) => {
                state = classify_content_failure(
                    profile,
                    &state,
                    failure_from_issues(&issues),
                    UNACKNOWLEDGED_PUT,
                )
                .map_err(transition_error)?;
                return terminate(profile, publication, command, state, events, verified_chunks);
            }
        }
    }
    if state.terminal == ContentTerminal::Verified {
        if profile.capabilities.contains(&ContentCapability::DurableCompletion) {
            state = mark_content_durable(profile, &state).map_err(transition_error)?;
        }
        events.push(terminal_event(profile, publication, command, &state)?);
    }
    finish(profile, publication, state, events, verified_chunks)
}

fn terminate(
    profile: &ContentAdapterProfile,
    publication: &ObjectPublication,
    command: &ContentCommand,
    state: ContentPartialState,
    mut events: Vec<Artifact<ContentEvent>>,
    verified_chunks: Vec<VerifiedChunkPayload>,
) -> crate::error::Result<ObjectExecution> {
    events.push(terminal_event(profile, publication, command, &state)?);
    finish(profile, publication, state, events, verified_chunks)
}

fn finish(
    profile: &ContentAdapterProfile,
    publication: &ObjectPublication,
    state: ContentPartialState,
    events: Vec<Artifact<ContentEvent>>,
    verified_chunks: Vec<VerifiedChunkPayload>,
) -> crate::error::Result<ObjectExecution> {
    Ok(ObjectExecution {
        state: partial_state(profile, &publication.manifest, &state)?,
        events,
        verified_chunks,
        backend_hint_ref: publication.backend_hint_ref.clone(),
    })
}

fn terminal_event(
    profile: &ContentAdapterProfile,
    publication: &ObjectPublication,
    command: &ContentCommand,
    state: &ContentPartialState,
) -> crate::error::Result<Artifact<ContentEvent>> {
    bind_event(
        profile,
        &event(EventInput {
            command,
            sequence: state
                .last_sequence
                .ok_or_else(|| crate::error::Failure::invalid_harness("terminal state lacks sequence"))?,
            terminal: state.terminal,
            chunk_ref: None,
            observed_bytes: 0,
            failure: state.failure,
            evidence_refs: &publication.manifest.evidence_refs,
        }),
    )
}

fn next_sequence(state: &ContentPartialState) -> crate::error::Result<u64> {
    state
        .last_sequence
        .map_or(Some(0), |sequence| sequence.checked_add(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness("object-storage event sequence overflow"))
}

fn transition_error(issue: ContentIssue) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("object-storage transition denied: {issue:?}"))
}

fn validate_key_prefix(prefix: &str) -> crate::error::Result<()> {
    if prefix.is_empty()
        || prefix.starts_with('/')
        || prefix.contains("..")
        || !prefix.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/'))
    {
        return Err(crate::error::Failure::invalid_harness("object-storage key prefix is not bounded"));
    }
    Ok(())
}

fn backend_hint_ref(class: ContentAdapterClass, label: &str) -> crate::error::Result<String> {
    Ok(crate::preserves_rail::content_ref_from_bytes(format!("{}\0{label}", class.as_str()).as_bytes()))
}

#[cfg(test)]
mod tests {
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

    #[tokio::test]
    async fn fetch_runs_the_command_transition_against_a_local_endpoint() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 2048];
            let _ = socket.read(&mut buffer).await.expect("read");
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello")
                .await
                .expect("write");
        });
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let mut client = CachedClient::connect(
            ClientConfig {
                endpoint: format!("http://{address}"),
                region: "us-east-1".to_string(),
                bucket: "bucket".to_string(),
                access_key: "access".to_string(),
                secret_key: "secret".to_string(),
                session_token: None,
            },
            tier,
        )
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
        assert_eq!(
            execution.state.artifact.terminal,
            ContentTerminal::Verified,
            "failure={:?} events={:?}",
            execution.state.artifact.failure,
            execution.events.iter().map(|event| (event.artifact.terminal, event.artifact.failure)).collect::<Vec<_>>()
        );
        assert_eq!(execution.verified_chunks.len(), 1);
        assert_eq!(execution.verified_chunks[0].bytes, b"hello");
        assert_eq!(execution.events.last().expect("terminal").artifact.terminal, ContentTerminal::Verified);
    }
}

#[cfg(test)]
mod publish_tests {
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

    #[tokio::test]
    async fn publish_puts_every_verified_chunk() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 2048];
            let _ = socket.read(&mut buffer).await.expect("read");
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                .await
                .expect("write");
        });
        let chunk_dir = tempfile::tempdir().expect("chunk root");
        let chunk_root = crate::chunk_store::CapabilityChunkRoot::open(chunk_dir.path()).expect("chunk root");
        let object = crate::chunk_store::put_bytes_with_root(&chunk_root, "artifact", b"hello", 5).expect("put");
        let tier_dir = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(tier_dir.path(), 1024, false).expect("tier");
        let mut client = CachedClient::connect(
            ClientConfig {
                endpoint: format!("http://{address}"),
                region: "us-east-1".to_string(),
                bucket: "bucket".to_string(),
                access_key: "access".to_string(),
                secret_key: "secret".to_string(),
                session_token: None,
            },
            tier,
        )
        .expect("client");
        let publication = publish_object_chunks(&profile(), &chunk_root, &object.manifest_ref, &mut client, "objects")
            .await
            .expect("publish");
        assert_eq!(publication.keys.len(), object.chunk_refs.len());
        assert!(publication.keys[0].starts_with("objects/"));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn corrupt_remote_chunk_fails_without_verified_payload() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 2048];
            let _ = socket.read(&mut buffer).await.expect("read");
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nwrong")
                .await
                .expect("write");
        });
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let mut client = CachedClient::connect(
            ClientConfig {
                endpoint: format!("http://{address}"),
                region: "us-east-1".to_string(),
                bucket: "bucket".to_string(),
                access_key: "access".to_string(),
                secret_key: "secret".to_string(),
                session_token: None,
            },
            tier,
        )
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
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Failed);
        assert!(execution.verified_chunks.is_empty());
    }
}

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
        fetch_object_chunks(
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
        .expect("fetch")
    }

    #[tokio::test]
    async fn disconnect_fails_without_verified_payload() {
        let execution = fetch_with(b"", None, None).await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Failed);
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
        assert!(execution.verified_chunks.is_empty());
    }

    #[tokio::test]
    async fn timeout_fails_without_verified_payload() {
        let execution = fetch_with(b"", Some(Duration::from_millis(200)), Some(Duration::from_millis(50))).await;
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Failed);
        assert!(execution.verified_chunks.is_empty());
    }
}

#[cfg(test)]
mod live_tests {
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
                max_total_bytes: 4096,
                max_chunk_count: 8,
                max_chunk_bytes: 4096,
                max_range_bytes: 4096,
                max_concurrent_operations: 1,
                max_queued_bytes: 4096,
                max_memory_bytes: 4096,
                max_deadline_ticks: 10,
                max_retries: 0,
                max_events: 16,
                max_status_entries: 8,
            },
            vec![reference("evidence")],
        )
        .expect("profile")
    }

    #[tokio::test]
    #[ignore = "requires MOLTEN_S3_ENDPOINT and MOLTEN_S3_BUCKET"]
    async fn live_publish_and_fetch_run_the_transition_core() {
        let endpoint = std::env::var("MOLTEN_S3_ENDPOINT").expect("endpoint");
        let bucket = std::env::var("MOLTEN_S3_BUCKET").expect("bucket");
        let chunk_dir = tempfile::tempdir().expect("chunk root");
        let chunk_root = crate::chunk_store::CapabilityChunkRoot::open(chunk_dir.path()).expect("chunk root");
        let object = crate::chunk_store::put_bytes_with_root(&chunk_root, "artifact", b"live-object", 4).expect("put");
        let tier_dir = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(tier_dir.path(), 4096, false).expect("tier");
        let mut client = CachedClient::connect(
            ClientConfig {
                endpoint,
                region: "us-east-1".to_string(),
                bucket,
                access_key: "molten".to_string(),
                secret_key: "molten".to_string(),
                session_token: None,
            },
            tier,
        )
        .expect("client");
        let profile = profile();
        let publication = publish_object_chunks(&profile, &chunk_root, &object.manifest_ref, &mut client, "objects")
            .await
            .expect("publish");
        let command = crate::content_store_adapter::command(&profile, crate::content_store_adapter::CommandInput {
            operation_ref: reference("operation"),
            operation: ContentOperation::Get,
            manifest: &publication.manifest,
            range: None,
            submitted_tick: 0,
            deadline_tick: 10,
            retry_count: 0,
            cancelled: false,
            policy_refs: vec![reference("policy")],
        })
        .expect("command");
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
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Verified);
        assert_eq!(execution.verified_chunks.len(), object.chunk_refs.len());
    }
}

#[cfg(test)]
mod live_fault_tests {
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
                max_total_bytes: 4096,
                max_chunk_count: 8,
                max_chunk_bytes: 4096,
                max_range_bytes: 4096,
                max_concurrent_operations: 1,
                max_queued_bytes: 4096,
                max_memory_bytes: 4096,
                max_deadline_ticks: 10,
                max_retries: 0,
                max_events: 16,
                max_status_entries: 8,
            },
            vec![reference("evidence")],
        )
        .expect("profile")
    }

    #[tokio::test]
    #[ignore = "requires MOLTEN_S3_ENDPOINT and MOLTEN_S3_BUCKET"]
    async fn live_corrupt_remote_chunk_fails_without_verified_payload() {
        let endpoint = std::env::var("MOLTEN_S3_ENDPOINT").expect("endpoint");
        let bucket = std::env::var("MOLTEN_S3_BUCKET").expect("bucket");
        let chunk_dir = tempfile::tempdir().expect("chunk root");
        let chunk_root = crate::chunk_store::CapabilityChunkRoot::open(chunk_dir.path()).expect("chunk root");
        let object = crate::chunk_store::put_bytes_with_root(&chunk_root, "artifact", b"live", 4).expect("put");
        let tier_dir = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(tier_dir.path(), 4096, false).expect("tier");
        let mut client = CachedClient::connect(
            ClientConfig {
                endpoint,
                region: "us-east-1".to_string(),
                bucket,
                access_key: "molten".to_string(),
                secret_key: "molten".to_string(),
                session_token: None,
            },
            tier,
        )
        .expect("client");
        let profile = profile();
        let publication = publish_object_chunks(&profile, &chunk_root, &object.manifest_ref, &mut client, "objects")
            .await
            .expect("publish");
        client.put(&publication.keys[0], b"wrong").await.expect("overwrite");
        let command = crate::content_store_adapter::command(&profile, crate::content_store_adapter::CommandInput {
            operation_ref: reference("operation"),
            operation: ContentOperation::Get,
            manifest: &publication.manifest,
            range: None,
            submitted_tick: 0,
            deadline_tick: 10,
            retry_count: 0,
            cancelled: false,
            policy_refs: vec![reference("policy")],
        })
        .expect("command");
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
        assert_eq!(execution.state.artifact.terminal, ContentTerminal::Failed);
        assert!(execution.verified_chunks.is_empty());
    }
}

/// Explicit retention protections for remote object chunks. Canonical retention
/// remains the authority; this guard only prevents a shell delete from acting on
/// a chunk the caller marked protected.
#[derive(Debug, Default)]
pub struct Retention {
    protected: std::collections::BTreeSet<String>,
}

impl Retention {
    pub fn protect(&mut self, chunk_ref: &str) {
        self.protected.insert(chunk_ref.to_string());
    }

    pub fn unprotect(&mut self, chunk_ref: &str) {
        self.protected.remove(chunk_ref);
    }

    pub fn is_protected(&self, chunk_ref: &str) -> bool {
        self.protected.contains(chunk_ref)
    }
}

pub async fn delete_chunk(
    client: &mut CachedClient,
    retention: &Retention,
    key: &str,
    chunk_ref: &str,
) -> crate::error::Result<()> {
    if retention.is_protected(chunk_ref) {
        return Err(crate::error::Failure::invalid_harness("object-storage delete denied by retention"));
    }
    let object_ref = object_ref_from_key(key)?;
    client.delete(key, &object_ref).await
}

fn object_ref_from_key(key: &str) -> crate::error::Result<String> {
    let hex = key.rsplit('/').next().unwrap_or_default();
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
        return Err(crate::error::Failure::invalid_harness("object-storage key does not name a canonical object"));
    }
    Ok(format!("blake3:{hex}"))
}

#[cfg(test)]
mod retention_tests {
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::content_store_adapter::ClientConfig;
    use crate::content_store_adapter::LocalTier;

    fn config(endpoint: String) -> ClientConfig {
        ClientConfig {
            endpoint,
            region: "us-east-1".to_string(),
            bucket: "bucket".to_string(),
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
            session_token: None,
        }
    }

    #[tokio::test]
    async fn protected_delete_makes_no_remote_request() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let mut client = CachedClient::connect(config(format!("http://{address}")), tier).expect("client");
        let raw = crate::preserves_rail::content_ref_from_bytes(b"hello");
        let key = format!("objects/{}", raw.trim_start_matches("blake3:"));
        let mut retention = Retention::default();
        retention.protect("chunk-ref");
        assert!(delete_chunk(&mut client, &retention, &key, "chunk-ref").await.is_err());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), listener.accept()).await.is_err(),
            "protected delete must not connect"
        );
    }

    #[tokio::test]
    async fn unprotected_delete_removes_remote_and_local_copy() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 2048];
            let _ = socket.read(&mut buffer).await.expect("read");
            socket
                .write_all(b"HTTP/1.1 204 No Content\r\nconnection: close\r\n\r\n")
                .await
                .expect("write");
        });
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let mut client = CachedClient::connect(config(format!("http://{address}")), tier).expect("client");
        let raw = crate::preserves_rail::content_ref_from_bytes(b"hello");
        let key = format!("objects/{}", raw.trim_start_matches("blake3:"));
        client.cache_verified(b"hello").expect("cache");
        assert!(client.contains(&raw).expect("tier"));
        delete_chunk(&mut client, &Retention::default(), &key, "chunk-ref").await.expect("delete");
        assert!(!client.contains(&raw).expect("tier"));
    }
}
