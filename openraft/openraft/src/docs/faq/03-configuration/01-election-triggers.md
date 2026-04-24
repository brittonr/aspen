### How do leader elections get triggered?

**Question**: When a leader node fails or is terminated, how do followers detect this and start a new election?

**Answer**: Followers automatically trigger elections when they stop receiving messages from the leader for longer than the configured `election_timeout_max`.

**How it works**:

1. **Heartbeat mechanism**: The leader periodically sends `AppendEntries` messages (heartbeats) to all followers and learners at intervals defined by [`Config::heartbeat_interval`][].

2. **Election timeout**: Each follower maintains an internal timer. When a follower receives an `AppendEntries` message from the leader, it resets this timer.

3. **Entering candidate state**: If a follower does not receive any `AppendEntries` messages for longer than [`Config::election_timeout_max`][], it transitions to the `Candidate` state and begins a new election by requesting votes from other nodes.

4. **Required configuration**: For elections to trigger automatically, ensure:
   - [`Config::is_tick_enabled`][] = `true` (enables time-based events)
   - [`Config::is_election_enabled`][] = `true` (allows followers to become candidates)
   - The leader's [`Config::is_heartbeat_enabled`][] = `true` (enables heartbeat sending)

**Common issues**:

- **Elections not triggering**: If followers never enter candidate state after the leader fails:
  - Verify `enable_tick = true` and `enable_elect = true` in your configuration
  - Ensure at least a quorum of nodes are online and can communicate
  - Check that network connectivity allows nodes to reach each other
  - Confirm the election timeout is properly configured (typically `election_timeout_min` should be at least 3× `heartbeat_interval`)

- **Environmental differences**: Elections may work in some environments (e.g., Docker) but not others (e.g., local tests) due to:
  - Network configuration differences
  - Insufficient nodes running (need a quorum: majority of nodes must be online)
  - System clock issues or timing differences

**Timing recommendations**:

Follow the Raft timing inequality: `heartbeat_interval ≪ election_timeout ≪ MTBF` (mean time between failures)

A typical configuration:

```rust,ignore
Config {
    heartbeat_interval: 100,           // milliseconds
    election_timeout_min: 300,         // 3× heartbeat_interval
    election_timeout_max: 600,         // 2× election_timeout_min
    is_tick_enabled: true,
    is_heartbeat_enabled: true,
    is_election_enabled: true,
    ..Default::default()
}
```

[`Config::heartbeat_interval`]: `crate::config::Config::heartbeat_interval`
[`Config::election_timeout_max`]: `crate::config::Config::election_timeout_max`
[`Config::is_tick_enabled`]: `crate::config::Config::is_tick_enabled`
[`Config::is_election_enabled`]: `crate::config::Config::is_election_enabled`
[`Config::is_heartbeat_enabled`]: `crate::config::Config::is_heartbeat_enabled`
