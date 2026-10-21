fn publication_request() -> WorldMergeRequest {
    let artifact = WorldMergeValue {
        root: Some(root(RootKind::Artifact, "artifact")),
        schema_ref: Some(schema()),
        available: true,
        canonical_bytes: Some(b"artifact".to_vec()),
        keyed_values: BTreeMap::new(),
    };
    let keyed = |label: &str, entries: &[(&str, &str)]| WorldMergeValue {
        root: Some(root(RootKind::DurableState, label)),
        schema_ref: Some(schema()),
        available: true,
        canonical_bytes: None,
        keyed_values: entries.iter().map(|(key, value)| ((*key).to_string(), value.as_bytes().to_vec())).collect(),
    };
    WorldMergeRequest {
        base_head: commit("base"),
        source_heads: vec![commit("left"), commit("right")],
        common_ancestor_verified: true,
        common_ancestor_ambiguous: false,
        roots: vec![
            WorldMergeRootInput {
                kind: RootKind::Artifact,
                base: artifact.clone(),
                left: artifact.clone(),
                right: artifact,
            },
            WorldMergeRootInput {
                kind: RootKind::DurableState,
                base: keyed("base-state", &[("a", "base-a"), ("b", "base-b")]),
                left: keyed("left-state", &[("a", "left-a"), ("b", "base-b")]),
                right: keyed("right-state", &[("a", "base-a"), ("b", "right-b")]),
            },
        ],
        profile: WorldMergeProfile {
            profile_ref: WorldMergeProfileRef::new(reference("profile")).expect("profile"),
            policy_ref: WorldMergePolicyRef::new(reference("policy")).expect("policy"),
            root_modes: BTreeMap::from([
                (RootKind::Artifact, WorldMergeMode::IdenticalOnly),
                (RootKind::DurableState, WorldMergeMode::KeyedDurableValues),
            ]),
            migrations: BTreeMap::new(),
            handlers: BTreeMap::new(),
        },
        bounds: WorldMergeBounds::standard(),
    }
}

fn plan_with_generated_output() -> WorldMergePlan {
    plan_world_merge(publication_request(), Vec::new(), None).expect("admitted generated-output plan")
}

#[test]
fn changed_plan_output_denies_before_any_publication_effect() {
    let mut plan = plan_with_generated_output();
    let durable = plan
        .outputs
        .iter_mut()
        .find(|output| output.kind == RootKind::DurableState)
        .expect("durable output");
    durable.generated_values.insert("injected".to_string(), b"not planned".to_vec());
    assert!(validate_world_merge_plan(&plan).is_err());
    let policy = WorldMergePolicyRef::new(reference("policy")).expect("policy");
    let mut objects = TestObjects::default();
    let mut conflicts = TestConflicts::default();
    let mut authority = TestAuthority { calls: 0 };
    let mut commits = TestCommits::default();
    let result = publish_world_merge(
        &mut objects,
        &mut conflicts,
        &mut authority,
        &mut commits,
        &WorldMergePublicationRequest {
            plan: &plan,
            policy_ref: &policy,
        },
    );
    assert!(result.is_err());
    assert!(objects.persisted.is_empty());
    assert!(conflicts.records.is_empty());
    assert_eq!(authority.calls, 0);
    assert_eq!(commits.calls, 0);
    assert!(commits.observed_roots.is_empty());
}

#[test]
fn generated_payload_presence_tampering_denies_before_publication() {
    // r[verify molten.audit_f06.publication]
    // r[verify molten.audit_f06.validation]
    let mut plan = plan_with_generated_output();
    let output = plan
        .outputs
        .iter_mut()
        .find(|output| output.kind == RootKind::DurableState)
        .expect("generated durable output");
    assert_eq!(output.generated_bytes, None);
    let original_root = canonical_generated_world_root(output).expect("original root").0;
    output.generated_bytes = Some(Vec::new());
    let changed_root = canonical_generated_world_root(output).expect("changed root").0;
    assert_ne!(original_root, changed_root, "payload presence changes the persisted root identity");

    let policy = WorldMergePolicyRef::new(reference("policy")).expect("policy");
    let mut objects = TestObjects::default();
    let mut conflicts = TestConflicts::default();
    let mut authority = TestAuthority { calls: 0 };
    let mut commits = TestCommits::default();
    let result = publish_world_merge(
        &mut objects,
        &mut conflicts,
        &mut authority,
        &mut commits,
        &WorldMergePublicationRequest {
            plan: &plan,
            policy_ref: &policy,
        },
    );
    assert!(result.is_err(), "unsealed payload presence must not reach publication");
    assert!(objects.persisted.is_empty());
    assert!(conflicts.records.is_empty());
    assert_eq!(authority.calls, 0);
    assert_eq!(commits.calls, 0);
    assert!(commits.observed_roots.is_empty());
}
