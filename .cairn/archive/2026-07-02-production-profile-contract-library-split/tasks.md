# Tasks: production-profile-contract-library-split

- [x] [serial] r[molten.prod_ops.profile_contract_library.reusable_module] Extract production profile Nickel contracts and constants into a reusable contract module.
- [x] [serial] r[molten.prod_ops.profile_contract_library.instance_profile] Rewrite the checked-in pilot profile as a concrete instance that imports and applies the reusable contract.
- [x] [serial] r[molten.prod_ops.profile_contract_library.no_runtime_nickel] Confirm runtime node startup continues to consume checked exported JSON rather than evaluating Nickel.
- [x] [parallel] r[molten.prod_ops.profile_contract_library.instance_profile] Update the production runbook export instructions for the split layout.
- [x] [parallel] r[molten.prod_ops.profile_contract_library.reusable_module] Repoint profile fixtures to the shared contract module.
