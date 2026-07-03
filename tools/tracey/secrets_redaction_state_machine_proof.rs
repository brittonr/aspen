//! Tracey markers for secrets redaction state-machine proof coverage.
//!
//! Implementation and tests live in `src/secrets/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to focused secrets redaction tests.
//!
//! Verification evidence:
//! - `src/secrets/parts/mod/p000/body.rs`
//! - `src/secrets/parts/mod/p001/body.rs`
//! - `src/secrets/parts/mod/p003/body.rs`
//! - `src/secrets/parts/mod/p007/body.rs`

// r[verify molten.secrets_state_proof.exact_reveal_binding]
// r[verify molten.secrets_state_proof.redaction_profile_gate]
// r[verify molten.secrets_state_proof.cleanup_retention_gate]
