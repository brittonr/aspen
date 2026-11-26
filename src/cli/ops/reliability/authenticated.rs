pub(super) fn run(request: std::path::PathBuf, response: std::path::PathBuf,
    authority_root: std::path::PathBuf, client_key: std::path::PathBuf) -> molten::error::Result<()> {
    let mut request: molten::live_reliability::protocol::Request = super::read(&request)?;
    if request.authorization.is_some() { return Err(denied("adapter will not replace supplied request authorization")); }
    let root = molten::node_state::DirectoryRoot::open_existing(&authority_root)?;
    let grant = molten::live_reliability::authority::load(&root)?;
    if grant.endpoint() != request.endpoint { return Err(denied("client authority names another endpoint")); }
    let secret: [u8; 32] = super::read(&client_key)?;
    let key = iroh::SecretKey::from_bytes(&secret);
    let mut nonce = [0; 32];
    molten::fabric_time::CryptographicEntropySource::fill_secret(
        &mut molten::fabric_time::OperatingSystemEntropySource, &mut nonce)
        .map_err(|error| denied(format!("client nonce entropy: {error:?}")))?;
    // Reads are not fenced; a write is fenced to the term the endpoint reports.
    let fence_term = if is_write(&request) {
        match fence_term(&request) {
            Some(term) => term,
            None => {
                let unavailable = molten::live_reliability::protocol::response(&request,
                    molten::live_reliability::protocol::Outcome::Fail, serde_json::Value::Null,
                    Some("write-fence-term-unavailable".into()));
                return super::write(&response, &unavailable);
            }
        }
    } else {
        0
    };
    request.authorization =
        Some(molten::live_reliability::authority::sign_request(&key, &grant, &request, nonce, fence_term)?);
    grant.verify_current(&root)?;
    let result = molten::live_reliability::client::execute(&request);
    // Authority refresh after an uncertain effect cannot rewrite its client-visible outcome.
    super::write(&response, &result)
}

fn is_write(request: &molten::live_reliability::protocol::Request) -> bool {
    request.phase == molten::live_reliability::protocol::Phase::Invoke
        && request.operation.as_ref().and_then(|value| value.get("function")).and_then(|value| value.as_str())
            == Some("write")
}

/// The endpoint's current Raft term from an unchallenged probe, for a write's signed fence. The
/// unsigned answer is advisory: a wrong term only makes the leader deny the write before any effect.
fn fence_term(request: &molten::live_reliability::protocol::Request) -> Option<u64> {
    let probe = molten::live_reliability::protocol::Request {
        schema: request.schema.clone(),
        phase: molten::live_reliability::protocol::Phase::Probe,
        run_id: request.run_id.clone(),
        endpoint: request.endpoint.clone(),
        operation: None,
        deadline_ms: request.deadline_ms,
        profile: request.profile.clone(),
        authorization: None,
        probe: None,
    };
    let answer = molten::live_reliability::client::execute(&probe);
    if answer.outcome != molten::live_reliability::protocol::Outcome::Ok { return None; }
    answer.fence_term
}

fn denied(message: impl Into<String>) -> molten::error::Failure { molten::error::Failure::invalid_harness(message) }
