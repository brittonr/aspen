# Design: local evidence ledger store

## Storage model

The ledger stores immutable content records keyed by canonical hash:

- report;
- suite;
- failure;
- gate receipt;
- repro bundle;
- repro verify receipt;
- redaction/reveal receipts;
- signed receipt envelopes;
- auxiliary refs manifests.

Indexes are derived from parsed canonical values and can be rebuilt. The hash-addressed content table is authoritative.

## Append-only evidence semantics

The ledger may add status records such as `verified`, `rejected`, `pinned`, `gc-eligible`, or `superseded`, but it must not mutate content bytes under an existing hash. If validation rules change, a new validation receipt is appended instead of rewriting old evidence.

## Interoperability

File exports remain canonical Preserves files. Importing a directory produced by `repro export` or `repro unpack` should add every artifact to the ledger and record an import receipt. Exporting from the ledger should reproduce the same canonical bytes.

## Retention and GC

Retention pins should name why an artifact is kept:

- pass evidence;
- diagnostic failure;
- user pin;
- policy retention;
- distributed exchange lease;
- parent/child dependency.

GC can remove unpinned content only when no retained receipt or dependency names it.
