# Tasks: Bind promotion logical identity across restarts

Checked tasks record the executed implementation and observations in [the verification record](evidence/2026-10-21-verification.md); this checklist alone grants no completion or publication authority.

- [x] [serial] Record the current promotion record fields, dispatch retry identity handling, `OutcomeUnknown` paths, and the existing promotion test baseline. r[molten.promotion_identity.roles]
- [x] [serial] Add the four identity roles to the promotion record with bounded encodings and unknown-tolerant reads for pre-change records. r[molten.promotion_identity.roles]
- [x] [serial] Implement canonical request binding with domain-separated BLAKE3 over exact parameters and the incompatible-parameter rejection. r[molten.promotion_identity.request_binding]
- [x] [serial] Implement distinct-operation preservation for equal payload hashes across different logical identities. r[molten.promotion_identity.no_payload_merge]
- [x] [parallel] Add the restart replay regression: commit, effect or possible effect, lost response, reopen, retry with the original identity; plus the no-new-identity-after-restart negative. r[molten.promotion_identity.restart_binding]
- [x] [parallel] Add positive and negative fixtures: role round-trip, both mismatch rejections, expired generation, `OutcomeUnknown` retention across reopen, malformed and truncated records. r[molten.promotion_identity.validation]
- [x] [serial] Update promotion and dispatch docs with the four-role contract and the no-exactly-once external-effect non-claim. r[molten.promotion_identity.roles] r[molten.promotion_identity.validation]
- [x] [serial] Run focused core, adapter, and reopen tests before and after edits, then focused Octet, Clippy, workspace, Nix, and required Cairn gates. r[molten.promotion_identity.validation]
