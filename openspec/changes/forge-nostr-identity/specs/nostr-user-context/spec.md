## ADDED Requirements

### Requirement: Per-user signing context for ForgeNode operations

ForgeNode operations that create signed objects SHALL accept an optional UserContext containing the user's npub and assigned ed25519 signing key.

#### Scenario: Authenticated user creates a commit

- **WHEN** a user with an authenticated npub creates a commit via the RPC API
- **THEN** the commit SHALL be signed with the user's assigned ed25519 key and the Author SHALL carry the user's npub

#### Scenario: Authenticated user creates an issue

- **WHEN** a user with an authenticated npub creates an issue
- **THEN** the COB change SHALL be signed with the user's assigned ed25519 key

#### Scenario: Unauthenticated operation uses node default key

- **WHEN** an operation is performed without user authentication
- **THEN** the operation SHALL use the ForgeNode's default secret key (backward compatible)

### Requirement: RPC handlers extract user identity from auth token

RPC handlers SHALL extract the authenticated npub from the capability token and construct a UserContext for ForgeNode operations.

#### Scenario: Request with valid token containing npub

- **WHEN** an RPC request carries a capability token with an npub claim
- **THEN** the handler SHALL look up the corresponding ed25519 keypair and pass it to ForgeNode

#### Scenario: Request without token

- **WHEN** an RPC request has no capability token and auth is not required
- **THEN** the handler SHALL use the node's default signing context

### Requirement: Web UI login via Nostr

The web UI SHALL provide a login flow that authenticates the user's npub and stores a session token.

#### Scenario: Login with NIP-07 browser extension

- **WHEN** a user clicks "Login with Nostr" and a NIP-07 extension is available
- **THEN** the UI SHALL request a signature over the challenge via window.nostr.signEvent() and authenticate

#### Scenario: Login without extension (manual nsec)

- **WHEN** no NIP-07 extension is detected
- **THEN** the UI SHALL offer a text input for the nsec with a warning about key exposure

#### Scenario: Logged-in user creates an issue via web UI

- **WHEN** a logged-in user submits the new issue form
- **THEN** the issue SHALL be attributed to their npub, not the node's default key
