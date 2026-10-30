//! Owner-defined DAG binding denials at federation admission and supervised dispatch.

/// A well-formed conformance identity for negative admission checks only; never approval.
fn structural_conformance_ref() -> String {
    super::setup::transport::reference("binding-denial-structural-conformance")
}

fn receiver_manifest_input(workspace: &std::path::Path) -> crate::error::Result<crate::mesh::admission::ManifestInput> {
    let root = crate::node_state::DirectoryRoot::open(workspace)?;
    let secrets = root.secrets()?;
    let adapter = super::setup::profiles::crypto(&secrets)?;
    let policy = super::setup::transport::reference("federation-live-signature-policy");
    let key =
        adapter.resolve_or_generate(crate::fabric_crypto_identity::KeyPurpose::FederationOrigin, &policy, true)?;
    let signer = key.handle.handle.public_key_ref.clone();
    let artifact_ref = crate::preserves_rail::canonical_hash(&super::setup::profiles::artifact())?;
    let local_policy = super::setup::profiles::local_policy(&artifact_ref, &signer)?;
    let proofset = super::setup::profiles::proofset(&artifact_ref, &signer)?;
    super::setup::admission::manifest_input(
        &local_policy,
        &proofset,
        &signer,
        adapter.profile(),
        &structural_conformance_ref(),
    )
}

// r[verify molten.dag_sync.fabric_admission]
#[test]
fn federation_manifest_requires_the_owner_admitted_dag_binding() {
    let workspace = crate::test_support::ProcessWorkspace::new("federation-dag-binding").expect("workspace");
    let admitted = receiver_manifest_input(&workspace).expect("receiver manifest input");
    let transport_binding = admitted
        .extension
        .required_port_bindings()
        .iter()
        .find(|binding| binding.binding.key.port_id == crate::fabric_transport::FABRIC_TRANSPORT_PORT_ID)
        .expect("admitted transport binding")
        .binding_ref
        .clone();
    let manifest = crate::mesh::admission::admit_manifest(admitted).expect("exact DAG binding admits");
    assert!(!manifest.dag_binding_ref().is_empty());
    for substitute in [
        super::setup::transport::reference("unadmitted-dag-binding"),
        transport_binding,
    ] {
        let mut input = receiver_manifest_input(&workspace).expect("fresh receiver manifest input");
        input.dag_binding_ref = substitute;
        assert!(
            crate::mesh::admission::admit_manifest(input).is_err(),
            "a binding that is not the owner-admitted DAG descriptor must not admit federation"
        );
    }
}

// r[verify molten.dag_sync.fabric_admission]
#[test]
fn supervised_dag_effect_denies_stale_generation_and_foreign_request() {
    let head = crate::preserves_rail::canonical_hash(&super::setup::profiles::artifact()).expect("artifact head");
    let schema = crate::preserves_rail::boundary_schema_ref(&super::setup::profiles::ARTIFACT_SCHEMA).expect("schema");
    let session = super::setup::transport::reference("binding-denial-session");
    let (_, request) =
        super::setup::dagports::request(&head, &session, schema.as_str(), 64).expect("receiver DAG request");
    let effect = super::receiver::effect(&request).expect("supervised effect");
    let exact = crate::dag_sync::canonical_dag_request(&request).expect("canonical request").record_ref;
    crate::mesh::transfer::execution::require_supervised_effect(&effect, &exact, effect.generation)
        .expect("exact request at the session generation dispatches");
    assert!(
        crate::mesh::transfer::execution::require_supervised_effect(&effect, &exact, effect.generation + 1).is_err(),
        "an effect from an earlier generation must not dispatch"
    );
    let other = super::setup::transport::reference("binding-denial-other-session");
    let (_, foreign) = super::setup::dagports::request(&head, &other, schema.as_str(), 64).expect("foreign request");
    let foreign = crate::dag_sync::canonical_dag_request(&foreign).expect("canonical foreign request").record_ref;
    assert_ne!(foreign, exact);
    assert!(
        crate::mesh::transfer::execution::require_supervised_effect(&effect, &foreign, effect.generation).is_err(),
        "an effect naming a different receiver request must not dispatch"
    );
}
