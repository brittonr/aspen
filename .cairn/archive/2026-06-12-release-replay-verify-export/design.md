# Design: release-replay-verify-export

Local dogfood already creates a generic `deterministic-replay-verify-v1` receipt and indexes it with `deterministic-replay-index-v1`. This change makes the raw verify receipt a first-class release evidence member:

- `molten dogfood local-node --replay-verify-out PATH` writes the generic verify receipt.
- Nix dogfood release evidence records both `replay-verify` and `replay-index` refs.
- Release bundle and bundle verification receipts bind both refs under a replay block.
- Signed-member verification requires the replay verify member when signed members are mandatory.
- Release export manifests and archive verification include `replay-verify.preserves` and `replay-verify.signed.preserves`.

The replay verify receipt remains reusable evidence only. It does not grant authority, source-gate trust, policy admission, provenance trust, resource trust, transport trust, retention/destructive-operation trust, signed-key trust, release promotion, or release acceptance.
