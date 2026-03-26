## ADDED Requirements

### Requirement: Federation sync returns git objects

When a remote peer requests git objects via `sync_objects` with `want_types` including "commit", "tree", or "blob", the origin cluster SHALL return the requested objects from its git bridge store.

#### Scenario: sync_objects returns commit, tree, blob

- **WHEN** Alice has a repo with a pushed commit and Bob calls `sync_objects` with `want_types: ["commit", "tree", "blob"]`
- **THEN** the response includes at least 3 objects (one commit, one tree, one blob) with non-empty data

#### Scenario: Federated git clone produces files

- **WHEN** Bob runs `git clone aspen://<ticket>/fed:<alice>:<repo>` against a repo Alice federated
- **THEN** the cloned working tree contains the files Alice committed with matching content
