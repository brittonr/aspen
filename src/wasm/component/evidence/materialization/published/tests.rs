use super::*;

fn measure_for_test(
    root: &cap_std::fs::Dir,
    members: &[RequiredMember],
    component: &str,
    wit: &str,
    profile: &crate::wasm_component::RuntimeProfile,
) -> Result<measurement::MeasuredMembers> {
    staging::with_private_stage(root, "private-stage", |stage| {
        measurement::measure_members(root, stage, members, measurement::Selection {
            component_path: component,
            wit_path: wit,
            profile,
        })
    })
}

fn member(role: &'static str, path: &str, bytes: &[u8]) -> RequiredMember {
    RequiredMember {
        role,
        logical_path: path.to_string(),
        digest: blake3::hash(bytes).to_hex().to_string(),
        size: bytes.len() as u64,
    }
}

#[test]
fn every_role_is_measured_and_executable_snapshot_is_retained() {
    let workspace = crate::test_support::Workspace::new("published-materialization-matching").expect("workspace");
    let root = workspace.input().expect("input capability");
    root.dir().create_dir("members").expect("member directory");
    let roles = [
        "source-closure",
        "lock",
        "final-portable",
        "wit-input",
        "package-object",
        "stage-receipt",
        "stage-artifact",
    ];
    let mut members = Vec::with_capacity(roles.len() + 1);
    for role in roles {
        root.dir().write(format!("members/{role}"), role.as_bytes()).expect("member bytes");
        members.push(member(role, &format!("/members/{role}"), role.as_bytes()));
    }
    // The same path may support multiple roles only when both facts agree.
    members.push(member("stage-artifact", "/members/final-portable", b"final-portable"));
    let profile = crate::wasm_component::supported().expect("profile");
    let measured = measure_for_test(root.dir(), &members, "/members/final-portable", "/members/wit-input", &profile)
        .expect("all required bytes match");
    assert_eq!(measured.component_bytes, b"final-portable");
    assert_eq!(measured.wit_bytes, b"wit-input");
    let observed: Vec<_> = measured
        .observations
        .iter()
        .map(|observation| {
            (
                observation.role.as_str(),
                observation.digest_blake3.as_str(),
                observation.size_bytes,
                observation.status,
            )
        })
        .collect();
    let expected: Vec<_> = members
        .iter()
        .map(|member| {
            (
                member.role,
                member.digest.as_str(),
                member.size,
                Some(mantle_wasm_consumer_verifier::ConsumerMemberStatus::Matched),
            )
        })
        .collect();
    assert_eq!(observed, expected);
    root.dir().write("members/final-portable", b"changed later").expect("mutate source");
    assert_eq!(measured.component_bytes, b"final-portable");
}

#[test]
fn missing_or_drifted_non_executable_member_denies_the_whole_source() {
    let workspace = crate::test_support::Workspace::new("published-materialization-required").expect("workspace");
    let root = workspace.input().expect("input capability");
    root.dir().write("component", b"component").expect("component");
    root.dir().write("wit", b"wit").expect("WIT");
    let members = [
        member("final-portable", "/component", b"component"),
        member("wit-input", "/wit", b"wit"),
        member("package-object", "/package", b"package"),
    ];
    let profile = crate::wasm_component::supported().expect("profile");
    assert!(measure_for_test(root.dir(), &members, "/component", "/wit", &profile).is_err());
    root.dir().write("package", b"changed").expect("drifted package");
    assert!(measure_for_test(root.dir(), &members, "/component", "/wit", &profile).is_err());
    root.dir().write("package", b"package").expect("restore package");
    let mut conflict = member("stage-artifact", "/package", b"package");
    conflict.size += 1;
    let mut conflicting = Vec::with_capacity(members.len() + 1);
    conflicting.extend(members);
    conflicting.push(conflict);
    assert!(measure_for_test(root.dir(), &conflicting, "/component", "/wit", &profile).is_err());
}

#[test]
fn parent_and_leaf_symlinks_deny_even_matching_bytes() {
    let workspace = crate::test_support::Workspace::new("published-materialization-symlinks").expect("workspace");
    let root = workspace.input().expect("input capability");
    root.dir().create_dir("real").expect("real parent");
    root.dir().write("real/member", b"matching").expect("member");
    root.dir().symlink("real", "alias").expect("parent alias");
    root.dir().symlink("real/member", "leaf").expect("leaf alias");
    let digest = blake3::hash(b"matching").to_hex().to_string();
    let measure = |logical_path| {
        measurement::measure_member(
            root.dir(),
            measurement::FileIdentity {
                logical_path,
                digest: &digest,
                size: 8,
            },
            true,
            None,
        )
    };
    assert!(measure("/alias/member").is_err());
    assert!(measure("/leaf").is_err());
    for path in [
        "relative",
        "/../member",
        "/real/../real/member",
        "/real//member",
        "/real/./member",
        "/",
    ] {
        assert!(measure(path).is_err());
    }
}

