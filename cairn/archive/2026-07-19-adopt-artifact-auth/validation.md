# Validation evidence

## Baseline

Before core changes, native Cairn validation and proposal/design/tasks gates passed. `nix develop -c cargo test -p molten-core fabric_crypto_identity` passed all six existing cryptographic-identity tests, establishing the current domain, purpose, generation, rotation, currentness, redaction, and retained-authority baseline.

The reviewed standalone profile is `config/consumers/molten.ncl` at artifact-auth revision `799459346d5416fbd7b9f55840a7371441b55afa`. It records Molten source baseline `70590459a218dc8e66948ab6f305a7c54142b710`, shared domain/purpose/profile/payload/public-key/verifier-context/generation/currentness fields, opaque-handle/backend/entropy/rotation extensions, and retained key/capability/federation/Preserves/Iroh authority. Current source review found the same semantic classes, with `Current`, verification overlap, superseded, and revoked states explicit in `fabric_crypto_identity`.

## Exact source admission

Cargo and Nix resolve `ssh://git@github.com/OnixResearch/artifact-auth.git` at full revision `799459346d5416fbd7b9f55840a7371441b55afa`. `Cargo.lock` contains one `artifact-auth-core` package from that revision. `flake.lock` records the same SSH URL, revision, and NAR hash `sha256-nEgz2FtVuDesX95yyxidp0vhjxL4INB6Ve8rkpLyJk0=`. Flake evaluation asserts exact Cargo/Nix revisions, one lock package, exact lock source, and the standalone `MIT OR Apache-2.0` license. Locks were generated only by Cargo and Nix tooling.

## Adapter and cutover result

`crates/molten-core/src/fabric_crypto_identity/artifact_auth.rs` is a pure post-verification adapter. It maps the Molten profile and verification request plus an explicit, separate standalone `CryptographicObservation`; it never reuses `cryptographic_verification_passed` as proof of the different standalone statement. The adapter maps BLAKE3 payload, public-key, verifier-context, and currentness references and preserves opaque-handle, backend, and rotation authority as explicit retained booleans.

Compatibility classifies intentionally distinct preimages, exact full-key identity, decision drift, consumer-specific issue taxonomy, non-claims, malformed/lossy refs, and unrelated-failure false parity. Legacy authority and rollback remain true. Runtime standalone authority is rejected in this change because no shell supplies exact standalone statement verification or current operational evidence; `standalone_authority_admitted` is fixed false. A later reviewed change is required for authority admission.

## Approach registry

| Family | Result |
| --- | --- |
| Reuse Molten's legacy verification boolean | Rejected: it authenticates a consumer-owned preimage, not `artifact-auth.statement.v1`. |
| Move key/backend/rotation behavior into artifact-auth | Rejected: it would transfer product authority and violate the published profile. |
| Shell-only compatibility DTO | Rejected: mapping/currentness logic would not remain a testable pure core. |
| Pure post-verification adapter with separate observation | Selected: exact statements are independently observable, differences are classifiable, and runtime authority remains Molten-owned. |

## Focused verification

Four new positive and negative adapter tests pass. They cover current and overlap verification, exact identity, mandatory non-claims, payload/context/profile/generation drift, revoked and superseded keys, malformed label-only key mapping, explicit failed standalone verification, unrelated-failure false parity, and retained signing/capability/federation/transport authority. Strict all-target Clippy and rustfmt pass for `molten-core`.

Full workspace tests, strict workspace all-target Clippy, the repository Octet check, native Cairn validation/gates, and `nix flake check -L` pass on `x86_64-linux`. Cargo generated the settled lock. The pinned unit2nix tool regenerated the default include-dev plan and the package-scoped `molten-release-policy` plan; both record settled Cargo lock SHA-256 `e9220dfe9ad04cf11f35651bcb6016bc8eb0e2dd93c19be62e9626654efbd6bc` and preserve their required test/binary targets.

These checks establish deterministic mapping and compatibility classification over supplied observations. They do not prove verifier correctness, currentness freshness, key storage, capability authority, federation membership, Preserves/Iroh behavior, runtime admission, deployment safety, or release eligibility.
