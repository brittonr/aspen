## ADDED Requirements

### Requirement: Full error chain reporting

The `convert_eval_result` function SHALL walk the `std::error::Error::source()` chain and include all cause messages in the `NixEvalError.message` field, joined with ` → `.

#### Scenario: NativeError inner cause is surfaced

WHEN snix-eval returns a `NativeError { gen_type: "final_deep_force", err }` error
THEN the formatted error message SHALL include both the outer message and the inner cause
AND the message SHALL contain enough detail to identify the specific builtin or thunk that failed

#### Scenario: Single-level errors are unchanged

WHEN snix-eval returns a simple error without a cause chain (e.g., parse error)
THEN the formatted error message SHALL be identical to `format!("{e}")`
AND no ` → ` separator SHALL appear
