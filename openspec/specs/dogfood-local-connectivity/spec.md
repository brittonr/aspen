# dogfood-local-connectivity

## Purpose

Defines the Dogfood Local Connectivity capability requirements preserved by Aspen's archived OpenSpec records.

## Requirements

### Requirement: Local node discovery

Dogfood nodes spawned on the same machine MUST discover each other without relay servers or mDNS.

#### Scenario: Local nodes discover peers

- **WHEN** dogfood spawns local nodes with relay servers and mDNS disabled
- **THEN** the nodes MUST discover each other through local cluster discovery

### Requirement: Client connectivity

The dogfood binary's AspenClient MUST connect to spawned nodes within 10 seconds when relay is disabled.

#### Scenario: Client connects without relay

- **WHEN** the dogfood binary starts an AspenClient against spawned local nodes
- **THEN** the client MUST connect within 10 seconds without relay connectivity

### Requirement: Federation trust establishment

Alice and bob clusters MUST successfully exchange AddPeerCluster RPCs and establish bidirectional federation trust.

#### Scenario: Federation trust is established

- **WHEN** alice and bob clusters exchange AddPeerCluster RPCs
- **THEN** bidirectional federation trust MUST be established

### Requirement: Git push through federation

A git push to alice's forge MUST succeed, and bob MUST be able to sync the objects via federation protocol.

#### Scenario: Bob syncs pushed objects

- **WHEN** a git push succeeds against alice's forge
- **THEN** bob MUST sync the pushed objects through the federation protocol

### Requirement: Large repo federation sync

A repo with 100+ files in nested directories (3 levels) MUST sync completely from alice to bob. DAG integrity check on bob MUST report 0 missing objects.

#### Scenario: Large repository sync remains complete

- **WHEN** alice federates a repository with 100+ files across nested directories
- **THEN** bob MUST receive a complete sync and report 0 missing DAG objects
