# Chunk manifest source-owned count admission

## Shared boundary

Both public entry points call `validate_chunk_manifest_identity` before range-collection allocation, traversal-reference cloning, and any port I/O. Call `ensure_count_at_most(manifest.chunks.len(), MAX_CHUNK_STORE_CHUNKS, "chunk manifest chunks")` there before validating refs. This reuses the parser's existing cap, label and `MoltenError::invalid_harness` diagnostic instead of adding a divergent setting. Only constant-time `Vec::len` and the error message are needed for an over-limit manifest.

## Compatibility and precedence

Do not reorder existing manifest, metadata, root and chunk-ref validations relative to one another for admitted inputs. At 65,536 chunks, continue to the existing checks and ordinary readback/traversal behavior. At 65,537 chunks, deny before examining refs, strategy, peers, range bytes, or any count-dependent allocation. Keep the remote hint a hint, not content authority; no receipt or schema changes.

## Verification

Exercise original failing behavior with a caller-built 65,537-entry manifest and an exact-65,536 positive readback, plus traversal precedence. Use one small referenced byte slice, not 65,536 cloned byte buffers. Then run focused library regression, a compiled external public API caller, scoped formatting/Clippy/Octet, and Cairn validation; track exactly which profiles actually run, without treating warning-only Octet as strict gate success.

## Supplied lookup and byte processing

After shared manifest admission and before allocating readback diagnostics,
apply `ensure_count_at_most(input.chunk_bytes.len(), MAX_CHUNK_STORE_CHUNKS,
"remote byte-source supplied chunks")`. Preserve existing required-range
checks before lookup for admitted tuple counts. Collect an ordered
`BTreeMap<&str, &[u8]>` from `.iter()`, so duplicate keys still overwrite
earlier entries but unnecessary byte vectors and ref strings are not cloned.
Before hashing a required buffer, check its length against existing
`MAX_CHUNK_STORE_OBJECT_BYTES`. Append verified buffers through existing
`extend_bytes_bounded` using that same 1 GiB limit, so neither a
caller-supplied numeric `chunk_size` nor a declared range length can make
the returned buffer unbounded. Keep existing diagnostics and receipt fields.
