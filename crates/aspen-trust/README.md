# aspen-trust

Dependency-light trust, secret-sharing, and envelope primitives for Aspen.

The default feature set exposes portable Shamir secret-sharing, key-derivation, envelope, and wire/state helpers. Async service APIs for key management and re-encryption are gated behind the `async` feature so reusable default consumers do not pull Tokio or async-trait.

License: AGPL-3.0-or-later.
