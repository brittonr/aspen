# Tasks: production-profile-named-units

- [x] [serial] r[molten.prod_ops.profile_named_units.named_thresholds] Add Nickel unit and threshold constants for production profile byte, time, and queue limits.
- [x] [serial] r[molten.prod_ops.profile_named_units.named_thresholds] Replace concrete production profile resource-limit literals with named constants.
- [x] [serial] r[molten.prod_ops.profile_named_units.export_stability] Verify that the current profile export remains stable after replacing literals with constants.
- [x] [parallel] r[molten.prod_ops.profile_named_units.export_stability] Add fixture coverage that fails when an unintended numeric export drift occurs.
- [x] [parallel] r[molten.prod_ops.profile_named_units.named_thresholds] Document the named thresholds in the production operator runbook.
