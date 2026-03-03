## ADDED Requirements

### Requirement: Generate root capability token

The CLI SHALL provide `token generate` to create a root capability token signed by a provided Ed25519 secret key. The token SHALL include specified capabilities and lifetime, and SHALL be output as a base64 string compatible with the `--token` global flag.

#### Scenario: Generate root token with default capabilities

- **WHEN** operator runs `aspen-cli token generate --secret-key-file ./key.secret --lifetime 365d`
- **THEN** the CLI SHALL output a base64-encoded `CapabilityToken` with `Full { prefix: "" }`, `ClusterAdmin`, and `Delegate` capabilities, expiring 365 days from now

#### Scenario: Generate token with specific capabilities

- **WHEN** operator runs `aspen-cli token generate --secret-key-file ./key.secret --lifetime 24h --cap read:logs/ --cap write:data/`
- **THEN** the CLI SHALL output a token with only `Read { prefix: "logs/" }` and `Write { prefix: "data/" }` capabilities

#### Scenario: Secret key from environment variable

- **WHEN** `ASPEN_SECRET_KEY` is set to a hex-encoded Ed25519 secret key
- **AND** operator runs `aspen-cli token generate --lifetime 1h`
- **THEN** the CLI SHALL use the environment variable key to sign the token

#### Scenario: Missing secret key

- **WHEN** operator runs `aspen-cli token generate --lifetime 1h` without `--secret-key-file` or `ASPEN_SECRET_KEY`
- **THEN** the CLI SHALL exit with an error message indicating a secret key is required

#### Scenario: Invalid lifetime format

- **WHEN** operator runs `aspen-cli token generate --secret-key-file ./key.secret --lifetime abc`
- **THEN** the CLI SHALL exit with an error message indicating the lifetime format is invalid

### Requirement: Inspect capability token

The CLI SHALL provide `token inspect` to decode a base64-encoded capability token and display its metadata in human-readable format. Inspection SHALL work offline without any cluster connection.

#### Scenario: Inspect token with human-readable output

- **WHEN** operator runs `aspen-cli token inspect <base64-token>`
- **THEN** the CLI SHALL display the token's issuer (public key), audience, capabilities, issued-at time, expires-at time, delegation depth, and whether the token has a nonce

#### Scenario: Inspect token with JSON output

- **WHEN** operator runs `aspen-cli token inspect --json <base64-token>`
- **THEN** the CLI SHALL output the token metadata as a JSON object

#### Scenario: Inspect expired token

- **WHEN** operator inspects a token whose `expires_at` is in the past
- **THEN** the output SHALL include an indication that the token is expired

#### Scenario: Inspect invalid token

- **WHEN** operator runs `aspen-cli token inspect <invalid-base64>`
- **THEN** the CLI SHALL exit with an error message indicating the token could not be decoded

### Requirement: Delegate child token from parent

The CLI SHALL provide `token delegate` to create a child token from a parent token. The child token SHALL have capabilities that are subsets of the parent's capabilities, and the delegation depth SHALL increment by one.

#### Scenario: Delegate with reduced capabilities

- **WHEN** operator has a parent token with `Full { prefix: "" }` and `Delegate`
- **AND** runs `aspen-cli token delegate --parent <parent-base64> --secret-key-file ./key.secret --lifetime 1h --cap read:logs/`
- **THEN** the CLI SHALL output a child token with only `Read { prefix: "logs/" }` capability and delegation depth incremented by 1

#### Scenario: Delegation fails without Delegate capability

- **WHEN** operator has a parent token without the `Delegate` capability
- **AND** runs `aspen-cli token delegate --parent <parent-base64> --secret-key-file ./key.secret --lifetime 1h --cap read:logs/`
- **THEN** the CLI SHALL exit with an error indicating the parent token does not allow delegation

#### Scenario: Capability escalation rejected

- **WHEN** operator has a parent token with `Read { prefix: "logs/" }`
- **AND** runs `aspen-cli token delegate --parent <parent-base64> --secret-key-file ./key.secret --lifetime 1h --cap write:data/`
- **THEN** the CLI SHALL exit with an error indicating the requested capability exceeds the parent's scope

#### Scenario: Maximum delegation depth exceeded

- **WHEN** operator has a parent token at delegation depth equal to `MAX_DELEGATION_DEPTH`
- **AND** attempts to delegate a child token
- **THEN** the CLI SHALL exit with an error indicating the maximum delegation depth has been reached

### Requirement: Capability specification format

The CLI SHALL accept capabilities via repeated `--cap` flags using a `type:prefix` string format that maps to `Capability` enum variants.

#### Scenario: Parse KV capabilities

- **WHEN** operator provides `--cap full:myapp/ --cap read:logs/ --cap write:data/ --cap delete:tmp/`
- **THEN** the CLI SHALL parse these as `Full { prefix: "myapp/" }`, `Read { prefix: "logs/" }`, `Write { prefix: "data/" }`, `Delete { prefix: "tmp/" }`

#### Scenario: Parse admin capabilities

- **WHEN** operator provides `--cap cluster-admin --cap delegate`
- **THEN** the CLI SHALL parse these as `ClusterAdmin` and `Delegate`

#### Scenario: Parse shell capabilities

- **WHEN** operator provides `--cap shell:pg_*`
- **THEN** the CLI SHALL parse this as `ShellExecute { command_pattern: "pg_*", working_dir: None }`

#### Scenario: Invalid capability format

- **WHEN** operator provides `--cap unknown:foo`
- **THEN** the CLI SHALL exit with an error listing valid capability types

### Requirement: Lifetime duration parsing

The CLI SHALL accept human-readable duration strings for the `--lifetime` flag using day/hour/minute/second suffixes.

#### Scenario: Parse duration suffixes

- **WHEN** operator provides `--lifetime 365d`, `--lifetime 24h`, `--lifetime 30m`, or `--lifetime 3600s`
- **THEN** the CLI SHALL parse these as 365 days, 24 hours, 30 minutes, and 3600 seconds respectively

#### Scenario: Default to seconds without suffix

- **WHEN** operator provides `--lifetime 3600`
- **THEN** the CLI SHALL parse this as 3600 seconds
