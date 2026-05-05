## Rust-Derived Receipt Contract Slice Verification

### Command: python3 scripts/generate-typed-nickel-contracts.py --check

```text
typed Nickel generated contracts fresh: 2 files
```

### Command: nix run nixpkgs#nickel -- typecheck schemas/dogfood-run-receipt.ncl

```text
(pass: no diagnostics)
```

### Command: nix run nixpkgs#nickel -- typecheck schemas/ci-run-receipt.ncl

```text
(pass: no diagnostics)
```

### Negative mutation: generated stale contract is rejected

Mutation applied temporarily:

```diff
-  pipeline_name | String,
+  pipeline_name | Number,
```

Checker output:

```text
stale generated contract: schemas/ci-run-receipt.ncl
--- schemas/ci-run-receipt.ncl (current)
+++ schemas/ci-run-receipt.ncl (expected)
@@ -44,7 +44,7 @@
 let CiRunReceipt = {
   schema | String,
   run_id | String,
-  pipeline_name | Number,
+  pipeline_name | String,
   repo_id | String,
   ref_name | String,
   commit_hash | String,
```

The mutated file was restored before continuing.

### Command: python3 scripts/check-typed-nickel-contract-registry.py

```text
typed Nickel registry OK: 12 families, 12 Crunch classifications, 6 non-candidates
```

### Command: openspec validate type-nickel-contract-boundaries --strict --json

```json
{
  "items": [
    {
      "id": "type-nickel-contract-boundaries",
      "type": "change",
      "valid": true,
      "issues": [],
      "durationMs": 1
    }
  ],
  "summary": {
    "totals": {
      "items": 1,
      "passed": 1,
      "failed": 0
    },
    "byType": {
      "change": {
        "items": 1,
        "passed": 1,
        "failed": 0
      }
    }
  },
  "version": "1.0"
}
```

### Command: git diff --check

```text
(pass: no diagnostics)
```
