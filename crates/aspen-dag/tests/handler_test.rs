//! Unit tests for DagSyncProtocolHandler and connect_dag_sync.
//!
//! For end-to-end network tests, see patchbay_dag_sync_test.rs.
//! The patchbay tests use real iroh QUIC over simulated network namespaces
//! and are much faster (<0.2s) than relay-dependent tests (~9s).

// This file is intentionally minimal. The e2e tests live in
// patchbay_dag_sync_test.rs which covers:
// - Full sync (linear chain, diamond DAG)
// - Partial sync with known heads
// - Stem/leaf split with traversal filters
//
// The protocol and sync modules have their own unit tests in src/.
