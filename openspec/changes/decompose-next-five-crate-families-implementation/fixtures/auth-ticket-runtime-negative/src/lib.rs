// This fixture intentionally depends only on aspen-auth-core. Importing the
// runtime verifier facade must fail so portable defaults cannot reach revocation
// or verifier shells through the auth/ticket boundary.
use aspen_auth::TokenVerifier;

pub fn needs_runtime_verifier() -> TokenVerifier {
    TokenVerifier::new()
}
