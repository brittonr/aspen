## Why

Job DAG code spans planning, admission, scheduling, worker execution, blob-ref IO, coordination, receipts, and CLI UX. Keeping these concerns in one broad module makes it harder to prove scheduling and execution decisions independently from local IO or transport behavior.

## What Changes

- Separate DAG planning, admission, scheduling, worker execution, blob-ref IO, coordination adapter, and CLI shells.
- Make pure job cores return admitted plans, assignments, and execution intents without performing storage, transport, or process effects.
- Add positive and negative tests for scheduling and execution boundaries.

## Impact

Job workflows become easier to test and reason about. Worker execution remains evidence-bearing and cannot be triggered by blob presence, queue availability, or transport delivery alone.
