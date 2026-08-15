# Pilot exact artifact-auth shell verification

## Why

Molten's pure artifact-auth adapter currently accepts a caller-supplied cryptographic observation. That preserves authority boundaries, but it does not prove that the product shell can sign and verify the exact `artifact_auth.statement.v1` bytes with a production-managed Ed25519 key. Authority admission must remain blocked until that gap has direct positive and adversarial evidence.

## What changes

- Expose one pure core mapping from Molten observations to the exact standalone statement.
- Add a thin product shell that signs those canonical bytes through the existing capability-file key adapter and verifies them with the pinned `artifact-auth-ed25519` implementation.
- Record public statement, key, and signature identities plus signature hex for bounded dual-run evidence.
- Add real positive, tamper, wrong-preimage, wrong-key, malformed-signature, revoked/currentness, and carrier-drift tests.
- Keep the legacy decision authoritative, standalone runtime authority unadmitted, and rollback available.

## Impact

The pilot changes `molten-core` mapping APIs, the root Molten cryptographic shell, exact Cargo/unit2nix dependency plans, focused tests, and operator documentation. It does not grant capability, membership, transport, deployment, lifecycle, or release authority and does not open a Cairn authority-admission change.
