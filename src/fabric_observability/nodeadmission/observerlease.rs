pub struct ObserverAuthorityLease {
    root: crate::node_state::DirectoryRoot,
    selection: super::NodeObservationSelection,
    trust_ref: String,
    credential_ref: String,
    receipt_ref: String,
}

pub fn admit_node_observation(
    root: &crate::node_state::DirectoryRoot,
    selection: &super::NodeObservationSelection,
    gate: &crate::node_runtime::ValidatedNodeSourceGate,
    now: u64,
) -> crate::error::Result<ObserverAuthorityLease> {
    super::node_observation_selection_ref(selection)?;
    if gate.validation_ref() != selection.source_validation_ref {
        return Err(super::denied());
    }
    admit(root, selection, now)
}

/// `now` is the shell-observed Unix second; admission never reads the ambient clock.
pub(super) fn admit(
    root: &crate::node_state::DirectoryRoot,
    selection: &super::NodeObservationSelection,
    now: u64,
) -> crate::error::Result<ObserverAuthorityLease> {
    let (trust, credential, trust_ref, credential_ref) = current(root, selection, now)?;
    let resource = super::node_observation_resource(selection)?;
    let input = crate::node_service_host::authority::boundaries::Verification {
        root,
        now,
        resource: &resource,
        ability: "read",
        contract: "linux-observation-reader",
        operation: super::node_observation_binding(selection)?,
    };
    let digest = crate::node_service_host::authority::boundaries::verify(&trust, &credential, input)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "linux-observation-authorized-v1",
        vec![
            crate::preserves_rail::string(super::node_observation_selection_ref(selection)?),
            crate::preserves_rail::string(&trust_ref),
            crate::preserves_rail::string(&credential_ref),
            crate::preserves_rail::string(digest),
        ],
    ))?;
    Ok(ObserverAuthorityLease {
        root: root.clone(),
        selection: selection.clone(),
        trust_ref,
        credential_ref,
        receipt_ref,
    })
}

impl ObserverAuthorityLease {
    pub fn receipt_ref(&self) -> &str {
        &self.receipt_ref
    }

    /// Reverification cannot consume the initial holder nonce a second time.
    pub fn validate_current(&self, now: u64) -> crate::error::Result<()> {
        let (trust, credential, trust_ref, credential_ref) = current(&self.root, &self.selection, now)?;
        if trust_ref != self.trust_ref || credential_ref != self.credential_ref {
            return Err(super::denied());
        }
        let resource = super::node_observation_resource(&self.selection)?;
        crate::node_service_host::authority::boundaries::verify_grant(
            &trust,
            &credential,
            &crate::node_service_host::authority::boundaries::Verification {
                root: &self.root,
                now,
                resource: &resource,
                ability: "read",
                contract: "linux-observation-reader",
                operation: super::node_observation_binding(&self.selection)?,
            },
        )?;
        Ok(())
    }
}

type Current = (
    crate::node_service_host::authority::wire::Trust,
    crate::node_service_host::authority::wire::Credential,
    String,
    String,
);

fn current(
    root: &crate::node_state::DirectoryRoot,
    selection: &super::NodeObservationSelection,
    now: u64,
) -> crate::error::Result<Current> {
    let path = crate::node_state::RelativePath::parse("linux-observation-trust")?;
    let trust_bytes = root.read(&path, 262144)?;
    let credential_bytes = root.read(&super::node_observer_credential_path(selection)?, 2228224)?;
    let trust = super::decode_node_observer_trust(&trust_bytes)?;
    let credential = super::decode_node_observer_credential(&credential_bytes)?;
    if now < trust.not_before || now >= trust.expires {
        return Err(super::denied());
    }
    crate::node_service_host::authority::boundaries::validate_keys(&trust)?;
    Ok((
        trust,
        credential,
        crate::preserves_rail::content_ref_from_bytes(&trust_bytes),
        crate::preserves_rail::content_ref_from_bytes(&credential_bytes),
    ))
}
