
#[cfg(test)]
mod tests {
    const COMMIT_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn world_operator_closed_commands_parse_explicit_request_and_plan_output() {
        let commands = [
            "inspect",
            "checkpoint",
            "branch",
            "run",
            "diff",
            "conflicts",
            "replay",
            "simulate",
            "verify",
            "promote",
            "export",
            "import",
            "gc-plan",
        ];
        for command in commands {
            let cli = <super::Cli as clap::Parser>::try_parse_from([
                "molten",
                "world",
                command,
                "--request",
                "workflow.json",
                "--plan-out",
                "workflow-plan.preserves",
            ])
            .expect("world operator command");
            assert!(matches!(cli.command, Some(super::Top::World { .. })));
        }
    }

    #[test]
    fn world_operator_rejects_missing_request() {
        let result = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world",
            "inspect",
            "--plan-out",
            "workflow-plan.preserves",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn world_commit_operator_commands_parse_explicit_state_and_identity() {
        let cli = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-commit",
            "plan-restore",
            "--state-root",
            "state",
            COMMIT_REF,
            "--out",
            "restore.preserves",
        ])
        .expect("world commit command");

        assert!(matches!(cli.command, Some(super::Top::WorldCommit { .. })));
    }

    #[test]
    fn world_commit_operator_commands_reject_missing_state_root() {
        let result = <super::Cli as clap::Parser>::try_parse_from(["molten", "world-commit", "inspect", COMMIT_REF]);

        assert!(result.is_err());
    }

    #[test]
    fn world_snapshot_clone_plan_parses_explicit_descriptor_bound_and_output() {
        let cli = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-snapshot",
            "clone-plan",
            "--descriptor",
            "snapshot.preserves",
            "--children",
            "2",
            "--out",
            "clone.preserves",
        ])
        .expect("world snapshot clone plan command");

        assert!(matches!(cli.command, Some(super::Top::WorldSnapshot { .. })));
    }

    #[test]
    fn world_snapshot_restore_rejects_missing_denial_receipt_path() {
        let result = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-snapshot",
            "restore",
            "--descriptor",
            "snapshot.preserves",
            "--destination",
            "destination.preserves",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn world_authority_plan_parses_bounded_request_policy_and_output() {
        let cli = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-authority",
            "plan",
            "--request",
            "authority-request.json",
            "--policy",
            "branch-policy.json",
            "--out",
            "authority-receipt.json",
        ])
        .expect("world authority plan command");

        assert!(matches!(cli.command, Some(super::Top::WorldAuthority { .. })));
    }

    #[test]
    fn world_authority_effect_command_requires_denial_receipt_path() {
        let result = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-authority",
            "activate",
            "--request",
            "authority-request.json",
            "--policy",
            "branch-policy.json",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn world_head_plan_parses_every_compare_and_swap_input() {
        let cli = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-head",
            "plan",
            "--branch",
            "main",
            "--expected-head",
            COMMIT_REF,
            "--successor-head",
            COMMIT_REF,
            "--expected-generation",
            "1",
            "--successor-generation",
            "2",
            "--purpose",
            "advance",
            "--policy-ref",
            COMMIT_REF,
            "--out",
            "claim.preserves",
        ])
        .expect("world-head plan command");

        assert!(matches!(cli.command, Some(super::Top::WorldHead { .. })));
    }

    #[test]
    fn world_head_mutation_commands_reject_missing_capability_root() {
        let result = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-head",
            "advance",
            "--claim",
            "claim.preserves",
            "--signature",
            "signature.json",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn world_distribution_plan_parses_bounded_identity_and_output() {
        let cli = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-distribution",
            "sync-plan",
            "--state-root",
            "state",
            "--commit",
            COMMIT_REF,
            "--epoch-ref",
            COMMIT_REF,
            "--policy-ref",
            COMMIT_REF,
            "--generation",
            "1",
            "--assume-missing",
            "--out",
            "world-sync.preserves",
        ])
        .expect("world distribution plan command");

        assert!(matches!(cli.command, Some(super::Top::WorldDistribution { .. })));
    }

    #[test]
    fn world_distribution_sync_rejects_missing_capability_root() {
        let result = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-distribution",
            "sync",
            "--commit",
            COMMIT_REF,
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn world_promotion_plan_requires_explicit_request_and_output() {
        let cli = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-promotion",
            "plan",
            "--request",
            "promotion.json",
            "--out",
            "promotion.preserves",
        ])
        .expect("world promotion plan command");

        assert!(matches!(cli.command, Some(super::Top::WorldPromotion { .. })));
    }

    #[test]
    fn world_promotion_mutation_rejects_missing_capability_root() {
        let result = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-promotion",
            "promote",
            "--request",
            "promotion.json",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn world_merge_plan_parses_explicit_base_sources_and_policy() {
        let cli = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-merge",
            "merge-plan",
            "--state-root",
            "state",
            "--base",
            COMMIT_REF,
            "--left",
            COMMIT_REF,
            "--right",
            COMMIT_REF,
            "--profile-ref",
            COMMIT_REF,
            "--policy-ref",
            COMMIT_REF,
            "--out",
            "merge.preserves",
        ])
        .expect("world-merge plan command");

        assert!(matches!(cli.command, Some(super::Top::WorldMerge { .. })));
    }

    #[test]
    fn world_merge_publish_rejects_missing_capability_root() {
        let result = <super::Cli as clap::Parser>::try_parse_from([
            "molten",
            "world-merge",
            "merge-publish",
            "--plan",
            "merge.preserves",
        ]);

        assert!(result.is_err());
    }
}
