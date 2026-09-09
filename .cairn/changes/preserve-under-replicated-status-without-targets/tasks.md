# F07 implementation tasks

All tasks require separate implementation authorization. No task records completed work.

- [ ] [serial] Run the smallest existing planner, status, and receipt baselines before core edits. Record failures. r[molten.audit_f07.validation_claims]
- [ ] [serial] Move `audit_insufficient_peers_remain_under_replicated` into normal repository tests with named fixture values. r[molten.audit_f07.deficit_status]
- [ ] [serial] Add positive satisfied-inventory and negative action-free deficit tests with exact issue and decision assertions. r[molten.audit_f07.deficit_status]
- [ ] [serial] Resolve per-content representation, canonical status compatibility, and historical receipt replay decisions before schema changes. r[molten.audit_f07.per_content_accounting] r[molten.audit_f07.validation_claims]
- [ ] [serial] Implement pure-core deficit accounting independently from action presence and aggregate success counts. r[molten.audit_f07.deficit_status] r[molten.audit_f07.per_content_accounting]
- [ ] [serial] Keep effects and publication in the imperative shell and prevent complete receipts while demand remains unresolved. r[molten.audit_f07.receipt_decision]
- [ ] [serial] Add partial-target success, multiple-content, unavailable-transfer, rejected-verification, and denied-plan tests. Assert rejection state preservation. r[molten.audit_f07.per_content_accounting] r[molten.audit_f07.receipt_decision]
- [ ] [serial] Add controlled-adapter tests for complete and partial receipts without fabricated transfers. Cover valid and malformed canonical status records. r[molten.audit_f07.receipt_decision] r[molten.audit_f07.validation_claims]
- [ ] [serial] Add F04 historical-success and F05 cleanup-denial integration controls without merging the packages. r[molten.audit_f07.per_content_accounting]
- [ ] [serial] Update operator docs with action-free deficits, partial receipts, the maintenance owner, and evidence limits. r[molten.audit_f07.validation_claims]
- [ ] [serial] Rerun focused baselines and run focused Octet error gates and Clippy with `-D warnings`. r[molten.audit_f07.validation_claims]
- [ ] [serial] Run workspace tests across all targets and compatible features, relevant Nix replication checks, and required repository gates. r[molten.audit_f07.validation_claims]
- [ ] [serial] Run required Cairn validation, traceability, compatibility, and evidence gates without weaker checks. Record exact results and blockers. r[molten.audit_f07.validation_claims]
