## ADDED Requirements

### Requirement: CLI login with local signing

The CLI SHALL authenticate with a Nostr nsec by signing the challenge locally and sending only the signature to the server. The nsec SHALL NOT be transmitted over the network.

#### Scenario: Successful CLI login

- **WHEN** user runs `aspen-cli auth login` and provides their nsec
- **THEN** the CLI SHALL fetch a challenge, sign it locally, verify with the server, and store the token

### Requirement: Token persistence

The CLI SHALL store the received token in a local file for reuse by subsequent commands.

#### Scenario: Token stored after login

- **WHEN** authentication succeeds
- **THEN** the token SHALL be written to `~/.config/aspen/token`

#### Scenario: Subsequent command uses stored token

- **WHEN** a CLI command is run after login and a stored token exists
- **THEN** the command SHALL include the token in its RPC requests
