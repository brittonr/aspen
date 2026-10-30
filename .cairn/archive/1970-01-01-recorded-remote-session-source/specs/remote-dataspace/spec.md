# Recorded remote session source

## ADDED Requirements

### Requirement: Capability-authenticated local source
r[molten.remote_dataspace.recorded_session_source] Molten MUST bind a local recorded session to capabilities for both initialized node identities, a topic, and a receiver-allocated monotonic generation. It MUST atomically persist bounded ordered canonical observations and their predecessor hashes. A sender MUST derive introductions only from the exact current open generation's committed history and established identity/session bootstrap, never from caller-carried reference lists or snapshots.

#### Scenario: Positive committed introduction
- GIVEN the receiver commits an assertion naming a reference
- WHEN the same generation builds a message carrying that reference
- THEN the message can be built without granting delivery authority.

#### Scenario: Retraction and close
- GIVEN a live introducing assertion
- WHEN its last occurrence retracts or its generation closes
- THEN later builds carrying the reference refuse before envelope publication.

#### Scenario: Currentness and corruption
- GIVEN a stale generation, mismatched peer/topic, gap, or changed predecessor hash
- WHEN the sender builds
- THEN no envelope is returned and canonical denial evidence identifies the refusal.

#### Scenario: Retraction after build
- GIVEN a referenced message was built while its introduction was live
- WHEN the introducing assertion retracts before local publication
- THEN publication rechecks the exact source and refuses before writing the envelope.

#### Scenario: Metadata is not an introduction
- GIVEN a protocol message carries protocol/schema/evidence metadata and an application body reference
- WHEN only its metadata has appeared in earlier traffic
- THEN the application body reference remains unknown; metadata never becomes a transferred capability or establishes knowledge of it.

#### Scenario: Capacity preserves close
- GIVEN a valid live recording reaches its nonterminal observation or byte bound
- WHEN a normal operation and then close are requested
- THEN the normal operation refuses without mutation and the reserved terminal capacity permits close and monotonic readmission.

#### Scenario: Local evidence is not a live acknowledgment
- GIVEN a referenced envelope and only a local/recorded source profile
- WHEN an unbound live gossip publication is requested
- THEN it refuses rather than treating the local recording as authenticated network-receiver knowledge.
