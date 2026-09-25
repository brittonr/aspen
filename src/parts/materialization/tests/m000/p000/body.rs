    use super::*;

    const SMALL_MAX_MEMBERS: usize = 8;
    const SMALL_MAX_MEMBER_BYTES: u64 = 1_024;
    const SMALL_MAX_TOTAL_BYTES: u64 = 4_096;
    const SMALL_MAX_PATH_BYTES: u64 = 128;

    fn policy(replacement: ReplacementPolicy) -> MaterializationPolicy {
        MaterializationPolicy::bounded("test-bundle-v1", replacement)
            .expect("base policy")
            .with_bounds(
                u64::try_from(SMALL_MAX_MEMBERS).expect("member bound fits u64"),
                SMALL_MAX_MEMBER_BYTES,
                SMALL_MAX_TOTAL_BYTES,
                SMALL_MAX_PATH_BYTES,
            )
            .expect("bounded policy")
    }

    fn payloads() -> Vec<MaterializationPayload> {
        vec![
            MaterializationPayload::new("nested/b.preserves", b"bravo".to_vec()),
            MaterializationPayload::new("a.txt", b"alpha".to_vec()),
        ]
    }

    #[test]
    fn pure_plan_is_order_independent_and_portable() {
        // r[verify molten.filesystem_materialization.plan]
        // r[verify molten.filesystem_materialization.receipt]
        let policy = policy(ReplacementPolicy::NoReplace);
        let first_payloads = payloads();
        let mut reversed = first_payloads.clone();
        reversed.reverse();
        let first = plan_payloads(&policy, &first_payloads).expect("first plan");
        let second = plan_payloads(&policy, &reversed).expect("second plan");
        assert_eq!(first, second);
        assert_eq!(first.members[0].logical_path.as_str(), "a.txt");
        let mut first_policy_order = policy.clone();
        first_policy_order
            .reserved_top_level_names
            .extend(["z-reserved".to_string(), "a-reserved".to_string()]);
        let mut second_policy_order = first_policy_order.clone();
        second_policy_order.reserved_top_level_names.reverse();
        assert_eq!(
            plan_payloads(&first_policy_order, &first_payloads).expect("first reserved order"),
            plan_payloads(&second_policy_order, &first_payloads).expect("second reserved order")
        );

        let first_workspace =
            crate::test_support::process_workspace("materialize_portable_first").expect("first workspace");
        let second_workspace =
            crate::test_support::process_workspace("materialize_portable_second").expect("second workspace");
        let first_receipt =
            materialize_path(&first_workspace, &policy, &first_payloads).expect("first materialization");
        let second_receipt =
            materialize_path(&second_workspace, &policy, &first_payloads).expect("second materialization");
        assert_eq!(first_receipt.receipt_ref, second_receipt.receipt_ref);
        assert_eq!(first_receipt.decision, DECISION_PASS);
        assert!(first_receipt.non_claims.iter().any(|claim| claim == "not-release-eligibility"));
    }

    #[test]
    fn planner_rejects_unsafe_duplicate_reserved_and_over_bound_members() {
        // r[verify molten.filesystem_materialization.validation]
        let policy = policy(ReplacementPolicy::NoReplace);
        for path in [
            "",
            "/absolute",
            "../parent",
            "a/../parent",
            "a//ambiguous",
            "a\\windows",
            "C:/prefixed",
            ".molten-materialize/member",
        ] {
            let input = MaterializationMemberInput {
                logical_path: path.to_string(),
                kind: MaterializationMemberKind::RegularFile,
                expected_content_ref: crate::preserves_rail::content_ref_from_bytes(b"x"),
                expected_size: 1,
            };
            assert!(plan_materialization(&policy, &[input]).is_err(), "unsafe path accepted: {path}");
        }
        assert!(plan_payloads(&policy, &[]).is_err());
        let duplicate = MaterializationPayload::new("same", b"one".to_vec());
        assert!(plan_payloads(&policy, &[duplicate.clone(), duplicate]).is_err());
        let unsupported = MaterializationMemberInput {
            logical_path: "link".to_string(),
            kind: MaterializationMemberKind::Symlink,
            expected_content_ref: crate::preserves_rail::content_ref_from_bytes(b"x"),
            expected_size: 1,
        };
        assert!(plan_materialization(&policy, &[unsupported]).is_err());
        let oversized =
            MaterializationPayload::new("large", vec![
                0;
                usize::try_from(SMALL_MAX_MEMBER_BYTES).expect("small bound") + 1
            ]);
        assert!(plan_payloads(&policy, &[oversized]).is_err());
        let hard_ceiling = MaterializationPolicy::bounded("hard-ceiling-test-v1", ReplacementPolicy::NoReplace)
            .expect("hard-ceiling base policy")
            .with_bounds(
                u64::try_from(HARD_MAX_MATERIALIZATION_MEMBERS.saturating_add(1)).expect("member bound fits u64"),
                DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES,
                DEFAULT_MAX_MATERIALIZATION_TOTAL_BYTES,
                u64::try_from(DEFAULT_MAX_MATERIALIZATION_PATH_BYTES).expect("path bound fits u64"),
            );
        assert!(hard_ceiling.is_err());
    }

    #[test]
    fn materialization_publishes_verified_receipts_and_replaces_only_when_selected() {
        // r[verify molten.filesystem_materialization.commit]
        // r[verify molten.filesystem_materialization.receipt]
        let root_path = crate::test_support::process_workspace("materialize_publish").expect("root");
        let root = MaterializationRoot::open(&root_path).expect("root capability");
        let payloads = payloads();
        let no_replace = policy(ReplacementPolicy::NoReplace);
        let plan = plan_payloads(&no_replace, &payloads).expect("plan");
        let receipt = root.materialize(&plan, &payloads).expect("materialize");
        assert!(receipt.valid());
        assert_eq!(receipt.plan_ref, plan.plan_ref);
        let receipt_text = crate::preserves_rail::to_text(&receipt.value).expect("receipt text");
        let reparsed_value = crate::preserves_rail::parse_text(&receipt_text).expect("receipt Preserves parse");
        let reparsed = parse_materialization_receipt(&reparsed_value).expect("typed receipt parse");
        assert_eq!(reparsed, receipt);
        let first_path =
            MaterializationPath::parse_within("a.txt", no_replace.max_path_bytes).expect("first payload path");
        assert_eq!(root.read(&first_path).expect("read"), b"alpha");

        let replacement = [MaterializationPayload::new("a.txt", b"replacement".to_vec())];
        let replace = policy(ReplacementPolicy::ReplaceRegularFiles);
        let replace_plan = plan_payloads(&replace, &replacement).expect("replacement plan");
        let replacement_receipt = root.materialize(&replace_plan, &replacement).expect("replace");
        assert!(replacement_receipt.valid());
        assert_eq!(std::fs::read(root_path.join("a.txt")).expect("replacement bytes"), b"replacement");

        let mut tampered_receipt = replacement_receipt;
        tampered_receipt.member_refs[0].1 = crate::preserves_rail::content_ref_from_bytes(b"tampered");
        assert!(!tampered_receipt.valid());
    }

    #[test]
    fn injected_mid_publication_failure_restores_replaced_members_without_a_receipt() {
        const FAIL_AFTER_FIRST_PUBLICATION: usize = 1;

        let root_path = crate::test_support::process_workspace("materialize_mid_commit_failure").expect("root");
        std::fs::write(root_path.join("a.txt"), b"old-a").expect("first original");
        let root = MaterializationRoot::open(&root_path).expect("root capability");
        let replacement = policy(ReplacementPolicy::ReplaceRegularFiles);
        let payloads = payloads();
        let plan = plan_payloads(&replacement, &payloads).expect("replacement plan");
        let staged = root.stage(&plan, &payloads).expect("stage");
        let result = root.commit_inner(&plan, &staged, Some(FAIL_AFTER_FIRST_PUBLICATION));
        assert!(result.is_err(), "fault injection must not return a passing receipt");
        assert_eq!(std::fs::read(root_path.join("a.txt")).expect("first restored"), b"old-a");
        assert!(!root_path.join("nested").exists(), "new destination directory must roll back");
        root.abort(&staged).expect("abort failed stage");
    }

    #[test]
    fn staged_commit_denies_wrong_root_stale_plan_partial_bytes_and_replacement() {
        // r[verify molten.filesystem_materialization.commit]
        // r[verify molten.filesystem_materialization.validation]
        let no_replace = policy(ReplacementPolicy::NoReplace);
        let payloads = payloads();
        let plan = plan_payloads(&no_replace, &payloads).expect("plan");
        let first_path = crate::test_support::process_workspace("materialize_stage_first").expect("first root");
        let second_path = crate::test_support::process_workspace("materialize_stage_second").expect("second root");
        let first = MaterializationRoot::open(&first_path).expect("first root");
        let second = MaterializationRoot::open(&second_path).expect("second root");
        let mut field_tampered_plan = plan.clone();
        field_tampered_plan.total_bytes = field_tampered_plan.total_bytes.saturating_add(1);
        assert!(first.stage(&field_tampered_plan, &payloads).is_err());
        let staged = first.stage(&plan, &payloads).expect("stage");
        assert!(second.commit(&plan, &staged).is_err());
        let other_plan =
            plan_payloads(&no_replace, &[MaterializationPayload::new("other", b"other".to_vec())]).expect("other plan");
        assert!(first.commit(&other_plan, &staged).is_err());
        first.abort(&staged).expect("abort stale stage");

        let mut tampered = payloads.clone();
        tampered[1].bytes = b"tampered".to_vec();
        assert!(first.stage(&plan, &tampered).is_err());
        assert!(!first.inner.dir.try_exists(stage_path(&plan).expect("stage path")).expect("stage absence"));

        std::fs::write(first_path.join("a.txt"), b"existing").expect("existing destination");
        assert!(first.materialize(&plan, &payloads).is_err());
        assert_eq!(std::fs::read(first_path.join("a.txt")).expect("existing survives"), b"existing");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_parent_and_leaf_cannot_redirect_materialization() {
        // r[verify molten.filesystem_materialization.root]
        let policy = policy(ReplacementPolicy::ReplaceRegularFiles);
        let outside = crate::test_support::process_workspace("materialize_outside").expect("outside root");
        let target = crate::test_support::process_workspace("materialize_symlink_target").expect("target root");
        std::fs::write(outside.join("outside.bin"), b"outside").expect("outside fixture");
        std::os::unix::fs::symlink(&*outside, target.join("linked-parent")).expect("parent symlink");
        let parent_payload = [MaterializationPayload::new(
            "linked-parent/outside.bin",
            b"overwrite".to_vec(),
        )];
        assert!(materialize_path(&target, &policy, &parent_payload).is_err());
        assert_eq!(std::fs::read(outside.join("outside.bin")).expect("outside survives"), b"outside");

        std::os::unix::fs::symlink(outside.join("outside.bin"), target.join("leaf.bin")).expect("leaf symlink");
        let leaf_payload = [MaterializationPayload::new("leaf.bin", b"overwrite".to_vec())];
        assert!(materialize_path(&target, &policy, &leaf_payload).is_err());
        assert_eq!(std::fs::read(outside.join("outside.bin")).expect("outside survives"), b"outside");
    }

    #[test]
    fn source_root_detects_tampered_member() {
        let source_path = crate::test_support::process_workspace("materialize_source").expect("source root");
        let payloads = payloads();
        for payload in &payloads {
            let path = source_path.join(&payload.logical_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("source parent");
            }
            std::fs::write(path, &payload.bytes).expect("source member");
        }
        let policy = policy(ReplacementPolicy::NoReplace);
        let plan = plan_payloads(&policy, &payloads).expect("source plan");
        let source = SourceDirectoryRoot::open_existing(&source_path).expect("source capability");
        assert_eq!(source.read_payloads(&policy, &plan).expect("source payloads").len(), payloads.len());
        std::fs::write(source_path.join("a.txt"), b"tampered").expect("tamper source");
        assert!(source.read_payloads(&policy, &plan).is_err());
        std::fs::remove_file(source_path.join("nested/b.preserves")).expect("remove required source member");
        assert!(source.read_payloads(&policy, &plan).is_err());
    }

    #[test]
    fn source_listing_admits_the_member_bound_and_denies_one_past() {
        let source_path = crate::test_support::process_workspace("materialize_source_bound").expect("source root");
        for index in 0..SMALL_MAX_MEMBERS {
            std::fs::write(source_path.join(format!("member-{index}")), b"member").expect("source member");
        }
        let policy = policy(ReplacementPolicy::NoReplace);
        let source = SourceDirectoryRoot::open_existing(&source_path).expect("source capability");
        let listed = source.list_regular_files_recursive(&policy).expect("member bound admits the listing");
        assert_eq!(listed.len(), SMALL_MAX_MEMBERS);

        std::fs::write(source_path.join("member-past-bound"), b"member").expect("one-past source member");
        assert!(source.list_regular_files_recursive(&policy).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn source_capability_survives_root_replacement_and_rejects_links() {
        let source_path = crate::test_support::process_workspace("materialize_source_anchor").expect("source root");
        std::fs::write(source_path.join("member"), b"anchored").expect("member fixture");
        let source = SourceDirectoryRoot::open_existing(&source_path).expect("source capability");
        let moved = source_path.with_extension("moved");
        std::fs::rename(&source_path, &moved).expect("replace source root");
        std::fs::create_dir(&*source_path).expect("replacement source root");
        std::fs::write(source_path.join("member"), b"substitute").expect("substitute member");
        let policy = policy(ReplacementPolicy::NoReplace);
        let member = MaterializationPath::parse_within("member", policy.max_path_bytes).expect("member path");
        assert_eq!(source.read_path(&member, policy.max_member_bytes).expect("anchored read"), b"anchored");

        std::os::unix::fs::symlink(moved.join("member"), moved.join("linked")).expect("source link");
        assert!(source.list_regular_files_recursive(&policy).is_err());
    }
