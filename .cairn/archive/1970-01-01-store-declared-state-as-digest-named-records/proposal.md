# Proposal: Store declared state as digest-named records

## Why

The manual gives a concrete layout for durable declared interest: each user setting is one file named after the digest
of the canonical form of its assertion, so content identity equals file identity, and a rewriting watcher merges the
files into the live configuration (`19-operation__synit-config.md → User settings`).

Molten keeps declared operator, lifecycle, and retention state in aggregate documents and database records. An
aggregate document gives two writers one mutable file, and it cannot answer which owner declared a fact. A
record-per-fact layout makes removal a single-file deletion, keeps unrelated owners intact, and makes the file name a
verifiable claim about the content.

The tracey baseline lists `molten.durable_state_ports.*` requirements as accepted and implementation-unestablished.

## What Changes

- Apply one admitted layout to any Molten store of declared-interest records: one record per fact, named by the BLAKE3
  digest of the canonical record content, with no other file able to claim the same digest.
  r[molten.durable_state_ports.digest_named_records]
- Require the record name to match its content on read. A record whose name and content disagree MUST be rejected
  rather than repaired in place.
- Keep merge deterministic: the record set for one store is the union of valid records, deduplicated by digest, and no
  record's presence depends on file order or modification time.
- Name the consumer before implementation. Without an admitted store that needs multi-writer declared state, this
  package closes with the decision and no code.

## Impact

- **Files**: the admitted declared-state store, the project's retention and node-state document paths, and the
  referenced tests and docs.
- **Testing**: positive cases for two writers adding records concurrently and for a record whose removal retracts one
  fact; negative cases for a malformed record, a name that does not match content, and duplicate content under two
  names.
- **Non-goals**: no migration of existing aggregate documents without an admitted consumer, no change to the
  content-addressed chunk store, and no new authority or retention decision.
