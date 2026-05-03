# aspen-trust

Dependency-light trust, secret-sharing, and envelope primitives for Aspen.

The default feature set exposes portable Shamir secret-sharing, key-derivation, envelope, and wire/state helpers. Async service APIs for key management and re-encryption are gated behind the `async` feature so reusable default consumers do not pull Tokio or async-trait.

Randomness is intentionally split by API: low-level Shamir splitting accepts a caller-provided RNG for deterministic tests and simulations, while `ClusterSecret::generate`, encrypted share-chain salt generation, and high-level reconfiguration helpers draw fresh entropy from the process CSPRNG.

License: AGPL-3.0-or-later.
