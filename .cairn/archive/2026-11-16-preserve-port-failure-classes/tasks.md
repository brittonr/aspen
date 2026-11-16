## Phase 1: Implementation

- [x] [serial] Add `FailureClass` and `Failure::Classified`, convert every fabric port class to it and back, and display the class instead of "invalid harness artifact". r[molten.modularity.fabric_boundary.errors]
- [x] [serial] Raise native lifecycle uncertainty as an uncertain-external-outcome failure and select the lifecycle control detail by class; update the collaboration-scope denial-code parser, harness kind codes and AGENTS.md. r[molten.modularity.fabric_boundary.errors]
- [x] [serial] Run the round-trip, display and lifecycle-selection tests, the harness and collaboration-scope suites, formatting, Clippy and source/lifecycle gates before review, sync and archive. r[molten.modularity.fabric_boundary.errors]

## Verification (2026-11-16)

Integration tree with the #43, #44, #57 and #58 changes applied: `cargo fmt --check`; `cargo test -p molten-node-host --lib error` (1 passed); `cargo test -p molten --lib fabric::port` (4 passed, including `every_port_class_survives_conversion_both_ways`); `cargo test -p molten --bin molten lifecycle` (9 passed, including `lifecycle_uncertainty_is_selected_by_class_not_message_text`); `cargo test -p molten --lib harness` (15 passed); `cargo test -p molten --lib collaboration_scope` (36 passed); `cargo clippy -p molten-node-host --lib --tests -- -D warnings` and `cargo clippy -p molten --lib --bins --tests -- -D warnings` passed. These are native test observations, not live port or deployment qualification.
