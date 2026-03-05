## ADDED Requirements

### Requirement: PathInfoService get operation

The system SHALL retrieve PathInfo entries from Aspen Raft KV by the 20-byte Nix output hash. Entries are stored as protobuf-encoded bytes under the key `snix:pathinfo:<hex-output-hash>`.

#### Scenario: Get existing PathInfo

- **WHEN** `get()` is called with a 20-byte output hash of a stored path
- **THEN** the service SHALL return `Ok(Some(path_info))` with store_path, node, references, nar_size, nar_sha256, and signatures

#### Scenario: Get non-existent PathInfo

- **WHEN** `get()` is called with an output hash not present in KV
- **THEN** the service SHALL return `Ok(None)`

### Requirement: PathInfoService put operation

The system SHALL store PathInfo entries in Aspen Raft KV. The key MUST be derived from the store path's output hash. The value MUST be protobuf-encoded PathInfo.

#### Scenario: Store a PathInfo entry

- **WHEN** `put()` is called with a PathInfo containing store_path, node, references, nar_size, nar_sha256
- **THEN** the service SHALL encode as protobuf, store at `snix:pathinfo:<output-hash-hex>`, and return the stored PathInfo

#### Scenario: Overwrite existing PathInfo

- **WHEN** `put()` is called with a PathInfo whose output hash already exists
- **THEN** the service SHALL overwrite the existing entry and return the new PathInfo

### Requirement: PathInfoService list operation

The system SHALL return a stream of all PathInfo entries by scanning the `snix:pathinfo:` KV prefix.

#### Scenario: List all stored paths

- **WHEN** `list()` is called after storing N PathInfo entries
- **THEN** the service SHALL yield exactly N entries via the returned stream

#### Scenario: List empty store

- **WHEN** `list()` is called on a store with no PathInfo entries
- **THEN** the service SHALL yield an empty stream

### Requirement: KV key format for PathInfo

The system SHALL use the key format `snix:pathinfo:<lowercase-hex-output-hash>` where the output hash is the 20-byte Nix store path hash encoded as 40 lowercase hex characters.

#### Scenario: Key format verification

- **WHEN** a PathInfo for store path `/nix/store/abc123...-hello-1.0` is stored
- **THEN** the KV key SHALL be `snix:pathinfo:abc123...` (40 hex chars from the store path hash)

### Requirement: PathInfo protobuf encoding

The system SHALL use snix's protobuf schema (`snix_store::proto::PathInfo`) for serialization. The encoding MUST be compatible with snix's gRPC PathInfoService for interoperability.

#### Scenario: Round-trip encode/decode

- **WHEN** a PathInfo is stored via `put()` and retrieved via `get()`
- **THEN** all fields SHALL match: store_path, node, references, nar_size, nar_sha256, deriver, signatures, ca
