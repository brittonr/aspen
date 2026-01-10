# Untracked Spawned Tasks (Resource Leak)

**Severity:** HIGH
**Category:** Concurrency
**Date:** 2026-01-10
**Status:** FIXED (2026-01-10)

## Summary

Multiple locations spawn tasks with `tokio::spawn` without tracking handles, preventing graceful shutdown and cleanup.

## Issue 1: System Events Bridge (Lines 145-149)

**File:** `crates/aspen-cluster/src/system_events_bridge.rs`

```rust
let service_clone = Arc::clone(service);
tokio::spawn(async move {
    if let Err(e) = service_clone.dispatch(&event).await {
        warn!(error = ?e, "failed to dispatch LeaderElected event");
    }
});  // No JoinHandle stored
```

## Issue 2: Snapshot Events Bridge (Lines 115-119)

**File:** `crates/aspen-cluster/src/snapshot_events_bridge.rs`

```rust
tokio::spawn(async move {
    if let Err(e) = service_clone.dispatch(&event).await {
        warn!(error = ?e, "failed to dispatch snapshot event");
    }
});  // Fire-and-forget
```

## Issue 3: Hooks Bridge (Lines 79-83)

**File:** `crates/aspen-cluster/src/hooks_bridge.rs`

```rust
for event in events {
    let service_clone = Arc::clone(&service);
    tokio::spawn(async move { ... });  // Unbounded spawning
}
```

## Issue 4: BufferedCounter Flush (Lines 340-351)

**File:** `crates/aspen-coordination/src/counter.rs`

```rust
tokio::spawn(async move {
    let to_flush = local.swap(0, Ordering::AcqRel);
    if to_flush > 0 {
        let _ = c.add(to_flush).await;
    }
});  // Flusher not tracked
```

## Risks

- Shutdown doesn't wait for in-flight operations
- Events can be lost during shutdown
- Unbounded task spawning can exhaust memory
- No backpressure on high-throughput scenarios

## Recommendation

Use `JoinSet` or `TaskTracker` for task management:

```rust
struct EventBridge {
    tasks: JoinSet<()>,  // Track all spawned tasks
}

impl EventBridge {
    async fn dispatch(&mut self, event: Event) {
        let service = Arc::clone(&self.service);
        self.tasks.spawn(async move {
            service.dispatch(&event).await;
        });
    }

    async fn shutdown(&mut self) {
        while let Some(_) = self.tasks.join_next().await {}
    }
}
```

## Fix Applied

The vulnerability has been fixed in all three event bridge modules:

### 1. `crates/aspen-cluster/src/hooks_bridge.rs`

- Added `JoinSet<()>` to track spawned dispatch tasks
- Added `MAX_IN_FLIGHT_DISPATCHES = 100` constant for backpressure
- Drain completed tasks before processing new events
- Apply backpressure when at capacity (wait for completion)
- Wait for in-flight dispatches on shutdown

### 2. `crates/aspen-cluster/src/system_events_bridge.rs`

- Added `JoinSet<()>` to track spawned dispatch tasks
- Modified `check_and_emit_events` to use JoinSet for task spawning
- Drain completed tasks in main loop
- Wait for in-flight dispatches on shutdown

### 3. `crates/aspen-cluster/src/snapshot_events_bridge.rs`

- Added `JoinSet<()>` to track spawned dispatch tasks
- Modified `dispatch_snapshot_event` to use JoinSet for task spawning
- Drain completed tasks in main loop
- Wait for in-flight dispatches on shutdown

All bridges now implement the Tiger Style pattern of graceful shutdown
with bounded task tracking.
