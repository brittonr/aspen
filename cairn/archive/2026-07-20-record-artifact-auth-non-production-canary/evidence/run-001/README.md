# Molten artifact-auth non-production canary run 001

This product-owned archive records the bounded public evidence produced on 2026-07-20 from Molten `19549c83f9cb046f0f6f4adebd4fa02f2b936deb` with artifact-auth `799459346d5416fbd7b9f55840a7371441b55afa`.

## Observed result

- Molten's capability-file adapter generated an OS-CSPRNG Ed25519 evidence key and wrote the receipt through the `Receipts` namespace.
- Capture and fresh-process replay passed with receipt `blake3:31b380d81cc4f7559d63a775d6012b4fadd8ca8550e5a561bacff6b9f0c8f4de`.
- Rotating generation 1 to generation 2 caused the original receipt to fail closed with current-key-state drift.
- A later fresh process observed generation 2 and a changed handle relative to the generation-1 receipt.

## Members

`harness.rs` is the exact temporary Rust example compiled against the landed revision. The JSON files and log are the public receipt, replay, rotation, and post-rotation observations. `manifest.ncl` declares the claim boundary; its cross-consumer revisions are review linkage, not a joint signature or attestation. `BLAKE3SUMS` binds every regular member except itself.

Private key material and mutable node state are intentionally absent.

## Validation

```text
nix shell nixpkgs#nickel -c nickel typecheck manifest.ncl
./hash-evidence.rs . > regenerated && cmp BLAKE3SUMS regenerated
```

A negative fixture containing a symlink must be rejected by `hash-evidence.rs`. JSON and log members must contain no private-key marker or serialized secret-key field.

## Non-claims

This archive is non-production operational evidence. It does not establish network revocation freshness, membership, capability, federation, transport, storage, lifecycle, signing-policy, release authority, production rollout, or standalone authority.
