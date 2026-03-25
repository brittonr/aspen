## ADDED Requirements

### Requirement: Aspen docs project exists under content/docs/aspen

The site repo SHALL contain a `content/docs/aspen/` directory with a `_index.adoc` project manifest and five sections: getting-started, architecture, operator, developer, and reference.

#### Scenario: Project manifest defines all sections

- **WHEN** `content/docs/aspen/_index.adoc` is read
- **THEN** it contains `:sections: getting-started, architecture, operator, developer, reference`

#### Scenario: Each section directory exists with _index.adoc

- **WHEN** the docs build processes `content/docs/aspen/`
- **THEN** each section listed in `:sections:` has a corresponding subdirectory with a `_index.adoc`

### Requirement: Getting Started section covers basic workflows

The getting-started section SHALL contain chapters for installation, running a single node, forming a cluster, and basic KV operations.

#### Scenario: Installation chapter exists

- **WHEN** a new user reads Getting Started
- **THEN** they find a chapter explaining how to install Aspen via Nix (`nix develop`, `nix run`)

#### Scenario: Single node quickstart exists

- **WHEN** a new user wants to try Aspen locally
- **THEN** they find a chapter with the exact commands to start a single node and interact via CLI

#### Scenario: Cluster formation chapter exists

- **WHEN** a user wants to run multiple nodes
- **THEN** they find a chapter explaining `nix run .#cluster` and verifying it works

### Requirement: Architecture section explains core design

The architecture section SHALL cover the layered design, Raft + redb storage, iroh P2P networking, ALPN routing, and the blob store.

#### Scenario: System overview chapter exists

- **WHEN** a reader opens the Architecture section
- **THEN** they find a high-level overview showing the layer diagram

#### Scenario: Consensus chapter explains Raft + redb

- **WHEN** a reader wants to understand storage
- **THEN** they find a chapter covering vendored OpenRaft, redb unified log+state, and single-fsync writes

#### Scenario: Networking chapter explains iroh transport

- **WHEN** a reader wants to understand communication
- **THEN** they find a chapter covering iroh QUIC, ALPN multiplexing, gossip, mDNS, and DHT

### Requirement: Operator section covers deployment and operations

The operator section SHALL contain chapters on deployment, configuration, and the self-hosting dogfood pipeline.

#### Scenario: Deployment chapter exists

- **WHEN** an operator wants to deploy Aspen
- **THEN** they find a chapter covering node flags, cluster formation, and NixOS module usage

#### Scenario: Dogfood pipeline chapter exists

- **WHEN** an operator wants to understand self-hosting
- **THEN** they find a chapter explaining Forge → CI → Nix build → deploy and the `dogfood-local` commands

### Requirement: Developer section covers contributing workflows

The developer section SHALL contain chapters on workspace layout, Tiger Style, testing, Verus verification, and CI.

#### Scenario: Workspace layout chapter exists

- **WHEN** a new contributor opens the Developer section
- **THEN** they find a chapter explaining crate structure, key modules, and feature flags

#### Scenario: Tiger Style chapter exists

- **WHEN** a contributor wants to understand coding conventions
- **THEN** they find a chapter covering Tiger Style principles

#### Scenario: Testing chapter exists

- **WHEN** a contributor wants to run or write tests
- **THEN** they find a chapter covering nextest, madsim, proptest, and NixOS VM tests

#### Scenario: Verus verification chapter exists

- **WHEN** a contributor wants to understand formal verification
- **THEN** they find a chapter on the two-file architecture and `nix run .#verify-verus`

### Requirement: Reference section indexes ADRs and configuration

The reference section SHALL include an ADR index, CLI reference, feature flags, and resource limits.

#### Scenario: ADR index lists all decisions

- **WHEN** a reader opens the ADR index
- **THEN** they see all ADRs listed with number, title, and content

#### Scenario: Feature flags reference exists

- **WHEN** a developer wants to know available features
- **THEN** they find a listing of all Cargo feature flags with descriptions

#### Scenario: Resource limits reference exists

- **WHEN** someone wants to know system limits
- **THEN** they find the Tiger Style constants (MAX_BATCH_SIZE, MAX_KEY_SIZE, timeouts, etc.)

### Requirement: Docs template renders with sidebar and navigation

Doc chapters SHALL render using `docs.html.erb` inside `docs-layout.html.erb`, producing pages with sidebar navigation and prev/next links.

#### Scenario: Template produces sidebar and content

- **WHEN** a docs chapter is rendered
- **THEN** the output HTML contains a sidebar `<nav>` and a main content area with the chapter body

#### Scenario: Site header includes Docs link

- **WHEN** any page on the site renders
- **THEN** the header navigation includes a "Docs" link pointing to `/docs/aspen/`
