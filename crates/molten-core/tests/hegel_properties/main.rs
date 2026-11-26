//! Bounded Hegel properties over `molten-core` pure decision cores.
//!
//! Every property draws a small bounded input, runs the pure core, and checks an invariant against
//! an oracle written independently in this test crate. Settings are derandomized and database-free,
//! so the deterministic CI rail replays the same cases on every run and a violation reports a
//! shrunk, reproducible counterexample. The `molten-core-hegel-properties` flake check requires
//! every `#[hegel::test]` declared here to be compiled into this binary and executed.

// r[verify molten.testing.molten_core_hegel_properties]

mod identity;
mod lease;
mod merge;
mod prolly;
mod reservation;
