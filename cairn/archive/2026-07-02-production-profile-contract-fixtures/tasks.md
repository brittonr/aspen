# Tasks: production-profile-contract-fixtures

- [x] [serial] r[molten.prod_ops.profile_contract_fixtures.positive_negative] Add a positive fixture for the reviewed production profile export.
- [x] [serial] r[molten.prod_ops.profile_contract_fixtures.positive_negative] Add negative Nickel fixtures for malformed refs, missing evidence, unsafe paths, vocabulary typos, invalid limits, invariant failures, and metadata errors.
- [x] [serial] r[molten.prod_ops.profile_contract_fixtures.validation_gate] Add a deterministic check that expects positive fixtures to export and negative fixtures to fail.
- [x] [parallel] r[molten.prod_ops.profile_contract_fixtures.validation_gate] Wire the fixture check into the smallest relevant validation rail used for production profile changes.
- [x] [parallel] r[molten.prod_ops.profile_contract_fixtures.evidence_boundary] Document that fixtures validate static contracts only and do not replace runtime or production gate receipts.
