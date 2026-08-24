## Why

Aspen is the Molten policy-gated distributed runtime workspace: a Preserves/BLAKE3 runtime with dataspaces, vats, and Iroh. Distributed coordination today leans on peer-peer exchange and gate audits. It does not yet record a design contract for a durable-store-coordinated ownership mode.

Celld shows that a single S3 bucket can replace a membership protocol, a failure detector, and a consensus service: ownership of each cell is a compare-and-swap lease in the bucket, nodes are replaceable, and the bucket is the durable source of truth. That is a clean architecture option for a dataspaces runtime: when coordination uses a durable store, ownership is a CAS lease with no fixed membership list.

This change records a bounded design contract. The runtime and its consumers retain coordination, transport, and evidence authority.

## What Changes

- Add a bounded design contract for durable-store-coordinated ownership.
- Require ownership transfer to be a single compare-and-swap lease when this mode is used.
- Require replaceable nodes and no fixed membership list.
- Keep the contract deterministic and effect-free.
- Add positive, negative, and boundary fixtures for explicit current, expected, and proposed CAS leases.
- Reference the reviewed Celld ownership model as a bounded, non-parity input.
- Reference WalTier's CAS-arbiter log as a related, non-parity input for the consistency-extension port.

## Impact

The change records a reviewable coordination option. It does not implement a store, a transport, or a consensus-free runtime today.

CAS lease identities and receipts remain BLAKE3.

## Dependencies

This change has no downstream prerequisite.

## Non-goals

- Do not implement object storage, Iroh, or a new transport.
- Do not replace the existing peer, gate, or evidence paths unless adopted.
- Do not claim parity with, or equivalence to, the Celld ownership implementation.
- Do not convert a CAS lease design into proof of runtime correctness or data integrity.
- Do not turn the CAS-arbiter family into a consensus requirement. Keep consistency as an explicit extension port.
