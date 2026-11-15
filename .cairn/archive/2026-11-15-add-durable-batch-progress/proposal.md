# Durable accepted batch progress

## Why
Issues #36/#37 require reconnectable observation of controller-accepted artifacts, not worker messages. The standalone pilot already owns durable task state and output bytes.

## What changes
Versioned accepted-output and terminal-run projections, exact attempt fences, bounded cursor reads, snapshot recovery, and explicit artifact gaps over the existing admitted Iroh RPC. State, progress events, and stream position share one Redb transaction. Reuse the existing committed-stream source; do not establish a second transport or job ledger.

## Non-goals
No cross-host qualification, GC authority, HA, partial pixel execution, or inferred executor stop.
