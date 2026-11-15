use super::*;

mod fixture;
mod sealed;

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn sample_policy() -> Tpm2SealedPolicy {
    let mut public_object = vec![0u8; 80];
    public_object[..12].copy_from_slice(&[0, 78, 0, 8, 0, 11, 0, 0, 0, 18, 0, 32]);
    public_object[12..44].fill(1);
    public_object[44..48].copy_from_slice(&[0, 16, 0, 32]);
    Tpm2SealedPolicy {
        pcrs: vec![16],
        public_object,
        persistent_handle: 0x8100_0010,
        provider_class: Tpm2ProviderClass::SwtpmWorkspace,
        provider_ref: reference("disposable-swtpm-fixture"),
        executable_ref: reference("tool-fixture"),
    }
}

#[test]
fn policy_rejects_password_bypass_and_binds_exact_backend() {
    let policy = sample_policy();
    policy.validate().expect("public policy shape");
    let expected = backend_ref(&policy).expect("backend identity");
    let mut changed = policy.clone();
    changed.public_object[9] |= 0x40;
    assert_eq!(changed.validate(), Err(Tpm2IdentityFailure::InvalidPolicy));
    changed = policy.clone();
    changed.pcrs = vec![16, 16];
    assert_eq!(changed.validate(), Err(Tpm2IdentityFailure::InvalidPolicy));
    changed = policy.clone();
    changed.pcrs = vec![17];
    assert_ne!(backend_ref(&changed).expect("different selection"), expected);
    changed = policy.clone();
    changed.public_object[79] ^= 1;
    assert_ne!(backend_ref(&changed).expect("different object"), expected);
    changed = policy.clone();
    changed.provider_class = Tpm2ProviderClass::LinuxResourceManager;
    assert_ne!(backend_ref(&changed).expect("different provider kind"), expected);
    changed = policy;
    changed.executable_ref = reference("other tool");
    assert_ne!(backend_ref(&changed).expect("different tool"), expected);
}

fn rotation_request(current: &OpaqueKeyHandle, destination: &OpaqueKeyHandle) -> KeyRotationRequest {
    KeyRotationRequest {
        operation_id: "rotate-transport".to_string(),
        profile_ref: current.profile_ref.clone(),
        purpose: KeyPurpose::TransportEndpoint,
        backend_class: destination.backend_class,
        backend_ref: destination.backend_ref.clone(),
        old_handle_ref: current.handle_ref.clone(),
        old_public_key_ref: current.public_key_ref.clone(),
        old_generation: current.generation,
        new_generation: destination.generation,
        policy_ref: reference("rotation-policy"),
        activation_boundary_ref: reference("rotation-activation"),
        overlap: RotationOverlapPolicy::None,
        revocation_evidence_ref: Some(reference("old-key-revocation")),
    }
}

fn fixture_tool(
    executable: &crate::node_state::Executable,
    workspace: &crate::node_state::Workspace,
    arguments: &[&str],
    input: bounded_exec::Input,
) {
    let mut arguments: Vec<String> = arguments.iter().map(|s| (*s).to_string()).collect();
    arguments.extend([
        "-T".to_string(),
        format!("swtpm:path={}/swtpm.sock", workspace.process_path().expect("workspace").display()),
    ]);
    process::run(executable, workspace, arguments, input, 4_096).expect("bounded fixture TPM operation");
}

fn assert_no_plaintext(scope: &crate::node_state::Scope, forbidden: &[(&str, &[u8])]) {
    let mut pending = Vec::with_capacity(64);
    let mut inspected = inspect_plaintext(scope, forbidden, &mut pending);
    for _ in 0..64 {
        let Some(child) = pending.pop() else {
            assert!(inspected > 0, "plaintext check requires consumer-owned regular files");
            return;
        };
        inspected += inspect_plaintext(&child, forbidden, &mut pending);
    }
    assert!(pending.is_empty(), "fixture directory inventory exceeds its bound");
    assert!(inspected > 0, "plaintext check requires consumer-owned regular files");
}

