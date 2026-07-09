## Tasks

- [x] [serial] r[molten.modularity.layer_boundaries.ownership] Documented evidence, policy, runtime, and adapter ownership for the evidence-backed runtime workflow in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.modularity.layer_boundaries.evidence_policy_split] Separated evidence verification from policy admission with `plan_evidence_policy_runtime_flow` and explicit evidence-only non-claims.
- [x] [serial] r[molten.modularity.layer_boundaries.runtime_adapter_split] Ensured runtime planning consumes admitted inputs and returns planned effects while adapters execute IO after admission.
- [x] [parallel] r[molten.modularity.layer_boundaries.tests] Added positive and negative tests for admitted flow, evidence-only authority denial, stale policy/evidence inputs, and adapter availability not being trust.
- [x] [serial] r[molten.modularity.layer_boundaries.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
