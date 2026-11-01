# Tasks: admit fabric schema labels as contract labels

## Implementation

- [x] [serial] Distinguish schema labels, reference-collection counts and content identities in the service-host bundle codec. r[molten.system_extension.hosted_codec]
      `src/node/servicehost/codec/wire.rs::field`.

## Verification

- [x] [serial] Prove the label admission and that content positions still refuse labels, then run the service-host codec and node service-host tests. r[molten.system_extension.hosted_codec]
      `fabric_schema_labels_are_not_content_references` plus the existing codec suite; evidence
      `evidence/2026-10-31-codec-labels.md`.
- [x] [serial] Run formatting, strict Clippy, Octet and the affected checks before publication. r[molten.system_extension.hosted_codec]
      Recorded in the same evidence file.
