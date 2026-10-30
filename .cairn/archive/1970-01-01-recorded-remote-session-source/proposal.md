# Proposal: Capability-authenticated recorded remote sessions

## Why

The introduction rule cannot rely on an offline caller's bootstrap list or runtime snapshot. Molten owns the missing source: a receiver-committed session history, generation fencing, and a sender view authenticated by the same capability-rooted local recording owner.

## What Changes

Add a separately versioned local/recorded session profile. Opening requires capabilities to initialized sender and receiver node roots, binds their actual identities and topic, allocates a receiver-owned monotonic generation, and establishes only those identity references and the resulting session reference. Receiver operations commit atomically into a bounded canonical hash-chained history. A sender reads and validates that history through the admitted receiver capability on every build; no public snapshot or reference-list import exists. Closing/reopening fences earlier views. Assertions introduce references; retractions withdraw them. Message refusal emits canonical evidence before construction/publication.

## Non-goals

No network admission protocol, remote trust bootstrap, authority grant, old-wire error reply, Syndicate compatibility, or automatic synchronization. Access to a recording root is an explicit local capability, not evidence of network-peer authorization. The transport must not treat this recording as remote admission.
