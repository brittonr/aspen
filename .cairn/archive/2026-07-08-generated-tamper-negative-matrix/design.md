## Context

Existing tests mutate reports, bundles, receipts, release members, chunk manifests, and redaction evidence in focused ways. The same negative classes recur across surfaces. Centralizing them reduces drift and makes new artifact families easier to harden.

## Design

Define a pure tamper matrix core over in-memory Preserves values and typed artifact descriptors. The matrix should support mutation classes such as:

- missing required field;
- stale content ref;
- wrong artifact kind;
- malformed content ref;
- duplicate member;
- tampered embedded receipt;
- noncanonical value;
- diagnostic-only artifact presented as pass evidence;
- missing child receipt;
- unsupported schema version.

The core returns expected denial class and fixture metadata. Shell code may materialize files for CLI tests, but gate/parsing behavior should be checked through pure parser and gate functions where possible.

## Validation

Start with harness reports, gate receipts, repro bundles, redaction evidence, and release evidence bundles. Positive fixtures prove valid artifacts still pass; negative fixtures prove each mutation class denies with the expected diagnostic and without pass evidence.
