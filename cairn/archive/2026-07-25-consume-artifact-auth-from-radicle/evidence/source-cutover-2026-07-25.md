# Molten artifact-auth Radicle source cutover

## Accepted source

- RID: `rad:z4JGYYW7WsesXUq7MXVdx16Fawu2f`
- HTTPS Git: `https://git.onix.computer/z4JGYYW7WsesXUq7MXVdx16Fawu2f.git`
- Reviewed commit: `799459346d5416fbd7b9f55840a7371441b55afa`
- Source archive BLAKE3: `246a7cad91e7e8a158e22da21f3bff3e61aa0431a58936b5a739178bc62064c7`
- Publication revision: `e41340bec587b6d049b5cc518ec7db925dde84be`
- Publication receipt BLAKE3: `e58a3de4d6b3b32a547c3cfe5c3e829292cda73891c7776f214f5d4edce10b1c`

## Generated identity agreement

Cargo regenerated `Cargo.lock` after the root and `molten-core` manifests moved from GitHub SSH to Radicle HTTPS. Nix regenerated only the `artifact-auth-src` lock node. Both package entries remain version `0.1.0` at the exact reviewed commit, and the Nix `narHash` remains `sha256-nEgz2FtVuDesX95yyxidp0vhjxL4INB6Ve8rkpLyJk0=`.

The typed release-dependency profile now records two HTTPS artifact-auth rows against the same source and revision. The pinned unit2nix tool regenerated `build-plan.json` with include-dev coverage and `release-policy-build-plan.json` for `molten-release-policy`; executable source records use only the Radicle URL. Historical package repository metadata may still name GitHub and is not a fetch fallback.

## Behavioral evidence

Before the cutover at `f9ad1c0c16c7bb3c67dcb0d87f5910f287679946`, focused artifact-auth tests passed for ten pure cryptographic-identity cases and ten product-shell cases. The same focused positive and adversarial cases pass after the cutover. Focused no-dependency Clippy and workspace formatting pass. No Rust implementation file changed.

## Authority boundary

Molten's legacy decision remains authoritative, standalone authority remains unadmitted, and Molten retains key generation/storage/signing, currentness, capability, membership, transport, runtime, evidence, deployment, lifecycle, and release decisions. This receipt does not prove source correctness, forge availability, whole-Molten correctness, whole-stack GitHub independence, or release readiness.
