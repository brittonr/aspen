
fn status_fact_for_token(
    state: &CoordinationState,
    manifest: &CoordinationServiceManifest,
    service: &str,
    key: &str,
    token: &FencingToken,
) -> Result<IoValue> {
    match service {
        SERVICE_LOCK => Ok(record("lock-held", vec![
            string(key),
            string(&token.owner),
            u64_value(token.token),
            string(&token.token_ref),
        ])),
        SERVICE_ELECTION => Ok(record("leader", vec![
            string(key),
            string(&token.owner),
            u64_value(token.token),
            string(&token.token_ref),
        ])),
        _ => status_fact_for(state, manifest, service, key),
    }
}