#[test]
fn profile_byte_bound_precedes_member_read() {
    let workspace = crate::test_support::Workspace::new("published-materialization-bound").expect("workspace");
    let root = workspace.input().expect("input capability");
    root.dir().write("component", b"component").expect("component");
    root.dir().write("wit", b"wit").expect("WIT");
    let members = [
        member("final-portable", "/component", b"component"),
        member("wit-input", "/wit", b"wit"),
    ];
    let mut profile = crate::wasm_component::supported().expect("profile");
    profile.resources.max_component_bytes = 8;
    assert!(measure_for_test(root.dir(), &members, "/component", "/wit", &profile).is_err());
    profile.resources.max_component_bytes = 9;
    let measured = measure_for_test(root.dir(), &members, "/component", "/wit", &profile).expect("exact bound");
    assert_eq!(measured.component_bytes, b"component");
}

#[test]
fn structural_rejection_is_never_upgraded_by_available_bytes() {
    let workspace =
        crate::test_support::Workspace::new("published-materialization-structural-deny").expect("workspace");
    let root = workspace.input().expect("input capability");
    root.dir().write("member", b"present").expect("member");
    let object = serde_json::json!({
        "logical_path": "/member",
        "digest_blake3": blake3::hash(b"present").to_hex().to_string(),
        "size_bytes": 7,
    });
    let zero = "0".repeat(64);
    let malformed = serde_json::to_vec(&serde_json::json!({
        "schema": PUBLISHED_BUNDLE_SCHEMA,
        "name": "missing-required-stages",
        "manifest_blake3": zero,
        "cohort_blake3": zero,
        "wit_inputs": [object],
        "package_inputs": [],
        "source_closure": object,
        "lock": object,
        "final_portable": object,
        "expected_octet_profile_blake3": zero,
        "expected_runtime_profile_blake3": zero,
        "stage_receipts": [],
        "wizer": null,
        "aot": null,
        "non_claims": ["not-runtime-authority", "not-release-eligibility"],
        "bundle_identity_blake3": zero,
    }))
    .expect("actual published DTO shape");
    let bundle =
        decode_bundle(&malformed, mantle_wasm_consumer_verifier::verify_consumer_bundle).expect("producer DTO");
    let report = mantle_wasm_consumer_verifier::verify_consumer_bundle(bundle);
    assert_eq!(report.status, mantle_wasm_consumer_verifier::ConsumerVerificationStatus::Rejected);
    assert!(require_structural(&report).is_err());
}

#[test]
fn actual_producer_fixture_structural_pass_with_missing_bytes_is_blocked_and_denied() {
    let workspace = crate::test_support::Workspace::new("published-materialization-blocked-report").expect("workspace");
    let root = workspace.output().expect("private staging parent");
    let raw = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/wasm-component-runtime/kamacite-mantle-hello-bundle.json"
    ));
    let bundle =
        decode_bundle(raw, mantle_wasm_consumer_verifier::verify_consumer_bundle).expect("actual producer DTO");
    let structural = mantle_wasm_consumer_verifier::verify_consumer_bundle(bundle.clone());
    require_structural(&structural).expect("published real bundle structural pass");
    staging::with_private_stage(root.dir(), "private-stage", |stage| {
        let published = staging::consumer_root(stage)?;
        let complete = published.verify_bundle(bundle).expect("published shell reads actual DTO");
        assert_eq!(complete.status, mantle_wasm_consumer_verifier::ConsumerVerificationStatus::Blocked);
        assert!(
            complete
                .members
                .iter()
                .all(|member| member.status == Some(mantle_wasm_consumer_verifier::ConsumerMemberStatus::Missing))
        );
        assert!(require_complete(&structural, &complete, &[]).is_err());
        Ok(())
    })
    .expect("private stage cleanup");
    assert!(!root.dir().try_exists("private-stage").expect("lease removed"));
}

#[test]
fn private_staging_refuses_reused_roots_and_members_and_removes_only_its_child() {
    let workspace = crate::test_support::Workspace::new("published-materialization-staging-lease").expect("workspace");
    let parent = workspace.output().expect("staging parent");
    parent.dir().create_dir("existing").expect("existing directory");
    parent.dir().write("existing/keep", b"unrelated").expect("existing bytes");
    assert!(staging::with_private_stage(parent.dir(), "existing", |_| Ok(())).is_err());
    assert_eq!(parent.dir().read("existing/keep").expect("unrelated survives"), b"unrelated");
    staging::with_private_stage(parent.dir(), "owned", |stage| {
        let _file = staging::create_member(stage, "nested/member")?;
        assert!(staging::create_member(stage, "nested/member").is_err());
        Ok(())
    })
    .expect("owned child removed");
    assert!(!parent.dir().try_exists("owned").expect("owned child absent"));
}
