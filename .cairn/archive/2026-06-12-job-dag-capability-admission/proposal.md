## Why

`job-dag-remote-admission` required capability refs but initially treated them as evidence refs only. Before target execution can be made safe, admission must validate that those refs identify concrete authority contexts that grant `job:execute` over the admitted job scope.

## What Changes

- Interpret job admission `capability` refs as authority-context refs available in the target artifact registry.
- Validate each authority context with the existing authority admission path.
- Require at least one target authority context to admit `job:execute` for the job ref.
- Bind authority admission receipt refs into admission plans and receipts.
- Deny placeholder, missing, stale, wrong-scope, or wrong-capability refs before execution.

## Impact

Job admission now uses concrete capability evidence instead of merely checking that capability-shaped refs are non-empty. This keeps remote execution fail-closed and makes future execution require a real authority context admitted at the target.
