## ADDED Requirements

### Requirement: Lazy eval mode for flake derivation resolution

The `NixEvaluator` SHALL use `EvalMode::Lazy` when evaluating flake expressions that select a specific attribute path (`.drvPath`). The Nix expression itself forces only the selected attribute; the VM SHALL NOT deep-force the entire result tree.

#### Scenario: Flake-compat eval returns drvPath without deep forcing

WHEN a flake with a tarball input is evaluated via `evaluate_flake_via_compat`
THEN snix-eval SHALL return the `.drvPath` string without forcing unrelated parts of the flake outputs
AND the eval SHALL complete without `final_deep_force` errors
AND no `nix eval` subprocess SHALL be spawned

#### Scenario: npins eval uses lazy mode

WHEN an npins project is evaluated via `evaluate_npins_derivation`
THEN snix-eval SHALL use `EvalMode::Lazy`
AND the `.drvPath` string SHALL be extracted from the result value

### Requirement: Strict eval mode preserved for validation

The `NixEvaluator` SHALL continue to use `EvalMode::Strict` for preflight validation (`evaluate_pure`, `validate_flake`) where exhaustive error detection is the goal.

#### Scenario: Preflight validation catches all errors

WHEN `validate_flake` is called on a flake with syntax errors in multiple outputs
THEN all errors SHALL be reported, not just the first one reached during lazy evaluation

### Requirement: Lazy result value extraction

When `EvalMode::Lazy` is used, `extract_drv_path_string` SHALL handle the case where the result is still a `Value::Thunk` by forcing it before extracting the string.

#### Scenario: Thunk result is forced before extraction

WHEN lazy eval returns a `Value::Thunk` for the `.drvPath` expression
THEN the evaluator SHALL force the thunk to obtain the string value
AND SHALL return an error if forcing fails (not panic)
