use core::alloc::GlobalAlloc;

const BLOCK_BYTES: usize = 16;
const BLOCK_COUNT: usize = 16_384;
const BITMAP_WORDS: usize = BLOCK_COUNT / 64;

#[repr(align(16))]
struct Heap([u8; BLOCK_BYTES * BLOCK_COUNT]);
static mut HEAP: Heap = Heap([0; BLOCK_BYTES * BLOCK_COUNT]);
static mut USED: [u64; BITMAP_WORDS] = [0; BITMAP_WORDS];

// Single-threaded bounded fixture arena. Freeing canonical input/response and
// post-return buffers makes repeated actor invocations reclaim their storage.
struct Arena;

unsafe impl GlobalAlloc for Arena {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let blocks = layout.size().max(1).div_ceil(BLOCK_BYTES);
        if blocks > BLOCK_COUNT { return core::ptr::null_mut(); }
        let base = unsafe { core::ptr::addr_of_mut!(HEAP.0).cast::<u8>() };
        let mut start = 0;
        let mut run = 0;
        let mut index = 0;
        while index < BLOCK_COUNT {
            let word = unsafe { (*core::ptr::addr_of!(USED))[index / 64] } >> (index % 64);
            let remaining = 64 - index % 64;
            if word & 1 != 0 {
                run = 0;
                index += usize::try_from(word.trailing_ones()).expect("bitmap run fits usize").min(remaining);
                continue;
            }
            if run == 0 {
                if (base as usize + index * BLOCK_BYTES) & layout.align().wrapping_sub(1) != 0 {
                    index += 1;
                    continue;
                }
                start = index;
            }
            let available = usize::try_from(word.trailing_zeros()).expect("bitmap run fits usize").min(remaining);
            run += available;
            index += available;
            if run >= blocks {
                unsafe { set_range(start, blocks, true); }
                return unsafe { base.add(start * BLOCK_BYTES) };
            }
        }
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: core::alloc::Layout) {
        let base = unsafe { core::ptr::addr_of_mut!(HEAP.0).cast::<u8>() };
        let first = (pointer as usize - base as usize) / BLOCK_BYTES;
        let blocks = layout.size().max(1).div_ceil(BLOCK_BYTES);
        unsafe { set_range(first, blocks, false); }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: core::alloc::Layout, new_size: usize) -> *mut u8 {
        let old_blocks = layout.size().max(1).div_ceil(BLOCK_BYTES);
        let new_blocks = new_size.max(1).div_ceil(BLOCK_BYTES);
        let base = unsafe { core::ptr::addr_of_mut!(HEAP.0).cast::<u8>() };
        let first = (pointer as usize - base as usize) / BLOCK_BYTES;
        if new_blocks <= old_blocks {
            unsafe { set_range(first + new_blocks, old_blocks - new_blocks, false); }
            return pointer;
        }
        if new_blocks <= BLOCK_COUNT - first
            && unsafe { is_range_free(first + old_blocks, new_blocks - old_blocks) }
        {
            unsafe { set_range(first + old_blocks, new_blocks - old_blocks, true); }
            return pointer;
        }
        let Ok(new_layout) = core::alloc::Layout::from_size_align(new_size, layout.align()) else {
            return core::ptr::null_mut();
        };
        let replacement = unsafe { self.alloc(new_layout) };
        if !replacement.is_null() {
            unsafe {
                core::ptr::copy_nonoverlapping(pointer, replacement, layout.size().min(new_size));
                self.dealloc(pointer, layout);
            }
        }
        replacement
    }
}

unsafe fn set_range(mut first: usize, mut blocks: usize, is_used: bool) {
    while blocks != 0 {
        let offset = first % 64;
        let count = blocks.min(64 - offset);
        let mask = (u64::MAX >> (64 - count)) << offset;
        let word = unsafe { &mut (*core::ptr::addr_of_mut!(USED))[first / 64] };
        if is_used {
            *word |= mask;
        } else {
            *word &= !mask;
        }
        first += count;
        blocks -= count;
    }
}

unsafe fn is_range_free(mut first: usize, mut blocks: usize) -> bool {
    while blocks != 0 {
        let offset = first % 64;
        let count = blocks.min(64 - offset);
        let mask = (u64::MAX >> (64 - count)) << offset;
        if unsafe { (*core::ptr::addr_of!(USED))[first / 64] } & mask != 0 {
            return false;
        }
        first += count;
        blocks -= count;
    }
    true
}

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: Arena = Arena;

// The freestanding guest has no libc import. Volatile loads keep LLVM from
// lowering this implementation back into a call to the same builtin.
// SAFETY: callers supply two readable regions of at least `length` bytes.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
unsafe extern "C" fn memcmp(left: *const u8, right: *const u8, length: usize) -> core::ffi::c_int {
    for index in 0..length {
        let left_byte = unsafe { left.add(index).read_volatile() };
        let right_byte = unsafe { right.add(index).read_volatile() };
        if left_byte != right_byte {
            return core::ffi::c_int::from(left_byte) - core::ffi::c_int::from(right_byte);
        }
    }
    0
}
