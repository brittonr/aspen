//! Companion worker for `nickel-eval.captured-modules.v1`.
//!
//! It is linked to the exact producer library and handles one strict request on stdin.
//! Hosts launch it only through `nickel_eval::captured::evaluate_isolated` with an
//! explicitly selected absolute path; it never reads files or the environment itself.
fn main() {
    std::process::exit(nickel_eval::closed::worker::run_stdio());
}
