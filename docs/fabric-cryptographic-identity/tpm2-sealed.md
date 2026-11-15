# Aspen TPM2-sealed transport identity

## Decision and reviewed sources

`ManagedSecret` was an eligibility label, **not an implemented loader**. At the
reviewed source cohort, [the backend enum](https://github.com/brittonr/aspen/blob/35478ffb0a6291e2dad583f9bec62fb26b4b10ae/crates/molten-core/src/fabric_crypto_identity/model.rs#L80-L98)
listed it, while [the actual loader](https://github.com/brittonr/aspen/blob/35478ffb0a6291e2dad583f9bec62fb26b4b10ae/src/fabric_crypto_identity/file/parts/adapter/p002/body.rs#L202-L222)
read a capability-rooted regular file. [The constructor](https://github.com/brittonr/aspen/blob/35478ffb0a6291e2dad583f9bec62fb26b4b10ae/src/fabric_crypto_identity/file/parts/adapter/p001/body.rs#L10-L16)
required `CapabilityFile`; no systemd-creds, Vault or TPM service implemented
`ManagedSecret`. This follows the [full issue decision](https://github.com/brittonr/aspen/issues/9#issuecomment-6768827011),
not an inference that managed eligibility implies availability.

Aspen now has a separate `KeyBackendClass::Tpm2Sealed` and
`fabric_crypto_identity::Sealed` shell. The existing internal
`molten` crate/CLI names and canonical schemas remain unchanged. This shell
unseals a transport seed into process memory because Iroh consumes an
`iroh::SecretKey`. It is **not PKCS11**: it does not keep an unextractable
signing key inside a token, wrap OpenSSL applications, import nixpkcs, implement
remote attestation, or change membership/capability/source-trust admission.
Federation, delegation, evidence and authority signing remain ineligible for
this backend. `ManagedSecret` still has no new loader.

## Provisioned backend contract

The Nickel selected-backend contract also restricts `tpm2-sealed` to exactly
`["transport-endpoint"]`. Its positive shape fixture uses template references
for contract testing only, not provisioned provider identities. A negative
fixture rejects the full signing-purpose list; the existing negative-profile
Nix loop already discovers it.

The composition root supplies an independently admitted production `Profile`,
an identity/secrets `Scope`, a retained `node_state::Executable`, and a
`Tpm2SealedPolicy`. The policy contains:

- a nonempty, sorted, unique SHA256 PCR selection in the range 0–23;
- exact 80-byte marshaled `TPM2B_PUBLIC` for a SHA256 keyed-hash sealed object:
  `fixedTPM|fixedParent`, a 32-byte nonzero `authPolicy`, NULL scheme and
  SHA256 unique value; `userWithAuth` is forbidden to prevent password bypass;
- the persistent object handle and explicit provider class
  (`LinuxResourceManager` at `/dev/tpmrm0`, or disposable `SwtpmWorkspace`);
- independently selected provider and executable BLAKE3 references.

`fabric_crypto_identity::backend_ref` hashes a versioned canonical Preserves record binding every
field, including the public object's byte identity and PCR selection. A raw
TPM handle alone is not backend identity. Tool measurement reads the same
retained executable inode later invoked by bounded execution. The swtpm socket
is always `swtpm.sock` relative to the retained namespace workspace; there is no
inherited TCTI, environment search, shell command construction or remote
provider selection. The Linux resource manager is an explicitly selected host
capability, not a claim of a capability-contained TPM device driver.

The actual provider must provision a policy-only sealed **48-byte MCKEY001
transport record** (8-byte schema, nonzero big-endian u64 generation, 32-byte
OS-CSPRNG Ed25519 seed), rather than a PKCS11 URI. Provisioning is a separate
operator-authorized action and is never performed during unseal or startup.
Neither a raw seed nor a plaintext record belongs in consumer-controlled files,
Nix expressions, the Nix store, command arguments, environment values or
diagnostic artifacts. Public policy/object metadata and encrypted TPM
private/context blobs are not seeds. Software-provider device state is a
separate sensitive authority, not an encrypted blob supplied to the adapter;
see the disposable rail's boundary below. Measured boot policy selection and
update/recovery authority remain operator responsibilities; the fixture's PCR
16 is not a production boot recommendation.

Use a dedicated backend namespace and keep its TPM object/provisioning authority
stable during a load. The shell compares `readpublic` with the exact admitted
object before and after unseal, but does not claim an atomic snapshot against
concurrent privileged TPM-owner replacement. Its trust boundary includes the
approved immutable tool closure, kernel, TPM transport and provisioning owner.
The Nix wrapper fixes the tool basename for tpm2's multicall dispatcher when
its outer executable is invoked through the retained `/proc/.../fd/...` path.

## Secret delivery and typed failures

`resolve()` returns only a canonical opaque transport handle and public key.
`load_transport_secret(&handle)` unseals again, checks the complete current
handle, generation, public identity, purpose and exact backend reference, and
returns the secret directly for `iroh::Endpoint::builder(...).secret_key(...)`.
The shell does not write plaintext or publish process stdout/stderr through
execution evidence. Each process has a five-second deadline, one-second
teardown budget, process-group termination, 48/80-byte secret/public stdout
bounds, and a 4096-byte diagnostic bound. Raw streams/errors are never logged.
Temporary secret buffers are overwritten on ordinary completion; this is not
a guarantee of compiler-proof zeroization, locked memory, swap/core-dump
exclusion or protection from a privileged process inspecting memory.

`Tpm2IdentityFailure` preserves invalid policy, denied profile, unavailable
capability/process, tool mismatch, object mismatch, timeout, output limit,
unseal denial, malformed record, mismatched handle and rotation denial. None
selects `CapabilityFile`, generates a replacement key, or grants authority.
The namespace's durable transport revocation marker is also honored.

## Rotation and activation

`plan_key_rotation` retains exact source handle/public identity/generation
checks and treats `backend_class` plus `backend_ref` as the exact destination.
Same-backend plans retain their existing behavior. The only newly admitted
cross-class transitions are `CapabilityFile` ↔ `Tpm2Sealed`, for transport
purpose and profile-admitted classes. `complete_key_rotation` still checks the
actual destination's exact backend, profile, purpose, generation and
currentness. `Sealed::complete_rotation` resolves the real unsealed
destination before issuing the existing transition outcome.

This is not automatic activation: the caller must preserve the existing
policy/intent/fenced activation/revocation protocol, retain no-overlap or
reviewed verification-only overlap policy, and choose the active backend
explicitly. The swtpm test prepares real destination key state in each
backend and checks both transition directions; a successful completion outcome
does not itself revoke the old external backend or activate node configuration.
The default node first-boot/daemon path remains capability-file. No default
NixOS service backend switch or automatic hardware mutation is introduced.

## Pinned disposable qualification rail

`nix/tpm2-identity.nix` uses the root flake's existing pinned nixpkgs cohort
`f9d8b65950353691ab56561e7c73d2e1063d810b`: tpm2-tools **5.7**, swtpm **0.10.1**.
The upstream [unseal/PCR authorization contract](https://github.com/tpm2-software/tpm2-tools/blob/5.7/man/tpm2_unseal.1.md)
and [policy-only creation attributes](https://github.com/tpm2-software/tpm2-tools/blob/5.7/man/tpm2_create.1.md)
are the executable tool interface, not copied provider implementation.

Parent-owned root integration is exactly this check inside the existing
per-system `checks` set, where `moltenTestBinaries` and `rustLibDir` already
exist:

```nix
tpm2-sealed-identity = import ./nix/tpm2-identity.nix {
  inherit pkgs rustLibDir;
  testBinaries = moltenTestBinaries;
};
```

No Cargo dependency/feature, lock, generated JSON or build-plan edit is needed.
The parent should additionally export
`docs/fabric-cryptographic-identity/fixtures/positive/tpm2-sealed.ncl` in the
existing `fabricCryptographicIdentityProfileCheck` next to its default export;
the default generated profile remains unchanged. The matching negative fixture
is picked up by its existing glob without a second convention.
The module discovers exactly one real library test executable, starts only a
private swtpm Unix socket, runs the ignored consumer with a finite timeout and
kills only its own emulator. The Rust fixture generates the record in memory,
seals it via bounded stdin, loads the real Iroh transport secret, tests exact
tool/object/handle mismatches, completes both real-destination rotation plans,
scans every regular file recursively in the TPM consumer namespace for the
record/seed/plaintext schema, performs an actual selected-PCR extension, and
requires typed `UnsealDenied` afterward. The separate capability-file rotation
namespace is also scanned for the TPM record and seed; its own plaintext key
format is intentional capability-file behavior, not a TPM fallback. Both
inventories must contain regular files. Failures identify the matching file and
pattern class without printing any file contents or secret bytes. No absent
tool or missing fixture is converted into a skip/success.

### Provider state is not consumer-owned storage

`MOLTEN_TPM2_FIXTURE_ROOT=$TMPDIR/tpm2-fixture` owns the socket and all files
created by provisioning tools and the adapter. No matching file or subdirectory
is excluded from its positive plaintext scan. The emulator's
`--tpmstate dir=$TMPDIR/tpm2-provider-state` is a **sibling**, outside the
namespace capability retained by the consumer. It belongs to the fixture's
provider/device-state authority, not the sealed adapter. This is an ownership
separation, not OS isolation from another process with the same UID.

The first real pinned consumer execution reached both cross-backend rotations
and then detected the complete secret record in a regular file. The assertion
did not report a filename, so the failed output alone does not establish which
file matched. Source inspection establishes the old ownership error: the Nix
runner placed unencrypted emulator state in the consumer's scanned `state/`
subdirectory. The adapter only reads public metadata and captures unseal stdout
in memory; provisioning supplies the record on stdin and writes named public,
encrypted private and context blobs. In contrast,
[libtpms persistent-object serialization](https://github.com/stefanberger/libtpms/blob/v0.10.2/src/tpm2/NVMarshal.c)
marshals persistent objects including their sensitive keyed-hash payload. The
[swtpm 0.10.1 state contract](https://github.com/stefanberger/swtpm/blob/v0.10.1/man/man8/swtpm.pod)
writes device state to `--tpmstate` and requires a separate `--key` option to
enable state-file encryption; this disposable rail supplies no such key.
Consequently this software provider can persist the sealed record in its own
device-state files. Relocating that authority fixes the fixture boundary; it
does not make those files secret-free or demonstrate physical-TPM storage
secrecy. Neither provider state nor secret bytes are published as evidence.
The parent subsequently executed the corrected Nix swtpm harness against the
current Cargo-built library test executable: the selected real consumer passed.
This exercised both rotations, all consumer-file scans and the actual PCR
extension followed by typed `UnsealDenied`. It is not a hermetic build of the
complete Nix test-binary package. The secret-free consumer log and actual cohort
are retained under the change's `evidence/swtpm-2026-11-13/` directory.

Parent verification, after integration and only once all slices settle:

```sh
cargo test -p molten-core fabric_crypto_identity
cargo test -p molten --lib fabric_crypto_identity
nickel export docs/fabric-cryptographic-identity/fixtures/positive/tpm2-sealed.ncl --format json
nix build .#checks.x86_64-linux.fabric-cryptographic-identity-profile -L
nix build .#checks.x86_64-linux.tpm2-sealed-identity -L
```

The focused Cargo run intentionally does not run the ignored external fixture;
the Nix check must run it explicitly. Its retained artifacts are
`result/consumer.log` (test runner output only) and `result/cohort.txt` (actual
pinned package versions and scope), never the emulator state or seed. These are
bounded swtpm test observations, not canonical production startup evidence,
physical TPM qualification, host-filesystem secrecy for the software provider,
measured-boot deployment, attestation or release eligibility. The original
plaintext assertion failure remains historical evidence; the corrected
software-provider consumer pass does not qualify a physical TPM or encrypt
the software provider's separate device-state files.
