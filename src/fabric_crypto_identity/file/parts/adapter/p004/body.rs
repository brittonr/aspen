
impl IrohEd25519<'_> {
    // r[impl molten.crypto_identity.rotation_revocation]
    pub fn revoke(
        &self,
        handle: &OpaqueKeyHandle,
        revocation_evidence_ref: &str,
        policy_ref: &str,
    ) -> crate::error::Result<StatusReadback> {
        require_blake3_ref("revocation evidence", revocation_evidence_ref)?;
        require_blake3_ref("revocation policy", policy_ref)?;
        let current = self.current_key(handle.purpose, policy_ref, false)?;
        require_current_handle(handle, &current.resolved.handle.handle)?;
        let marker_path = revocation_path(handle.purpose)?;
        // Reading, extending and publishing the revocation set is one update under the set's lock, so
        // a concurrent revocation can never publish a set that drops this or an earlier record.
        let _update = self.namespace.lock_update(&marker_path)?;
        #[cfg(test)]
        seam::fire("revoke-before-read", self);
        let mut revocations = read_key_revocations(self.namespace, handle.purpose)?;
        #[cfg(test)]
        seam::fire("revoke-after-read", self);
        if !key_revocation_matches(&revocations, KeyRevocationSubject::of(handle)) {
            // Deny before publishing a set that readers would reject as over-bound.
            if revocations.len() >= MAX_KEY_REVOCATIONS {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "{} key revocation set is full ({MAX_KEY_REVOCATIONS} records); the key was not revoked",
                    handle.purpose.as_str()
                )));
            }
            revocations.push(KeyRevocation {
                handle_ref: handle.handle_ref.clone(),
                public_key_ref: handle.public_key_ref.clone(),
                generation: handle.generation,
                evidence_ref: revocation_evidence_ref.to_string(),
                policy_ref: policy_ref.to_string(),
            });
        }
        self.namespace.write_restricted(
            &marker_path,
            &crate::preserves_rail::canonical_bytes(&key_revocations_value(&revocations))?,
            OWNER_ONLY_SECRET_FILE_MODE,
        )?;
        self.status_for_handle(
            &current.resolved.handle.handle,
            current.resolved.permission_status,
            KeyCurrentness::Revoked,
            vec![revocation_evidence_ref.to_string()],
        )
    }
}

#[cfg(test)]
impl IrohEd25519<'_> {
    /// The namespace this adapter reads, so a test hook can observe exactly what it is about to read.
    pub(super) fn test_namespace(&self) -> &crate::node_state::Scope {
        self.namespace
    }
}
