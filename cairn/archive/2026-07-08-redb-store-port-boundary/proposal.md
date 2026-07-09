## Why

Redb appears directly in several domains, including chunk storage, raft control-plane state, artifact registry, delivery idempotency, typed storage, and evaluation cache. Direct database access in domain modules makes pure planning hard to test and makes future storage backends or crate extraction risky.

## What Changes

- Define a repository-owned local-store port for index reads, index writes, atomic updates, and store diagnostics.
- Move Redb-specific table and transaction details behind an adapter boundary.
- Make domain cores return deterministic store plans or typed queries instead of opening Redb directly.
- Add positive and negative tests proving denied plans do not begin writes or mutate indexes.

## Impact

Storage behavior should become easier to test and easier to replace without changing canonical artifacts. Redb remains the first adapter, but it stops being a cross-domain implementation dependency.
