#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn admits_defaults_and_binds_receipt_caveat() {
        let profile = default_runtime_limit_profile();
        let admission = admit_runtime_limits(RuntimeLimitAdmissionInput {
            profile: &profile,
            hard_caps: &default_runtime_limit_hard_caps(),
            overrides: &RuntimeLimitOverrides::default(),
            allow_widening_overrides: false,
        })
        .expect("limit admission");
        assert_eq!(admission.decision, "pass");
        assert!(admission.limits.used_default_budget);
        let text = crate::preserves_rail::to_text(&admission.value).expect("admission text");
        assert!(text.contains("default-budget-caveat"));
        assert!(text.contains("pure-limit-admission"));
    }

    #[test]
    fn denies_one_past_hard_cap() {
        let mut profile = default_runtime_limit_profile();
        profile.frame_bytes = default_runtime_limit_hard_caps()
            .frame_bytes
            .checked_add(1)
            .expect("one past cap");
        let admission = admit_runtime_limits(RuntimeLimitAdmissionInput {
            profile: &profile,
            hard_caps: &default_runtime_limit_hard_caps(),
            overrides: &RuntimeLimitOverrides::default(),
            allow_widening_overrides: false,
        })
        .expect("limit admission");
        assert_eq!(admission.decision, "deny");
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("frame bytes")));
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("hard cap")));
    }

    #[test]
    fn denies_incoherent_units() {
        let mut profile = default_runtime_limit_profile();
        profile.live_join_timeout_ms = DEFAULT_LIVE_LISTENER_TIMEOUT_MS
            .checked_add(DEFAULT_CONTROL_TICK_MS)
            .expect("larger timeout");
        profile.frame_bytes = DEFAULT_SESSION_BYTES.checked_add(1).expect("larger frame");
        let admission = admit_runtime_limits(RuntimeLimitAdmissionInput {
            profile: &profile,
            hard_caps: &default_runtime_limit_hard_caps(),
            overrides: &RuntimeLimitOverrides::default(),
            allow_widening_overrides: false,
        })
        .expect("limit admission");
        assert_eq!(admission.decision, "deny");
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("timeout")));
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("frame bytes")));
    }

    #[test]
    fn denies_widening_overrides_and_accepts_tightening() {
        let profile = default_runtime_limit_profile();
        let widening = RuntimeLimitOverrides {
            queue_depth: Some(profile.queue_depth.checked_add(1).expect("widen queue")),
            ..RuntimeLimitOverrides::default()
        };
        let denied = admit_runtime_limits(RuntimeLimitAdmissionInput {
            profile: &profile,
            hard_caps: &default_runtime_limit_hard_caps(),
            overrides: &widening,
            allow_widening_overrides: false,
        })
        .expect("denied override");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("widens")));

        let tightening = RuntimeLimitOverrides {
            queue_depth: Some(profile.queue_depth.checked_sub(1).expect("tighten queue")),
            ..RuntimeLimitOverrides::default()
        };
        let accepted = admit_runtime_limits(RuntimeLimitAdmissionInput {
            profile: &profile,
            hard_caps: &default_runtime_limit_hard_caps(),
            overrides: &tightening,
            allow_widening_overrides: false,
        })
        .expect("accepted override");
        assert_eq!(accepted.decision, "pass");
        assert_eq!(accepted.limits.queue_depth, profile.queue_depth - 1);
    }
}
