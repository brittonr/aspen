# aspen-crypto

Dependency-light cryptographic primitives for Aspen reusable crates.

This crate owns portable helpers for cookie validation, key derivation, and optional Aspen identity-key lifecycle support. The default feature set is intentionally minimal and avoids transport/runtime dependencies; enable the `identity` feature when identity-key file I/O and Iroh key support are required.

License: AGPL-3.0-or-later.
