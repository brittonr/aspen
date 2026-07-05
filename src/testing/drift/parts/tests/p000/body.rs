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
