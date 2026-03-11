## ADDED Requirements

### Requirement: Blob artifact download before executor

When handle_node_upgrade receives a BlobHash artifact, it SHALL download the blob via iroh-blobs to the staging directory before spawning the NodeUpgradeExecutor. The executor SHALL find the staged binary at the expected path.

#### Scenario: Blob artifact with blob_store available

- **WHEN** a NodeUpgrade request arrives with a BlobHash artifact
- **AND** the ClientProtocolContext has a non-None blob_store
- **THEN** the handler SHALL download the blob to `{staging_dir}/aspen-node-{blob_hash}`
- **AND** the handler SHALL spawn the executor only after the download completes
- **AND** the executor SHALL find the binary at the staging path

#### Scenario: Blob download timeout

- **WHEN** a blob download does not complete within the configured timeout
- **THEN** the handler SHALL respond with NodeUpgradeResult { is_accepted: false }
- **AND** the error message SHALL indicate the download timed out

#### Scenario: Blob download failure

- **WHEN** a blob download fails (hash not found, network error)
- **THEN** the handler SHALL respond with NodeUpgradeResult { is_accepted: false }
- **AND** the error message SHALL include the failure reason

### Requirement: Blob feature gate

When the blob feature is not enabled, blob-based upgrades SHALL be rejected with a clear error rather than failing at the staging path check.

#### Scenario: Blob artifact without blob feature

- **WHEN** a NodeUpgrade request arrives with a BlobHash artifact
- **AND** the blob_store is None (feature not enabled)
- **THEN** the handler SHALL respond with NodeUpgradeResult { is_accepted: false }
- **AND** the error message SHALL indicate blob upgrades require the blob feature

### Requirement: Nix artifacts unaffected

Nix store path artifacts SHALL continue to work as before, with no blob download step.

#### Scenario: Nix artifact skips blob download

- **WHEN** a NodeUpgrade request arrives with a NixStorePath artifact
- **THEN** the handler SHALL NOT attempt any iroh-blobs download
- **AND** the executor SHALL handle the Nix profile switch directly
