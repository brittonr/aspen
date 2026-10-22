# Design: Store declared state as digest-named records

## Context

Molten's durable state already has capability-rooted directories, canonical Preserves records, and BLAKE3 identity for
content. Declared interest — who asked to keep or to run something — is still written into shared documents and
database records rather than one record per owner.

## Approach

Define the layout for an admitted declared-state store:

- Directory per store class, with one record file per declared fact.
- The file name is the BLAKE3 digest of the canonical record bytes.
- A reader enumerates the directory, parses each record, and verifies that the recomputed digest matches the file name
  before the record participates in the merge.
- The merged view is the union of valid records, ordered by digest, independent of directory order.
- Removing a record file retracts exactly that fact. A concurrent writer adds a different file instead of rewriting a
  shared document.

## Decisions

### Decision: Digest names, not sequence numbers or owner names

**Choice:** Name each record file by the digest of its canonical content.

**Rationale:** The name is a checkable claim about the content, duplicate facts collapse to one file, and no writer
needs a lock to allocate a name. Owner-scoped names would need a collision rule and would let one owner overwrite
another's record.

### Decision: Consumer first

**Choice:** Keep the first task as a consumer decision. If no admitted store needs multi-writer declared state, the
package closes without code.

**Rationale:** The layout is only correct in the context of a real reader. Adding it speculatively would create a
second way to store declared state with no consumer.

## Risks / Trade-offs

- Many small files cost inodes and directory operations. Bound the store size and record size, and keep the layout for
  declared-interest facts rather than for bulk content.
- A digest name leaks nothing about the content, so operators list records by reading them. Records keep a bounded,
  readable body plus a canonical ref for grouping.
- The design uses BLAKE3, not the manual's SHA-1, because BLAKE3 is the stack's identity algorithm.
