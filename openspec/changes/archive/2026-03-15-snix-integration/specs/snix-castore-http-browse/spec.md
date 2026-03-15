## ADDED Requirements

### Requirement: HTTP browsing of castore contents

The snix-bridge binary SHALL optionally serve an HTTP interface for browsing castore contents using `snix-castore-http`.

#### Scenario: Browse directory listing

- **WHEN** a user navigates to `http://localhost:{port}/store/{hash}-{name}/` in a browser
- **THEN** the server SHALL render an HTML directory listing of the store path's contents

#### Scenario: Download file from store path

- **WHEN** a user requests `http://localhost:{port}/store/{hash}-{name}/bin/hello`
- **AND** the path points to a regular file in the castore
- **THEN** the server SHALL stream the file contents from BlobService with appropriate content-type

#### Scenario: Symlink redirect

- **WHEN** a user requests a path that is a symlink in the castore
- **THEN** the server SHALL respond with an HTTP redirect to the symlink target

#### Scenario: Range request support

- **WHEN** a user requests a file with a `Range` header
- **THEN** the server SHALL respond with HTTP 206 and the requested byte range

### Requirement: Debug feature flag

The castore HTTP browser SHALL only be available when the `snix-http` feature flag is enabled and the `--browse` CLI flag is passed.

#### Scenario: Browser disabled by default

- **WHEN** `aspen-snix-bridge` starts without `--browse`
- **THEN** the HTTP browser SHALL NOT be served

#### Scenario: Browser enabled with flag

- **WHEN** `aspen-snix-bridge` starts with `--browse --browse-port 8080`
- **THEN** the HTTP browser SHALL be served on port 8080
