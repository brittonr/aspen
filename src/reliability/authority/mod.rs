//! Separately provisioned experimental coordination authority; source evidence is not effect
//! authority.
mod attestation;
mod canonical;
mod client;
mod issuance;
mod storage;
#[cfg(test)]
mod tests;

pub use attestation::Challenge;
pub use attestation::ProbeState;
pub use attestation::SignedProbe;
pub use attestation::sign_probe;
pub use attestation::verify_peer_probe;
pub use attestation::verify_probe;
pub use client::SignedInvocation;
pub use client::sign_request;
pub use client::verify_request;
pub use issuance::Grant;
pub use issuance::Policy;
pub use issuance::SignedGrant;
pub use issuance::SignedPolicy;
pub use issuance::VerifiedGrant;
pub use issuance::grant_identity;
pub use storage::ProvisionInput;
pub use storage::load;
pub use storage::load_at;
pub use storage::load_probe_key;
pub use storage::provision;

fn denied(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

#[cfg(test)]
pub(crate) fn signed_probe_fixture(key: &iroh::SecretKey, challenge: Challenge, state: ProbeState) -> SignedProbe {
    let signature = canonical::sign("live-readiness-probe", &(&challenge, &state), key).expect("test probe signing");
    SignedProbe {
        challenge,
        state,
        signature,
    }
}
