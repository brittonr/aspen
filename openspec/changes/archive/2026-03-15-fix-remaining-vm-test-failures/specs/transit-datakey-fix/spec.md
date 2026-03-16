## ADDED Requirements

### Requirement: Transit datakey returns plaintext by default

The WASM secrets plugin's `handle_datakey` SHALL return the plaintext data key in the response unless the caller explicitly requests `key_type = "wrapped"`.

#### Scenario: Default datakey request includes plaintext

- **WHEN** a caller sends `SecretsTransitDatakey` with `key_type = "aes256-gcm"`
- **THEN** the response SHALL have `is_success: true`, `plaintext: Some(...)`, and `ciphertext: Some(...)`

#### Scenario: Plaintext mode explicitly requested

- **WHEN** a caller sends `SecretsTransitDatakey` with `key_type = "plaintext"`
- **THEN** the response SHALL have `is_success: true`, `plaintext: Some(...)`, and `ciphertext: Some(...)`

#### Scenario: Wrapped mode excludes plaintext

- **WHEN** a caller sends `SecretsTransitDatakey` with `key_type = "wrapped"`
- **THEN** the response SHALL have `is_success: true`, `plaintext: None`, and `ciphertext: Some(...)`

### Requirement: SOPS client sends correct datakey return mode

The SOPS TransitClient `generate_data_key` method SHALL send `key_type = "plaintext"` in the `SecretsTransitDatakey` RPC request.

#### Scenario: SOPS encrypt generates data key with plaintext

- **WHEN** `aspen-sops encrypt` is called with a Transit key
- **THEN** the `SecretsTransitDatakey` RPC request SHALL contain `key_type: "plaintext"`
- **AND** the returned plaintext data key SHALL be non-empty

### Requirement: sops-transit VM test passes end-to-end

The `sops-transit-test` NixOS VM test SHALL pass, including the `transit generate data key` and `transit full envelope encryption flow` subtests.

#### Scenario: Full sops-transit test suite passes

- **WHEN** `nix build .#checks.x86_64-linux.sops-transit-test` is run
- **THEN** all subtests SHALL pass including datakey generation, encrypt/decrypt round-trip, key rotation, SOPS file encrypt, Go SOPS interop, and multi-key-group interop
