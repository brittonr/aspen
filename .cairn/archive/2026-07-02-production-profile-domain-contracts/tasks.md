# Tasks: production-profile-domain-contracts

- [x] [serial] r[molten.prod_ops.profile_domain_contracts.scalar_types] Add Nickel validators for BLAKE3 refs, non-empty profile text, absolute state roots, and safe relative layout directories.
- [x] [serial] r[molten.prod_ops.profile_domain_contracts.positive_limits] Replace loose resource-limit `Number` contracts with positive integer limit contracts.
- [x] [serial] r[molten.prod_ops.profile_domain_contracts.scalar_types] Apply the domain contracts to `docs/production-node-profile.ncl` without changing the valid profile JSON shape.
- [x] [parallel] r[molten.prod_ops.profile_domain_contracts.scalar_types] Document the scalar contract meanings in the production operator runbook.
- [x] [parallel] r[molten.prod_ops.profile_domain_contracts.positive_limits] Add negative export coverage for malformed refs, unsafe paths, zero limits, and fractional limits.
