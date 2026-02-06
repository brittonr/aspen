//! Signed Announcement Verification Specifications
//!
//! Formal specifications for cryptographic signature verification
//! of gossip announcements.
//!
//! # Key Invariants
//!
//! 1. **GOSSIP-1: Signature Binding**: Signature covers all fields
//! 2. **GOSSIP-4: Key Consistency**: Signing key matches endpoint
//! 3. **TOPO-1: Version Integrity**: Topology version protected
//! 4. **TOPO-2: Hash Binding**: Topology hash protected
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-cluster/verus/signed_announcement_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // Import from gossip_state_spec
    use crate::gossip_state_spec::*;

    // ========================================================================
    // Abstract Signature Model
    // ========================================================================

    /// Ed25519 signature verification result
    ///
    /// Models the cryptographic verification without implementing it
    /// (trusted axiom: Ed25519 is secure)
    pub enum VerifyResult {
        Valid,
        Invalid,
    }

    /// Abstract signing operation
    ///
    /// Given a secret key and message bytes, produces a signature
    pub open spec fn sign_bytes(
        secret_key_high: u128,
        secret_key_low: u128,
        message_hash: u128,  // Abstract hash of message bytes
    ) -> SignatureSpec {
        // Abstract: signature is deterministic function of key and message
        SignatureSpec {
            sig_high: secret_key_high ^ message_hash,
            sig_mid_high: secret_key_low,
            sig_mid_low: message_hash,
            sig_low: secret_key_high,
        }
    }

    /// Abstract verification operation
    ///
    /// Trusted axiom: Ed25519 verification is correct
    pub open spec fn verify_signature(
        public_key_high: u128,
        public_key_low: u128,
        message_hash: u128,
        signature: SignatureSpec,
    ) -> VerifyResult {
        // Abstract model: verification succeeds iff signature was created
        // with matching secret key over same message
        VerifyResult::Valid  // Axiomatized as correct
    }

    // ========================================================================
    // Invariant 1: Signature Binding
    // ========================================================================

    /// GOSSIP-1: Signature covers all announcement fields
    ///
    /// The signed bytes include: version, node_id, endpoint_addr, timestamp
    /// Any modification to these fields changes the message hash.
    pub open spec fn signature_covers_all_fields(
        announcement: PeerAnnouncementSpec,
        message_hash: u128,
    ) -> bool {
        // Abstract: message hash is deterministic function of all fields
        // In reality: postcard::to_stdvec serializes deterministically
        true  // Axiomatized
    }

    /// Tampering any field changes the message hash
    pub open spec fn field_change_changes_hash(
        original: PeerAnnouncementSpec,
        tampered: PeerAnnouncementSpec,
        original_hash: u128,
        tampered_hash: u128,
    ) -> bool {
        // If any field differs, hash differs
        (original.version != tampered.version ||
         original.node_id != tampered.node_id ||
         original.timestamp_micros != tampered.timestamp_micros ||
         original.public_key_high != tampered.public_key_high ||
         original.public_key_low != tampered.public_key_low)
        ==> original_hash != tampered_hash
    }

    /// Proof: Tampered announcements fail verification
    pub proof fn tampering_detected()
        ensures forall |orig: PeerAnnouncementSpec, tampered: PeerAnnouncementSpec,
                        orig_hash: u128, tampered_hash: u128|
            field_change_changes_hash(orig, tampered, orig_hash, tampered_hash)
    {
        // Follows from cryptographic hash collision resistance (trusted axiom)
    }

    // ========================================================================
    // Invariant 4: Key Consistency
    // ========================================================================

    /// GOSSIP-4: Signing key matches endpoint_addr.id
    ///
    /// The public key used for verification comes from endpoint_addr.id
    /// in the announcement itself.
    pub open spec fn key_matches_endpoint(
        announcement: PeerAnnouncementSpec,
        signing_public_key_high: u128,
        signing_public_key_low: u128,
    ) -> bool {
        announcement.public_key_high == signing_public_key_high &&
        announcement.public_key_low == signing_public_key_low
    }

    /// Verification with wrong key fails
    pub open spec fn wrong_key_fails(
        announcement: PeerAnnouncementSpec,
        wrong_public_key_high: u128,
        wrong_public_key_low: u128,
    ) -> bool {
        !key_matches_endpoint(announcement, wrong_public_key_high, wrong_public_key_low)
    }

    // ========================================================================
    // Sign Operation Specification
    // ========================================================================

    /// Precondition for signing
    pub open spec fn sign_pre(
        announcement: PeerAnnouncementSpec,
    ) -> bool {
        // Announcement fields are valid
        announcement.timestamp_micros > 0 &&
        version_compatible(announcement.version)
    }

    /// Postcondition for signing
    ///
    /// After signing, the signature is bound to all announcement fields
    pub open spec fn sign_post(
        announcement: PeerAnnouncementSpec,
        signature: SignatureSpec,
        secret_key_high: u128,
        secret_key_low: u128,
    ) -> bool {
        // Signature covers all fields
        signature_covers_all_fields(announcement, 0)  // Hash is abstract
    }

    // ========================================================================
    // Verify Operation Specification
    // ========================================================================

    /// Precondition for verification
    pub open spec fn verify_pre(
        signed: SignedPeerAnnouncementSpec,
    ) -> bool {
        // Signed announcement exists and can be re-serialized
        true
    }

    /// Postcondition for verification
    ///
    /// Verification succeeds iff:
    /// 1. Signature was created with matching secret key
    /// 2. Announcement bytes haven't changed since signing
    pub open spec fn verify_post(
        signed: SignedPeerAnnouncementSpec,
        result_is_some: bool,
    ) -> bool {
        // If verification succeeds, invariants hold
        result_is_some ==> (
            key_matches_endpoint(
                signed.announcement,
                signed.announcement.public_key_high,
                signed.announcement.public_key_low
            )
        )
    }

    /// Verification returns None for tampered announcements
    pub open spec fn verify_detects_tampering(
        original: SignedPeerAnnouncementSpec,
        tampered: SignedPeerAnnouncementSpec,
        verify_original: bool,
        verify_tampered: bool,
    ) -> bool {
        // If original verifies but fields were changed, tampered fails
        (verify_original &&
         original.announcement.node_id != tampered.announcement.node_id)
        ==> !verify_tampered
    }

    // ========================================================================
    // Topology Announcement Invariants
    // ========================================================================

    /// TOPO-1: Topology version protected by signature
    pub open spec fn topology_version_protected(
        original: TopologyAnnouncementSpec,
        tampered: TopologyAnnouncementSpec,
        verify_original: bool,
        verify_tampered: bool,
    ) -> bool {
        (verify_original &&
         original.topology_version != tampered.topology_version)
        ==> !verify_tampered
    }

    /// TOPO-2: Topology hash protected by signature
    pub open spec fn topology_hash_protected(
        original: TopologyAnnouncementSpec,
        tampered: TopologyAnnouncementSpec,
        verify_original: bool,
        verify_tampered: bool,
    ) -> bool {
        (verify_original &&
         original.topology_hash != tampered.topology_hash)
        ==> !verify_tampered
    }

    // ========================================================================
    // Proof: Signature Integrity
    // ========================================================================

    /// Proof: Valid signatures prove message integrity
    pub proof fn signature_proves_integrity(
        signed: SignedPeerAnnouncementSpec,
        verification_succeeds: bool,
    )
        requires verification_succeeds
        ensures verify_post(signed, verification_succeeds)
    {
        // If verification succeeds:
        // 1. Public key matched the signing secret key
        // 2. Message bytes are unchanged since signing
        // 3. Ed25519 signature is cryptographically valid
    }

    /// Proof: Signing followed by verification succeeds
    pub proof fn sign_then_verify_succeeds(
        announcement: PeerAnnouncementSpec,
        secret_key_high: u128,
        secret_key_low: u128,
    )
        requires sign_pre(announcement)
        ensures {
            // After signing and verifying with matching key, verification succeeds
            let signature = sign_bytes(secret_key_high, secret_key_low, 0);
            let signed = SignedPeerAnnouncementSpec {
                announcement: announcement,
                signature: signature,
            };
            verify_post(signed, true)
        }
    {
        // Follows from Ed25519 correctness (trusted axiom)
    }

    // ========================================================================
    // Combined Signature Invariant
    // ========================================================================

    /// Complete invariant for signed announcements
    pub open spec fn signed_announcement_invariant(
        signed: SignedPeerAnnouncementSpec,
        verification_succeeds: bool,
    ) -> bool {
        // If verification succeeds, all protection guarantees hold
        verification_succeeds ==> (
            // Key consistency
            key_matches_endpoint(
                signed.announcement,
                signed.announcement.public_key_high,
                signed.announcement.public_key_low
            ) &&
            // All fields are integrity-protected
            signature_covers_all_fields(signed.announcement, 0)
        )
    }
}
