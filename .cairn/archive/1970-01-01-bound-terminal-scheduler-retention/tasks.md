# F11 tasks

Checked tasks record the implemented contract and executed observations in [the verification record](evidence/2026-10-21-verification.md); remaining acceptance gates stay unchecked until they pass. This checklist alone grants no completion or publication authority.

- [x] [serial] Run the smallest existing scheduler, capacity, and shell tests before core edits and record the baseline. Cite r[molten.audit_f11.validation].
- [x] [serial] Move repeated completion under active limit one into normal core tests with named retention and repetition bounds. Cite r[molten.audit_f11.retention].
- [x] [serial] Add positive and negative Complete, Cancel, cleanup, and physical-capacity fixtures before the retention change. Cite r[molten.audit_f11.retention].
- [x] [serial] Review the named derived retention bound, occurrence sequence, high-water fence, and canonical compatibility boundary with the fabric-time maintainer. Cite r[molten.audit_f11.freshness].
- [x] [serial] Add old-callback-after-eviction, duplicate issuance, logical-name reuse, and sequence-exhaustion tests with rejected-state equality. Cite r[molten.audit_f11.freshness] and r[molten.audit_f11.atomicity].
- [x] [serial] Implement deterministic retention and checked occurrence admission in the pure core without an unbounded retired-ID collection. Cite r[molten.audit_f11.retention] and r[molten.audit_f11.freshness].
- [x] [serial] Update checked capacity planning and shell reservations without hidden capacity growth or callback authority in adapters. Cite r[molten.audit_f11.atomicity].
- [x] [serial] Preserve F09 blocked occurrence identity and F10 shared Ready admission during retention integration. Cite r[molten.audit_f11.freshness].
- [x] [serial] Add positive and negative live/simulation callback and serialization tests, including malformed or missing occurrence fences. Cite r[molten.audit_f11.validation].
- [x] [serial] Document schema/cohort compatibility, deterministic replay, evicted-history readback, and receipt non-claims in fabric-time docs. Cite r[molten.audit_f11.validation].
- [x] [serial] Repeat focused tests after edits with replay across eviction and stale callback injection into a fresh logical-name occurrence. Cite r[molten.audit_f11.validation].
- [x] [serial] Run focused Octet checks, the existing strict Octet gate, and Clippy with `-D warnings` across required targets and features. Cite r[molten.audit_f11.validation].
- [x] [serial] Run required workspace tests, relevant Nix checks, and the repository Nix gate without weakening existing checks. Cite r[molten.audit_f11.validation].
- [x] [serial] Run native `.cairn/` validation and required gates with tracked evidence, keeping accumulation and new fence claims separate. Cite r[molten.audit_f11.validation].
