include!("../../../../../examples/component-extension/src/allocator.rs");

#[test]
fn arena_reclaims_cross_word_ranges_without_overlapping_live_allocations() {
    // r[verify molten.wasm_component_adoption.validation]
    let arena = Arena;
    let small = core::alloc::Layout::from_size_align(17, 16).expect("small layout");
    let large = core::alloc::Layout::from_size_align(2049, 64).expect("cross-word layout");
    let first = unsafe { arena.alloc(small) };
    let second = unsafe { arena.alloc(large) };
    assert!(!first.is_null());
    assert!(!second.is_null());
    assert_eq!(second as usize % 64, 0);
    assert!(second as usize >= first as usize + small.size());
    unsafe {
        core::ptr::write_bytes(second, 0x5a, large.size());
    }
    unsafe {
        arena.dealloc(first, small);
    }
    let reclaimed = unsafe { arena.alloc(small) };
    assert_eq!(reclaimed, first);
    assert!(unsafe { core::slice::from_raw_parts(second, large.size()) }.iter().all(|byte| *byte == 0x5a));
    unsafe {
        arena.dealloc(reclaimed, small);
        arena.dealloc(second, large);
    }
    resized_ranges_preserve_live_bytes(&arena);
    let whole = core::alloc::Layout::from_size_align(BLOCK_BYTES * BLOCK_COUNT, 16).expect("whole arena");
    let full = unsafe { arena.alloc(whole) };
    assert!(!full.is_null());
    assert!(unsafe { arena.alloc(small) }.is_null());
    unsafe {
        arena.dealloc(full, whole);
    }
    let full_again = unsafe { arena.alloc(whole) };
    assert_eq!(full_again, full);
    unsafe {
        arena.dealloc(full_again, whole);
    }
}

fn resized_ranges_preserve_live_bytes(arena: &Arena) {
    let large = core::alloc::Layout::from_size_align(2049, 64).expect("large layout");
    let small = core::alloc::Layout::from_size_align(17, 64).expect("small layout");
    let first = unsafe { arena.alloc(large) };
    assert!(!first.is_null());
    unsafe {
        core::ptr::write_bytes(first, 0x37, small.size());
    }
    let shrunk = unsafe { arena.realloc(first, large, small.size()) };
    assert_eq!(shrunk, first);
    let grown = unsafe { arena.realloc(shrunk, small, large.size()) };
    assert_eq!(grown, first);
    let shrunk = unsafe { arena.realloc(grown, large, small.size()) };
    let blocker = unsafe { arena.alloc(large) };
    assert!(!blocker.is_null());
    unsafe {
        core::ptr::write_bytes(blocker, 0x59, large.size());
    }
    let moved = unsafe { arena.realloc(shrunk, small, large.size()) };
    assert!(!moved.is_null());
    assert_ne!(moved, shrunk);
    assert_eq!(moved as usize % 64, 0);
    assert!(unsafe { core::slice::from_raw_parts(moved, small.size()) }.iter().all(|byte| *byte == 0x37));
    assert!(unsafe { core::slice::from_raw_parts(blocker, large.size()) }.iter().all(|byte| *byte == 0x59));
    let failed = unsafe { arena.realloc(moved, large, BLOCK_BYTES * BLOCK_COUNT + 1) };
    assert!(failed.is_null());
    assert!(unsafe { core::slice::from_raw_parts(moved, small.size()) }.iter().all(|byte| *byte == 0x37));
    unsafe {
        arena.dealloc(moved, large);
        arena.dealloc(blocker, large);
    }
}
