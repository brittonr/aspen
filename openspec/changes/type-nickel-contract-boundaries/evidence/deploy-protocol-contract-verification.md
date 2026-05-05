## Deploy Protocol Contract Slice Verification

This slice completed the deploy-protocol portion of `I5` by fixing the generated Nickel binding order so `DeployStatusResult` no longer references `DeployNodeStatus` before it is bound. Nickel `let` bindings are not recursive.

### Command: `nix run nixpkgs#nickel -- typecheck schemas/deploy-protocol.ncl`

```text
PASS: no diagnostics
```

### Command: `UPDATE_SNAPSHOTS=1 cargo test -p aspen-ci test_deploy_protocol_schema_snapshot`

```text
PASS: regenerated schemas/deploy-protocol.ncl from DeployRequest, DeployInitResult, DeployNodeStatus, and DeployStatusResult metadata.
```

### Command: `cargo test -p aspen-ci test_deploy_protocol_schema_snapshot`

```text
PASS: test orchestrator::deploy_executor::tests::test_deploy_protocol_schema_snapshot ... ok
PASS: test result: ok. 1 passed; 0 failed
```

### Command: `python3 scripts/check-typed-nickel-contract-registry.py`

```text
typed Nickel registry OK: 12 families, 12 Crunch classifications, 6 non-candidates
```

### Command: `openspec validate type-nickel-contract-boundaries --strict --json`

```json
{
  "items": [
    {
      "id": "type-nickel-contract-boundaries",
      "type": "change",
      "valid": true,
      "issues": []
    }
  ],
  "summary": {
    "totals": {
      "items": 1,
      "passed": 1,
      "failed": 0
    }
  }
}
```

### Command: `git diff --check`

```text
PASS: no whitespace diagnostics
```
