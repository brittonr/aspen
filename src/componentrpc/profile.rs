//! Closed pilot resource and delivery contracts, not advisory numeric hints.
pub const MEMORY_BYTES: usize = 64 * 1024 * 1024;
pub const FUEL: u64 = 10_000_000;
pub const TIMEOUT_SECONDS: u64 = 10;
pub const NONCLAIMS: [&str; 6] = [
    "not-protocol-stability",
    "not-semantic-equivalence",
    "not-transport-security",
    "not-production-readiness",
    "not-component-correctness",
    "not-existing-protocol-replacement",
];

pub fn classify(claim: &str) -> super::Result<()> {
    if claim != "pinned-experimental-fixture-only" {
        return Err("pilot evidence cannot support that claim".into());
    }
    Ok(())
}

pub fn resources() -> super::Result<String> {
    super::hash(&crate::codec::record("component-rpc-resources-v1", vec![
        crate::codec::string(FUEL.to_string()),
        crate::codec::string(MEMORY_BYTES.to_string()),
        crate::codec::string(super::MAX_BYTES.to_string()),
        crate::codec::string("64/16/16/65536/1048576"),
        crate::codec::string("one-invocation"),
        crate::codec::string(TIMEOUT_SECONDS.to_string()),
    ]))
}
pub fn delivery() -> super::Result<String> {
    super::hash(&crate::codec::record("component-rpc-delivery-policy-v1", vec![
        crate::codec::string("single-attempt"),
        crate::codec::string("no-automatic-retry"),
        crate::codec::string("post-effect-failure-is-uncertain"),
        crate::codec::string("replay-recorded-effects-only"),
    ]))
}
pub fn response_session(scope: &super::bridge::Scope) -> super::Result<String> {
    super::hash(&crate::codec::record("component-rpc-response-session-v1", vec![
        crate::codec::string(&scope.session),
        crate::codec::string(super::bridge::invocation(scope)?),
    ]))
}
