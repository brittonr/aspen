# Guest VM Code Safety Gaps

**Severity:** MEDIUM
**Category:** Code Safety
**Date:** 2026-01-10

## Summary

The no_std guest VM binary has unsafe code with missing safety documentation.

## Affected Location

**File:** `crates/aspen-jobs-guest/src/lib.rs`

### Issue 1: FFI Pointer Conversion (Lines 163-168)

```rust
let input = unsafe {
    if input_ptr.is_null() || input_len == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(input_ptr, input_len)
    }
};
```

**Missing:**

- SAFETY comment explaining invariants
- Alignment validation
- Documentation of Hyperlight ABI contract

### Issue 2: Mutable Static State (Lines 175-182, 189, 210)

```rust
unsafe {
    RESULT_BUFFER = Some(output.clone());
    // ...
}
```

**Missing:**

- SAFETY comment documenting single-threaded execution assumption
- No mutual exclusion between `execute()`, `get_result_len()`, and `hyperlight_main()`

## Recommendation

Add comprehensive SAFETY comments:

```rust
// SAFETY: This function is called from the Hyperlight VM host which guarantees:
// 1. input_ptr is either null or points to valid memory of size input_len
// 2. The memory is properly aligned for u8
// 3. The memory remains valid for the duration of this function
// 4. Execution is single-threaded within the VM guest
let input = unsafe {
    if input_ptr.is_null() || input_len == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(input_ptr, input_len)
    }
};
```
