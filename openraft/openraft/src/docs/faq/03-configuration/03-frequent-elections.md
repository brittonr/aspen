### Frequent leader elections and timeouts

**Symptom**: Logs show repeated leader elections, or [`RaftMetrics::current_leader`][] changes frequently

**Cause**: [`Config::election_timeout_min_ms`][] is too small for your storage or network latency.
If [`RaftLogStorage::append`][] takes longer than the election timeout, heartbeats time out and
trigger elections.

**Solution**: Increase both [`Config::election_timeout_min_ms`][] and [`Config::election_timeout_max_ms`][].
Ensure `heartbeat_interval_ms < election_timeout_min_ms / 2` and that election timeout is at least
10× your typical [`RaftLogStorage::append`][] latency.

[`RaftMetrics::current_leader`]: `crate::metrics::RaftMetrics::current_leader`
[`Config::election_timeout_min_ms`]: `crate::config::Config::election_timeout_min_ms`
[`Config::election_timeout_max_ms`]: `crate::config::Config::election_timeout_max_ms`
[`RaftLogStorage::append`]: `crate::storage::RaftLogStorage::append`
