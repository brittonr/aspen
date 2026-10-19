# Preflight caller-built chunk manifest count

## Problem

The chunk-manifest parser denies more than `MAX_CHUNK_STORE_CHUNKS = 65,536`, but `ChunkManifest` has public fields. Public `verify_remote_range_readback` and `plan_chunk_traversal_sync` accept caller-built manifests and currently validate references without enforcing the parser's count. Range readback then reserves space for every chunk before selecting a range; traversal clones every chunk reference into the plan. These public paths can bypass the existing parser admission boundary.

## Change

Use the existing count guard in the shared `validate_chunk_manifest_identity` function, before iterating the manifest's chunks. An exact-limit manifest remains eligible for existing identity/range/strategy validation. An over-limit manifest returns the source-owned count error before count-dependent work. No schema, manifest parser, public signatures, policy, waiver, or unrelated growth sites change.

## Boundary

This is an intake cardinality fix for the two public sync/readback helpers, not a new claim that caller-constructed manifests are fully authenticated or that remote locations confer object identity. Other critical review sites require independent owners.

## Follow-up: supplied byte-source tuples

Review found that readback also eagerly cloned every caller-supplied
`(String, Vec<u8>)` into a map, including unneeded extras, without limiting
the input tuple count. Preflight that separate count against the same
65,536-chunk cap before allocation, then collect borrowed `(&str, &[u8])`
entries instead. Preserve last-duplicate-wins lookup and admitted range-error
precedence. Required buffer processing and verified output reuse the
existing 1 GiB object-byte bound, not a new policy or receipt schema.