fn inspect_plaintext(
    scope: &crate::node_state::Scope,
    forbidden: &[(&str, &[u8])],
    pending: &mut Vec<crate::node_state::Scope>,
) -> usize {
    let mut inspected = 0;
    for entry in scope.list_entries().expect("bounded fixture inventory") {
        match entry.kind {
            crate::node_state::EntryKind::Directory => {
                let child = scope.existing_subdir(&entry.path).expect("fixture child").expect("existing child");
                assert!(pending.len() < 64, "fixture pending directory inventory exceeds its bound");
                pending.push(child);
            }
            crate::node_state::EntryKind::RegularFile => {
                let bytes =
                    scope.read_entry(&entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES).expect("fixture file");
                inspected += 1;
                for (label, secret) in forbidden {
                    assert!(
                        !bytes.windows(secret.len()).any(|window| window == *secret),
                        "consumer-owned file {} contains {label}; contents redacted",
                        entry.path.display(),
                    );
                }
            }
            _ => {} // Only fixture-owned Unix sockets may be non-regular.
        }
    }
    inspected
}

// r[verify molten.crypto_identity.tpm2_backend_rotation]
#[test]
fn generation_and_rotation_deny_wrong_purpose_and_destination() {
    let profile = super::super::profile(&production_ed25519_profile(reference("profile"), reference("entropy")))
        .expect("production profile");
    let mut generation = KeyGenerationRequest {
        operation_id: "generate-transport".to_string(),
        profile_ref: profile.profile.profile_ref.clone(),
        purpose: KeyPurpose::TransportEndpoint,
        backend_class: KeyBackendClass::Tpm2Sealed,
        backend_ref: reference("sealed-backend"),
        entropy_profile_ref: reference("entropy"),
        generation: 1,
        policy_ref: reference("policy"),
        permit_first_boot_generation: true,
    };
    assert!(admit_key_generation(&profile.profile, &generation).is_ok());
    generation.purpose = KeyPurpose::Authority;
    assert!(
        admit_key_generation(&profile.profile, &generation)
            .expect_err("signing purpose denied")
            .contains(&CryptoIdentityIssue::UnsupportedPurpose(KeyPurpose::Authority))
    );
    let make_handle = |backend_class, backend_ref: &str, generation| {
        key_handle(KeyHandleInput {
            profile: &profile,
            purpose: KeyPurpose::TransportEndpoint,
            generation,
            public_key_ref: &reference(&format!("public-{generation}")),
            backend_class,
            backend_ref,
            currentness: KeyCurrentness::Current,
            currentness_evidence_ref: &reference("currentness"),
        })
        .expect("canonical public test handle")
        .handle
    };
    let source = make_handle(KeyBackendClass::CapabilityFile, &reference("file"), 1);
    let destination = make_handle(KeyBackendClass::Tpm2Sealed, &reference("sealed"), 2);
    let request = rotation_request(&source, &destination);
    let plan = plan_key_rotation(&profile.profile, &source, &request).expect("cross-backend plan");
    assert!(complete_key_rotation(&plan, &destination).is_ok());
    let mut mismatched = destination.clone();
    mismatched.backend_ref = reference("other PCR policy");
    assert!(complete_key_rotation(&plan, &mismatched).is_err());
    mismatched = destination.clone();
    mismatched.backend_class = KeyBackendClass::CapabilityFile;
    assert!(complete_key_rotation(&plan, &mismatched).is_err());
    let mut stale = request;
    stale.old_handle_ref = reference("stale source");
    assert!(plan_key_rotation(&profile.profile, &source, &stale).is_err());
    let file_destination = make_handle(KeyBackendClass::CapabilityFile, &reference("next-file"), 3);
    let reverse = rotation_request(&destination, &file_destination);
    let reverse_plan = plan_key_rotation(&profile.profile, &destination, &reverse).expect("reverse plan");
    assert!(complete_key_rotation(&reverse_plan, &file_destination).is_ok());
}
