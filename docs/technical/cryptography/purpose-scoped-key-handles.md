# Purpose-Scoped Key Handles

An opaque key handle is a public description of a permitted cryptographic role, not a portable private key and not an authority grant. This article explains how Molten separates those concepts at the pure-core and capability-file boundaries. Readers should first understand the [cryptographic identity contract](../../fabric-cryptographic-identity.md) and the distinction between checked reference syntax and authority in [nominal authority references](../../nominal-authority-references.md). The [Technical companion](../README.md) locates adjacent topics.

## What the handle identifies

The core `OpaqueKeyHandle` carries a schema, `handle_ref`, `profile_ref`, purpose, generation, public-key reference, backend class and reference, currentness, currentness-evidence reference, and a `secret_material_exposed` flag. These are inspected fields of the [core model](../../../crates/molten-core/src/fabric_crypto_identity/model.rs), not a claim that arbitrary callers may select their meaning. In particular, a backend reference is not a filesystem path or a secret-store credential.

The five purposes are `TransportEndpoint`, `FederationOrigin`, `Delegation`, `EvidenceSigning`, and `Authority`. Purpose is not inferred from the public key's encoding. Two operations can use the same signature algorithm while belonging to different authority contexts. Naming the role explicitly prevents an endpoint identity from silently becoming a federation-origin signing identity merely because both use Ed25519.

`canonical_key_handle` builds a Preserves record containing the public handle fields and hashes that record to obtain `handle_ref`. Its implementation also constructs a self-check signing request and passes it through core admission. Thus the handle reference identifies the canonical public description, rather than Rust memory layout, a display string, or secret bytes. See the [canonical handle implementation](../../../src/fabric_crypto_identity/parts/canonical/p000/body.rs).

Opacity should not be confused with unforgeability of the public data structure. Its fields are visible, and a content reference is reproducible from its content. The critical control is whether the shell resolves actual admitted key state and whether the request agrees with that state. Possession of a copied handle does not cause a private key to appear and does not itself satisfy the caller's external authorization obligations.

## Admission before the effect

`plan_sign` is deterministic over the supplied profile and request. It validates the signature domain and handle; rejects exposed secret material; checks profile, purpose, and signer-public-reference agreement; and compares the requested generation and handle reference with the supplied current values. These comparisons are visible in [core admission](../../../crates/molten-core/src/fabric_crypto_identity/admission.rs). The function does not open a file, sample entropy, or determine independently which generation is current.

The file adapter supplies that missing operational fact. `sign` resolves the purpose-specific persisted key with first-boot generation disabled, uses that resolution's generation and handle reference in the core request, and only then loads a key record and signs canonical domain bytes. The [shell implementation](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs) therefore has a different responsibility from the planner: it connects pure comparisons to capability-rooted storage and an actual cryptographic effect.

The integration wrappers add consumer-specific purpose checks. `sign_federation_payload` requires federation purpose on both handle and domain; `sign_evidence_payload` requires evidence purpose. Their [implementation](../../../src/fabric_crypto_identity/integration.rs) forwards only after that distinction holds. This is a narrower guarantee than proving federation membership or evidence admissibility.

## Illustrative cross-purpose failure

Suppose an operator has resolved a generation-4 transport handle and retained its public key. An application then constructs a federation-origin domain referring to that same public key and asks to sign it with the transport handle. The matching public key does not make the operation admissible: the purposes differ, so `plan_sign` reports `PurposeMismatch`. The federation wrapper rejects the mismatch even before invoking the adapter.

Changing the copied handle's purpose is not a legitimate migration. The shell resolves the federation-purpose file, not the transport-purpose file, and compares the request against the current handle for that role. If no federation key exists, resolution with generation disabled fails. If one exists, its canonical handle and public identity define the comparison target. The supported route is resolving the appropriate role and satisfying the corresponding signing admission, not relabeling a transport handle.

A second failure is temporal rather than categorical. If a caller retains an old handle after rotation, its purpose can remain correct while its generation or handle reference is stale. Purpose scoping and generation fencing address independent substitution errors; neither can replace the other.

## Review and verification guidance

Review a signing call by tracing four values together: intended consumer purpose, canonical domain purpose, resolved key purpose, and actual current handle. Then identify where the policy reference originates. Reference well-formedness is not policy approval, just as a `KeyRef`'s nominal category is not proof of current authority.

Suggested verification is to exercise a valid same-purpose signing path, a transport-to-federation substitution, and a stale-handle request after rotation. Existing shell scenarios are located in [identity adapter tests](../../../src/fabric_crypto_identity/parts/tests/p000/body.rs). These are review targets, not newly executed test evidence for this article.

## Limits and non-claims

Handle admission does not prove membership, capability possession, trust-root selection, backend availability, or payload truth. The core's `Overlap` currentness permits signing in its general model, whereas the current capability-file rotation implementation supports only no-overlap rotation; do not infer an implemented overlap-signing service from the enum. Nor does an opaque handle establish concurrency serialization between resolution and later key loading. This article describes the inspected checks without claiming a stronger transaction protocol.

## Sources

- [Cryptographic identity contract](../../fabric-cryptographic-identity.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Core key model](../../../crates/molten-core/src/fabric_crypto_identity/model.rs)
- [Core signing admission](../../../crates/molten-core/src/fabric_crypto_identity/admission.rs)
- [Canonical handle construction](../../../src/fabric_crypto_identity/parts/canonical/p000/body.rs)
- [Capability-file signing](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs)
- [Consumer purpose checks](../../../src/fabric_crypto_identity/integration.rs)
