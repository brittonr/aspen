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
    request.authorization = Some(molten::live_reliability::authority::sign_request(&key, &grant, &request, nonce)?);
    grant.verify_current(&root)?;
    let result = molten::live_reliability::client::execute(&request);
    // Authority refresh after an uncertain effect cannot rewrite its client-visible outcome.
    super::write(&response, &result)
}

fn denied(message: impl Into<String>) -> molten::error::Failure { molten::error::Failure::invalid_harness(message) }
