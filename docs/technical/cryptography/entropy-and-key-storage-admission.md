# Entropy and Key Storage Admission

Production identity depends on both key generation and the continued controlled use of persisted secret material. A mathematically valid Ed25519 key is not enough if its origin is a deterministic fixture or its storage admits unintended readers. This article follows the capability-file adapter described by the [cryptographic identity contract](../../fabric-cryptographic-identity.md), separating pure admission from operating-system effects. It assumes knowledge of capability-rooted node state; the [Technical companion](../README.md) provides adjacent storage material.

## Admission describes an allowed operation

The pure `CryptoAdapterProfile` names a production or fixture class, algorithm, allowed backend classes and purposes, entropy-profile reference, domain version, signature bound, sharing configuration, and required non-claims. Production validation requires `Ed25519Iroh`, production-eligible backends, and an entropy-profile reference. The [core admission implementation](../../../crates/molten-core/src/fabric_crypto_identity/admission.rs) validates those declarations without reading an entropy device or opening a secret file.

`admit_key_generation` additionally checks agreement between request and profile, including purpose, backend, and entropy reference; rejects generation zero; and requires first-boot generation permission. Its returned plan marks production persistence as restricted and does not permit replacement of an existing key. A well-formed entropy reference is a binding to declared context, not a measurement proving that random bytes were sampled correctly.

The checked [production template](../../fabric-cryptographic-identity/profile-template.ncl) declares `os-csprng`, capability-rooted storage, owner-only permissions, and purpose separation. The governing contract attributes production Ed25519 generation to the operating-system CSPRNG. In the inspected shell, the concrete generation call is `iroh::SecretKey::generate()`. These observations support the selected profile and call path; they do not constitute an independent audit of Iroh's random-number implementation or the host entropy subsystem.

## Resolution does not silently repair missing identity

`IrohEd25519FileAdapter::new` first admits the production profile, requires capability-file support, validates the backend reference, and accepts only Identity or Secrets node-state namespaces. The [resolution implementation](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs) checks revocation before observing the purpose-specific leaf. Missing, nonregular, and regular files take distinct paths.

A missing file is generated only when the caller explicitly permits first-boot generation. With that permission disabled, the adapter returns an error rather than inventing a replacement identity. A nonregular leaf is rejected. A regular file must have restricted permissions before bounded reading and record decoding. These decisions preserve the distinction between intentional enrollment and recovery from unexpected key loss: treating every missing key as first boot would change public identity while hiding the operational failure.

The generated binary record contains the `MCKEY001` schema marker, a big-endian generation, and 32 secret bytes. The writer requests mode `0600`. Decoding requires the exact record length and schema and a positive generation; it is not generic deserialization of a debug or JSON object. See the [record layout](../../../src/fabric_crypto_identity/file/parts/adapter/p000/body.rs), [encoding](../../../src/fabric_crypto_identity/file/parts/adapter/p002/body.rs), and [decoding guards](../../../src/fabric_crypto_identity/file/parts/adapter/p003/body.rs).

## Permission observations have a scope

On Unix, `permission_status` checks that group and other permission bits are absent. Missing mode information is unsupported; non-Unix builds report unsupported through this helper. `require_restricted_permission` rejects both unsafe and unsupported results. The precise inspected predicate is not a general proof of file ownership, ACL safety, mount safety, process isolation, or absence of a privileged reader. Requesting mode `0600` at creation and inspecting restricted mode bits at use are concrete protections within a larger host-security boundary.

Public status is a different surface from key admission. The core redactor retains bounded public fields and receipt references, marks sensitive diagnostic categories as redacted, and rejects an input declaring private material present. It does not transform raw private bytes into safe public evidence. There is also a narrow source discrepancy: the [governing status prose](../../fabric-cryptographic-identity.md) mentions an opaque handle reference, but the inspected [canonical status record](../../../src/fabric_crypto_identity/parts/canonical/p001/body.rs) has no handle-reference field. Consumers should not assume that field exists on this surface.

## Illustrative restart failure

Suppose a node successfully generated its transport key, then a backup restoration restores the key leaf with mode `0644`. The bytes may decode correctly and yield the expected public key, but ordinary resolution rejects the unsafe permission observation before reading them for signing use. Generating another key is not a valid response to this error: it would conflate unsafe restoration with authorized first boot.

Conversely, suppose the restored leaf is absent and generation permission is false. The failure is missing admitted state, not an entropy defect. The operator needs the deployment's recovery procedure; the adapter does not infer authorization to replace identity. This example is illustrative and does not prescribe a new recovery policy.

## Review and verification guidance

Suggested verification covers first generation, restart resolution with generation disabled, missing state, nonregular leaves, unsafe permissions, malformed record length, and fixture-profile rejection. Relevant scenarios live in [production lifecycle tests](../../../src/fabric_crypto_identity/parts/tests/p000/body.rs) and [negative adapter tests](../../../src/fabric_crypto_identity/parts/tests/p001/body.rs). No tests were executed for this documentation article.

Review entropy provenance separately from storage admission and canonical public evidence. The production model lists `ManagedSecret` as eligible, but the inspected implementation here is the capability-file adapter; eligibility is not evidence of an implemented or reviewed managed service. Deterministic BLAKE3 fixtures remain test integrity mechanisms, not substitute production identities.

## Limits and non-claims

This boundary does not guarantee backend availability, secure erasure, host compromise resistance, crash-atomic multi-file transitions, or production deployment approval. Reference syntax and [nominal category separation](../../nominal-authority-references.md) cannot upgrade a profile declaration into observed entropy or storage safety. The core remains free of ambient generation and filesystem effects; evidence about those effects originates in the shell and its operational context.

## Sources

- [Cryptographic identity contract](../../fabric-cryptographic-identity.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Production profile template](../../fabric-cryptographic-identity/profile-template.ncl)
- [Pure generation admission](../../../crates/molten-core/src/fabric_crypto_identity/admission.rs)
- [Capability-file resolution and generation](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs)
- [Permission and record guards](../../../src/fabric_crypto_identity/file/parts/adapter/p003/body.rs)
- [Canonical public status](../../../src/fabric_crypto_identity/parts/canonical/p001/body.rs)
