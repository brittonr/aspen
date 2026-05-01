use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_coordination_protocol::LockResultResponse;
use aspen_forge_protocol::ForgeRepoBackend;
use aspen_forge_protocol::ForgeRepoInfo;
use aspen_jobs_protocol::JobSubmitResultResponse;

const TEST_CREATED_AT_MS: u64 = 1_234;
const TEST_THRESHOLD_DELEGATES: u32 = 1;
const TEST_FENCING_TOKEN: u64 = 9;
const TEST_DEADLINE_MS: u64 = 10_000;

#[test]
fn downstream_serializes_client_wire_types() {
    let request = ClientRpcRequest::Ping;
    let bytes = postcard::to_allocvec(&request).expect("serialize client request");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize client request");

    assert!(matches!(decoded, ClientRpcRequest::Ping));

    let response = ClientRpcResponse::Pong;
    let bytes = postcard::to_allocvec(&response).expect("serialize client response");
    let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize client response");

    assert!(matches!(decoded, ClientRpcResponse::Pong));
}

#[test]
fn downstream_serializes_domain_protocol_types() {
    let repo = ForgeRepoInfo {
        id: "repo-id".into(),
        name: "repo".into(),
        description: Some("fixture".into()),
        default_branch: "main".into(),
        delegates: vec!["delegate".into()],
        threshold_delegates: TEST_THRESHOLD_DELEGATES,
        created_at_ms: TEST_CREATED_AT_MS,
        backends: vec![ForgeRepoBackend::Git],
        backend_routes: Vec::new(),
    };
    let encoded = serde_json::to_string(&repo).expect("serialize forge repo");
    let decoded: ForgeRepoInfo = serde_json::from_str(&encoded).expect("deserialize forge repo");
    assert_eq!(decoded.name, "repo");
    assert_eq!(decoded.threshold_delegates, TEST_THRESHOLD_DELEGATES);

    let job = JobSubmitResultResponse {
        is_success: true,
        job_id: Some("job-id".into()),
        error: None,
    };
    let encoded = serde_json::to_string(&job).expect("serialize job response");
    let decoded: JobSubmitResultResponse = serde_json::from_str(&encoded).expect("deserialize job response");
    assert_eq!(decoded.job_id.as_deref(), Some("job-id"));

    let lock = LockResultResponse {
        is_success: true,
        fencing_token: Some(TEST_FENCING_TOKEN),
        holder_id: Some("holder".into()),
        deadline_ms: Some(TEST_DEADLINE_MS),
        error: None,
    };
    let encoded = serde_json::to_string(&lock).expect("serialize lock response");
    let decoded: LockResultResponse = serde_json::from_str(&encoded).expect("deserialize lock response");
    assert_eq!(decoded.fencing_token, Some(TEST_FENCING_TOKEN));
    assert_eq!(decoded.deadline_ms, Some(TEST_DEADLINE_MS));
}
