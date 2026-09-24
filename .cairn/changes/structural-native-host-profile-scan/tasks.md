# Tasks: Structural native host profile scan

## Phase 1: Implementation

- [x] [serial] Replace the exact materialized-output literal with a qualification-tolerant structural probe bound to the effect completion requirement. a[structural-native-host-profile-scan.structural-field]
- [x] [serial] Route the required-source literal probes through a helper that names the missing literal and file. a[structural-native-host-profile-scan.named-failure]

## Phase 2: Validation

- [x] [serial] Positive: prove the probe accepts imported, `super::`, and `crate::` qualified forms. a[structural-native-host-profile-scan.structural-field]
- [x] [serial] Negative: prove the probe rejects `Option<String>` and a renamed `materialized_output_ref` field. a[structural-native-host-profile-scan.structural-field]
- [x] [serial] Negative: prove a missing literal fails with a named diagnostic. a[structural-native-host-profile-scan.named-failure]
- [x] [serial] Build `checks.x86_64-linux.native-system-extension-host-profile` and record the result in `evidence/verification.md`. a[structural-native-host-profile-scan.check-passes]
