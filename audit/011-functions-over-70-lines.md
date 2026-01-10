# Functions Exceeding 70 Line Limit (Tiger Style)

**Severity:** LOW
**Category:** Code Quality
**Date:** 2026-01-10

## Summary

4 functions exceed Tiger Style's 70-line limit for function length.

## Affected Functions

### 1. `initialize_job_system()` - 145 lines

**File:** `src/bin/aspen-node.rs:879-1024`

Contains feature-gated worker registration that could be extracted.

### 2. `setup_client_protocol()` - 134 lines

**File:** `src/bin/aspen-node.rs:1029-1163`

Initializes multiple subsystems (secrets, auth, jobs, docs).

### 3. `setup_router()` - 84 lines

**File:** `src/bin/aspen-node.rs:1166-1250`

Router builder setup with federation and handler registration.

### 4. `main()` - 70 lines

**File:** `src/bin/aspen-node.rs:442-512`

At the limit; current organization is acceptable.

## Recommendation

Extract helper functions:

```rust
// From initialize_job_system()
fn register_maintenance_workers(...) { ... }
fn register_vm_executor_worker(...) { ... }
fn register_shell_worker(...) { ... }

// From setup_client_protocol()
fn create_adapters(...) { ... }
fn initialize_features(...) { ... }
```
