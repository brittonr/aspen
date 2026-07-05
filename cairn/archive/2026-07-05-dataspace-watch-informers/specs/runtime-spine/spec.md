# Runtime Spine Delta: dataspace watch informers

### Requirement: Resource watch streams bind revision cursors
r[molten.resource_watch.revision_cursor_streams] Molten MUST expose resource watch streams as canonical dataspace-backed event sequences that bind resource refs, resource type, scope, generation, event kind, prior cursor, next cursor, admission receipt refs, selector refs, observer authority refs, and evidence refs. Watch cursors MUST be deterministic replay boundaries rather than wall-clock timestamps.

#### Scenario: Ordered resource events advance cursor
- GIVEN a watcher authorized for a resource scope and a sequence of admitted resource create, update, and delete events
- WHEN Molten emits watch events for that sequence
- THEN each event binds the prior cursor, next cursor, resource generation, and admission receipt refs in deterministic order.

#### Scenario: Stale cursor cannot claim continuity
- GIVEN a watcher resumes from a cursor older than the retained event window
- WHEN Molten evaluates the resume request
- THEN Molten denies continuous replay from that cursor or emits a compacted event requiring relist evidence
- AND diagnostics identify the stale cursor.

### Requirement: Informer snapshots prove list/watch consistency
r[molten.resource_watch.informer_snapshot_consistency] Molten SHOULD provide informer snapshot receipts that bind an initial list ref, starting cursor, applied watch event refs, final cursor, selector refs, observer authority refs, and cache-state ref. Informer snapshots MUST deny pass claims when events are skipped, reordered, duplicated without idempotency evidence, or applied from the wrong starting cursor.

#### Scenario: List plus watch yields current cache
- GIVEN an initial list ref at cursor `start_cursor` and ordered watch events through `end_cursor`
- WHEN the informer core applies the events to the listed resources
- THEN it emits a cache receipt binding the final cache-state ref and `end_cursor`.

#### Scenario: Missed event denies cache-current claim
- GIVEN an informer cache missing a required watch event between its list cursor and final cursor
- WHEN the informer claims to be current
- THEN Molten denies the current-cache receipt
- AND diagnostics identify the missing event range.

### Requirement: Watch selectors are authority bounded
r[molten.resource_watch.selector_authority_bounds] Molten MUST evaluate resource watch selectors against explicit observer authority, scope, policy refs, and supported selector operators before returning current assertions or future events. Broad or cross-scope selectors MUST deny unless the observer holds matching authority evidence.

#### Scenario: Authorized selector receives matching resources
- GIVEN an observer with authority for a scope and a supported label selector in that scope
- WHEN the observer opens a resource watch
- THEN Molten may deliver current and future matching resource events bound to the selector and authority evidence.

#### Scenario: Unauthorized selector denies discovery
- GIVEN an observer without cross-scope discovery authority
- WHEN the observer requests a broad selector over multiple scopes
- THEN Molten denies before revealing matching resource identities.
