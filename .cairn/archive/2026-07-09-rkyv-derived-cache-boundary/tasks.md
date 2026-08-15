## Tasks

- [x] [serial] r[molten.local_eval_cache.rkyv_preserves_source_of_truth] Document the derived rkyv archive data flow and source-of-truth boundary in cache/storage docs.
- [x] [serial] r[molten.local_eval_cache.rkyv_manifest] Define the canonical Preserves manifest fields for derived archive sidecars.
- [x] [parallel] r[molten.local_eval_cache.rkyv_admission] Add a pure manifest/source-ref admission core that returns admit, rebuild, or deny without file IO.
- [x] [parallel] r[molten.local_eval_cache.rkyv_validation] Add shell-owned validation and exact-byte receipt plumbing before archive reads.
- [x] [serial] r[molten.local_eval_cache.rkyv_identity_boundary] Ensure cache/evidence/ref code paths continue to hash canonical Preserves inputs rather than rkyv archive bytes.
- [x] [serial] r[molten.storage.derived_archive_sidecars] Keep typed storage values canonical while allowing rkyv only as tagged, rebuildable sidecars.
- [x] [parallel] r[molten.local_eval_cache.rkyv_negative_tests] Add positive tests for admitted/rebuilt archives and negative tests for stale refs, bad digests, tampering, missing validation, malformed manifests, incompatible profile versions, and authority overclaims.
- [x] [serial] r[molten.local_eval_cache.rkyv_negative_tests] Run focused cache/storage tests plus Cairn validation.