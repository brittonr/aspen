    use super::*;

    const WORKFLOW: &str = "release-evidence-fixture";

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn field(path: &str, value: &str, is_ref: bool) -> EvidenceField {
        EvidenceField {
            path: path.to_string(),
            value: value.to_string(),
            is_ref,
        }
    }

    fn summary(fields: Vec<EvidenceField>) -> EvidenceSummary {
        EvidenceSummary {
            workflow: WORKFLOW.to_string(),
            fields,
        }
    }

    #[test]
    fn equal_canonical_refs_pass() {
        let bundle_ref = local_ref("bundle");
        let promotion_ref = local_ref("promotion");
        let input = ComparisonInput {
            left: summary(vec![
                field("bundle", &bundle_ref, true),
                field("promotion", &promotion_ref, true),
            ]),
            right: summary(vec![
                field("bundle", &bundle_ref, true),
                field("promotion", &promotion_ref, true),
            ]),
            allowed_variances: Vec::new(),
        };
        let comparison = compare(&input).expect("drift comparison");
        assert_eq!(comparison.decision, "pass");
        assert!(comparison.diagnostics.is_empty());
        assert!(crate::preserves_rail::validate_content_ref(&comparison.receipt_ref).is_ok());
    }

    #[test]
    fn unexplained_ref_divergence_denies_first_difference() {
        let input = ComparisonInput {
            left: summary(vec![field("bundle", &local_ref("bundle-a"), true)]),
            right: summary(vec![field("bundle", &local_ref("bundle-b"), true)]),
            allowed_variances: Vec::new(),
        };
        let comparison = compare(&input).expect("drift comparison");
        assert_eq!(comparison.decision, "deny");
        assert_eq!(comparison.diagnostics[0].path, "bundle");
        assert_eq!(comparison.diagnostics[0].kind, "value-drift");
    }

    #[test]
    fn declared_volatile_field_is_normalized() {
        let stable_ref = local_ref("stable-release-gate");
        let input = ComparisonInput {
            left: summary(vec![
                field("release-gate", &stable_ref, true),
                field("tmp-root", "/tmp/a", false),
            ]),
            right: summary(vec![
                field("release-gate", &stable_ref, true),
                field("tmp-root", "/tmp/b", false),
            ]),
            allowed_variances: vec![AllowedVariance {
                path: "tmp-root".to_string(),
                reason: "temporary-root".to_string(),
            }],
        };
        let comparison = compare(&input).expect("drift comparison");
        assert_eq!(comparison.decision, "pass");
    }

    #[test]
    fn stale_variance_declaration_is_rejected() {
        let input = ComparisonInput {
            left: summary(vec![field("release-gate", &local_ref("gate"), true)]),
            right: summary(vec![field("release-gate", &local_ref("gate"), true)]),
            allowed_variances: vec![AllowedVariance {
                path: "missing-field".to_string(),
                reason: "temporary-root".to_string(),
            }],
        };
        let error = compare(&input).expect_err("stale variance must fail");
        assert!(error.to_string().contains("does not name a compared field"));
    }

    #[test]
    fn artifact_summary_uses_canonical_ref_not_rendered_text() {
        let artifact = crate::preserves_rail::record("fixture", vec![crate::preserves_rail::string("same")]);
        let left = artifact_summary(WORKFLOW, &artifact).expect("left artifact summary");
        let right = artifact_summary(WORKFLOW, &artifact).expect("right artifact summary");
        let comparison = compare(&ComparisonInput {
            left,
            right,
            allowed_variances: Vec::new(),
        })
        .expect("compare artifact summaries");
        assert_eq!(comparison.decision, "pass");
    }

    #[test]
    fn declared_variance_reasons_cover_only_explicit_volatile_fields() {
        let stable_ref = local_ref("stable-release-gate");
        let left = summary(vec![
            field("release-gate", &stable_ref, true),
            field("runtime-path", "/nix/store/left", false),
            field("diagnostic-log", "/tmp/left.log", false),
            field("store-path", "/nix/store/left-output", false),
            field("temporary-root", "/tmp/left", false),
            field("rendered-output", "left pretty text", false),
        ]);
        let right = summary(vec![
            field("release-gate", &stable_ref, true),
            field("runtime-path", "/nix/store/right", false),
            field("diagnostic-log", "/tmp/right.log", false),
            field("store-path", "/nix/store/right-output", false),
            field("temporary-root", "/tmp/right", false),
            field("rendered-output", "right pretty text", false),
        ]);
        let allowed_variances = vec![
            AllowedVariance {
                path: "runtime-path".to_string(),
                reason: "runtime-path".to_string(),
            },
            AllowedVariance {
                path: "diagnostic-log".to_string(),
                reason: "diagnostic-log".to_string(),
            },
            AllowedVariance {
                path: "store-path".to_string(),
                reason: "store-path".to_string(),
            },
            AllowedVariance {
                path: "temporary-root".to_string(),
                reason: "temporary-root".to_string(),
            },
            AllowedVariance {
                path: "rendered-output".to_string(),
                reason: "rendered-output".to_string(),
            },
        ];
        let comparison = compare(&ComparisonInput {
            left,
            right,
            allowed_variances,
        })
        .expect("declared variance comparison");
        assert_eq!(comparison.decision, "pass");
        assert!(comparison.diagnostics.is_empty());
    }

    #[test]
    fn ambient_state_variance_is_rejected_instead_of_masked() {
        let stable_ref = local_ref("stable-release-gate");
        let input = ComparisonInput {
            left: summary(vec![field("release-gate", &stable_ref, true), field("ambient-state", "left", false)]),
            right: summary(vec![field("release-gate", &stable_ref, true), field("ambient-state", "right", false)]),
            allowed_variances: vec![AllowedVariance {
                path: "ambient-state".to_string(),
                reason: "ambient-state".to_string(),
            }],
        };
        let error = compare(&input).expect_err("ambient state variance must be rejected");
        assert!(error.to_string().contains("unsupported drift variance reason ambient-state"));
    }

    #[test]
    fn retry_only_rerun_does_not_mask_canonical_ref_drift() {
        let input = ComparisonInput {
            left: summary(vec![
                field("artifact", &local_ref("left-artifact"), true),
                field("retry-attempt", "first", false),
            ]),
            right: summary(vec![
                field("artifact", &local_ref("right-artifact"), true),
                field("retry-attempt", "second", false),
            ]),
            allowed_variances: vec![AllowedVariance {
                path: "retry-attempt".to_string(),
                reason: "diagnostic-log".to_string(),
            }],
        };
        let comparison = compare(&input).expect("retry drift comparison");
        assert_eq!(comparison.decision, "deny");
        assert_eq!(comparison.diagnostics[0].path, "artifact");
        assert_eq!(comparison.diagnostics[0].kind, "value-drift");
    }
