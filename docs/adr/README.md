# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records for the Aspen project.

## What is an ADR?

An Architecture Decision Record (ADR) captures an important architectural decision made along with its context and consequences. ADRs provide a historical record of key technical choices, helping current and future contributors understand why the system is designed the way it is.

## ADR Format

Each ADR follows this structure:

- **Title**: Number and descriptive name (e.g., "ADR-001: OpenRaft for Consensus Protocol")
- **Status**: Accepted, Proposed, Deprecated, or Superseded
- **Date**: When the decision was made
- **Context**: The problem and constraints
- **Decision**: The chosen solution
- **Consequences**: Trade-offs and implications
- **Alternatives Considered** (optional): Other options evaluated

## Current ADRs

### Core Infrastructure

- [001-openraft-consensus.md](001-openraft-consensus.md) - OpenRaft for Consensus Protocol
  - Vendored OpenRaft for distributed coordination and linearizable key-value storage

- [002-iroh-p2p-networking.md](002-iroh-p2p-networking.md) - Iroh for P2P Networking
  - Iroh for QUIC-based P2P networking with NAT traversal and relay support

- [003-ractor-actor-framework.md](003-ractor-actor-framework.md) - Ractor for Actor Framework
  - Ractor for actor-based concurrency and supervision trees

### Storage Layer

- [004-redb-storage.md](004-redb-storage.md) - redb for Raft State Machine Storage
  - Embedded redb database for ACID-compliant Raft log and state machine persistence

- [011-hybrid-sqlite-storage.md](011-hybrid-sqlite-storage.md) - Hybrid SQLite Storage Architecture
  - SQLite with connection pooling for production workloads and better concurrency

### Discovery and API

- [005-gossip-discovery.md](005-gossip-discovery.md) - Gossip-Based Peer Discovery
  - iroh-gossip for automatic peer discovery and cluster formation

- [007-trait-based-api.md](007-trait-based-api.md) - Trait-Based API Design
  - Narrow trait-based APIs for decoupling and testability

### Testing and Quality

- [006-madsim-testing.md](006-madsim-testing.md) - Deterministic Simulation Testing with madsim
  - Deterministic simulation for chaos engineering and correctness validation

- [008-tiger-style-philosophy.md](008-tiger-style-philosophy.md) - Tiger Style Coding Philosophy
  - Coding principles emphasizing safety, performance, and zero technical debt

### Operational Patterns

- [009-actor-supervision.md](009-actor-supervision.md) - Actor Supervision and Self-Healing
  - Supervision strategies for fault tolerance and automatic recovery

## Creating a New ADR

When making a significant architectural decision:

1. Copy an existing ADR as a template
2. Number it sequentially (next available number)
3. Write Context, Decision, and Consequences sections
4. Consider alternatives and document trade-offs
5. Submit for review via pull request
6. Update this README with a link and summary

## Status Definitions

- **Proposed**: Under discussion, not yet adopted
- **Accepted**: Decision made and actively implemented
- **Deprecated**: No longer recommended, but may still be in use
- **Superseded**: Replaced by a newer ADR (link to replacement)

## Related Documentation

- [../project_summary.md](../project_summary.md) - High-level project overview
- [../tigerstyle.md](../tigerstyle.md) - Detailed Tiger Style coding guidelines
- [../getting-started.md](../getting-started.md) - Developer onboarding guide
