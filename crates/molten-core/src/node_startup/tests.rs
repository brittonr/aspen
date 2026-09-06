use super::*;

fn fixture() -> (TrustedCohort, Descriptor) {
    let hash = blake3::hash(b"fixture").to_hex().to_string();
    let cohort = Cohort {
        source_revision: "a".repeat(40),
        source_inventory_blake3: hash.clone(),
        executable_blake3: "b".repeat(64),
        build_rustc_blake3: "c".repeat(64),
        build_toolchain: "nightly-2026-05-26".into(),
        octet_revision: OCTET_REVISION.into(),
        octet_cli_blake3: "e".repeat(64),
        octet_driver_blake3: "f".repeat(64),
        octet_lints_blake3: "1".repeat(64),
        octet_rustc_blake3: "2".repeat(64),
        octet_toolchain: "nightly-2026-03-21-x86_64-unknown-linux-gnu".into(),
    };
    (
        TrustedCohort {
            schema: POLICY_SCHEMA.into(),
            descriptor_blake3: hash.clone(),
            cohort: cohort.clone(),
        },
        Descriptor {
            schema: BUNDLE_SCHEMA.into(),
            cohort,
            members: ROLES
                .into_iter()
                .map(|role| Member {
                    role,
                    blake3: hash.clone(),
                    bytes: 7,
                })
                .collect(),
        },
    )
}

// r[verify molten.startup_evidence.inputs]
#[test]
fn exact_plan_and_measured_members() {
    let (policy, descriptor) = fixture();
    policy.check_descriptor_bytes(b"fixture").unwrap();
    let plan = EvidencePlan::admit(&policy, descriptor, &policy.cohort.executable_blake3).unwrap();
    assert_eq!(plan.members().len(), 10);
    for index in 0..10 {
        plan.verify_member(index, b"fixture").unwrap();
    }
    assert_eq!(plan.verify_member(10, b"fixture"), Err(Rejection::MemberInventory));
    assert_eq!(plan.verify_member(0, b"changed"), Err(Rejection::MemberIdentity));
    assert_eq!(plan.verify_member(0, b"fixture!"), Err(Rejection::MemberIdentity));
}

#[test]
fn policy_and_descriptor_deny_before_member_plan() {
    let (mut policy, _) = fixture();
    assert_eq!(policy.check_descriptor_bytes(b"changed"), Err(Rejection::DescriptorIdentity));
    assert_eq!(policy.check_descriptor_bytes(b""), Err(Rejection::Bounds));
    assert_eq!(policy.check_descriptor_bytes(&vec![0; MAX_DESCRIPTOR_BYTES + 1]), Err(Rejection::Bounds));
    policy.cohort.octet_toolchain = "nightly-test-toolchain".into();
    assert_eq!(policy.validate(), Err(Rejection::Policy));
    policy.cohort.octet_toolchain = "nightly-2026-03-21-x86_64-unknown-linux-gnu".into();
    policy.cohort.executable_blake3 = "AB".repeat(32);
    assert_eq!(policy.validate(), Err(Rejection::Policy));
}

#[test]
fn membership_order_and_bounds_are_exact() {
    let (policy, descriptor) = fixture();
    let admit = |d| EvidencePlan::admit(&policy, d, &policy.cohort.executable_blake3).err();
    let mut d = descriptor.clone();
    d.members.pop();
    assert_eq!(admit(d), Some(Rejection::MemberInventory));
    let mut d = descriptor.clone();
    d.members.push(d.members[0].clone());
    assert_eq!(admit(d), Some(Rejection::MemberInventory));
    let mut d = descriptor.clone();
    d.members.swap(0, 1);
    assert_eq!(admit(d), Some(Rejection::MemberInventory));
    let mut d = descriptor.clone();
    d.members[0].bytes = 0;
    assert_eq!(admit(d), Some(Rejection::Bounds));
    let mut d = descriptor.clone();
    d.members[0].bytes = MAX_MEMBER_BYTES + 1;
    assert_eq!(admit(d), Some(Rejection::Bounds));
    let mut d = descriptor;
    for m in &mut d.members {
        m.bytes = MAX_MEMBER_BYTES;
    }
    assert_eq!(admit(d), Some(Rejection::Bounds));
}

#[test]
fn source_binary_and_tool_roles_do_not_alias() {
    let (policy, descriptor) = fixture();
    assert_eq!(EvidencePlan::admit(&policy, descriptor.clone(), &"a".repeat(64)).err(), Some(Rejection::Executable));
    let mut changed = descriptor.clone();
    changed.cohort.source_revision = "b".repeat(40);
    assert_eq!(
        EvidencePlan::admit(&policy, changed, &policy.cohort.executable_blake3).err(),
        Some(Rejection::Cohort)
    );
    let mut changed = descriptor.clone();
    changed.cohort.octet_rustc_blake3 = policy.cohort.build_rustc_blake3.clone();
    assert_eq!(
        EvidencePlan::admit(&policy, changed, &policy.cohort.executable_blake3).err(),
        Some(Rejection::Cohort)
    );
    let mut changed = descriptor;
    changed.members[5].blake3 = "f".repeat(64);
    assert_eq!(
        EvidencePlan::admit(&policy, changed, &policy.cohort.executable_blake3).err(),
        Some(Rejection::SourceInventory)
    );
}

fn source_fixture(plan: &EvidencePlan) -> Vec<SourceFile> {
    let mut files: Vec<_> = plan.members()[..5]
        .iter()
        .map(|m| SourceFile {
            name: m.role.filename().into(),
            blake3: m.blake3.clone(),
            bytes: m.bytes,
        })
        .collect();
    for name in [
        "src/job/dag.rs",
        "src/main.rs",
        "src/node/daemon.rs",
        "src/node/runtime.rs",
        "src/octet/gate.rs",
        "src/upgrades/mod.rs",
        "src/node/content.rs",
        "src/node/parts/daemon/p018/body.rs",
        "src/node/parts/daemon/p019/body.rs",
        "crates/molten-core/src/content_store_adapter/node_service.rs",
        "crates/molten-core/src/node_startup.rs",
        "src/octet/startup_snapshot.rs",
        "src/node/startup_evidence.rs",
    ] {
        files.push(SourceFile {
            name: name.into(),
            blake3: "c".repeat(64),
            bytes: 1,
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    files
}

#[test]
fn source_context_and_raw_inventory_are_bound() {
    let (policy, descriptor) = fixture();
    let plan = EvidencePlan::admit(&policy, descriptor, &policy.cohort.executable_blake3).unwrap();
    let files = source_fixture(&plan);
    validate_source_inventory(&plan, &files).unwrap();
    let mut changed = files.clone();
    changed[0].blake3 = "f".repeat(64);
    assert_eq!(validate_source_inventory(&plan, &changed), Err(Rejection::SourceContext));
    let mut changed = files.clone();
    changed.pop();
    assert_eq!(validate_source_inventory(&plan, &changed), Err(Rejection::SourceContext));
    let mut changed = files.clone();
    changed.insert(0, changed[0].clone());
    assert_eq!(validate_source_inventory(&plan, &changed), Err(Rejection::SourceInventory));
    for name in ["/absolute", "../outside", "a/../b", "a//b", "a\\b", "a b"] {
        let mut changed = files.clone();
        changed[0].name = name.into();
        assert_eq!(validate_source_inventory(&plan, &changed), Err(Rejection::SourceInventory));
    }
}
