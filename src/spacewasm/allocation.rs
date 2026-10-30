use std::alloc;
use std::cell;
use std::marker;
use std::ptr;
use std::rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub limit: usize,
    pub allocated: usize,
    pub peak: usize,
    pub allocations: usize,
    pub denied: usize,
    pub ir_limit: usize,
    pub ir_allocated: usize,
    pub ir_peak: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetError {
    NestedSession,
    OutstandingAllocations,
    LimitTooLarge,
    ReservationOutstanding,
    AllocationFailed,
    OutOfMemory,
}

#[derive(Clone, Copy, Default)]
struct State {
    active: bool,
    usage: Usage,
    reserved: Option<(ptr::NonNull<u8>, alloc::Layout)>,
}

std::thread_local! {
    static STATE: cell::Cell<State> = cell::Cell::new(State::default());
}

/// Keep this guard alive until every NASA object and stream has been dropped.
/// It is thread-bound, just like the allocator context it controls.
pub struct Budget {
    thread: marker::PhantomData<rc::Rc<()>>,
}

impl Budget {
    pub fn enter(limit: usize) -> Result<Self, BudgetError> {
        STATE.with(|state| {
            let previous = state.get();
            if previous.active {
                return Err(BudgetError::NestedSession);
            }
            if previous.usage.allocated != 0 {
                return Err(BudgetError::OutstandingAllocations);
            }
            if limit > i32::MAX as usize {
                return Err(BudgetError::LimitTooLarge);
            }
            state.set(State {
                active: true,
                usage: Usage {
                    limit,
                    ir_limit: limit,
                    ..Usage::default()
                },
                reserved: None,
            });
            Ok(Self {
                thread: marker::PhantomData,
            })
        })
    }

    pub fn usage(&self) -> Usage {
        STATE.with(|state| state.get().usage)
    }

    pub fn limit_ir(&self, bytes: usize) -> Result<(), BudgetError> {
        STATE.with(|state| {
            let mut current = state.get();
            if bytes < current.usage.ir_allocated {
                return Err(BudgetError::OutOfMemory);
            }
            current.usage.ir_limit = bytes.min(current.usage.limit);
            state.set(current);
            Ok(())
        })
    }
}

impl Drop for Budget {
    fn drop(&mut self) {
        STATE.with(|state| {
            let mut current = state.get();
            if let Some((pointer, layout)) = current.reserved.take() {
                unsafe { alloc::dealloc(pointer.as_ptr(), layout) };
                current.usage.allocated -= layout.size();
                if is_ir(layout) {
                    current.usage.ir_allocated -= layout.size();
                }
            }
            current.active = false;
            state.set(current);
        });
    }
}

/// Reserve the exact closure allocation before NASA's infallible host constructor.
/// A zero-sized closure needs no allocation. Only one reservation may be pending.
pub fn reserve<T>(value: &T) -> Result<(), BudgetError> {
    let layout = alloc::Layout::for_value(value);
    if layout.size() == 0 {
        return Ok(());
    }
    if STATE.with(|state| state.get().reserved.is_some()) {
        return Err(BudgetError::ReservationOutstanding);
    }
    let pointer = allocate(layout).map_err(|error| match error {
        ::spacewasm::AllocError::OutOfMemory => BudgetError::OutOfMemory,
        _ => BudgetError::AllocationFailed,
    })?;
    STATE.with(|state| {
        let mut current = state.get();
        current.reserved = Some((pointer, layout));
        state.set(current);
    });
    Ok(())
}

// Conservatively charge every allocation with the exact upstream TextPage layout.
fn is_ir(layout: alloc::Layout) -> bool {
    layout == alloc::Layout::new::<::spacewasm::TextPage>()
}

fn allocate(layout: alloc::Layout) -> Result<ptr::NonNull<u8>, ::spacewasm::AllocError> {
    STATE.with(|state| {
        let mut current = state.get();
        if current.active
            && let Some((pointer, reserved)) = current.reserved
            && reserved == layout
        {
            current.reserved = None;
            state.set(current);
            return Ok(pointer);
        }
        if !current.active
            || layout.size() > current.usage.limit - current.usage.allocated
            || (is_ir(layout) && layout.size() > current.usage.ir_limit - current.usage.ir_allocated)
        {
            current.usage.denied = current.usage.denied.saturating_add(1);
            state.set(current);
            return Err(::spacewasm::AllocError::OutOfMemory);
        }
        let pointer =
            ptr::NonNull::new(unsafe { alloc::alloc(layout) }).ok_or(::spacewasm::AllocError::AllocationFailed)?;
        current.usage.allocated += layout.size();
        if is_ir(layout) {
            current.usage.ir_allocated += layout.size();
        }
        current.usage.ir_peak = current.usage.ir_peak.max(current.usage.ir_allocated);
        current.usage.peak = current.usage.peak.max(current.usage.allocated);
        current.usage.allocations = current.usage.allocations.saturating_add(1);
        state.set(current);
        Ok(pointer)
    })
}

/// # Safety
/// `err`, when nonnull, must be writable for one u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __spacewasm_alloc(size: usize, align: usize, err: *mut u32) -> *mut u8 {
    let result = alloc::Layout::from_size_align(size, align)
        .ok()
        .filter(|layout| layout.size() != 0)
        .ok_or(::spacewasm::AllocError::AllocationFailed)
        .and_then(allocate);
    match result {
        Ok(pointer) => pointer.as_ptr(),
        Err(error) => {
            if !err.is_null() {
                unsafe { err.write(error.into()) };
            }
            ptr::null_mut()
        }
    }
}

/// # Safety
/// The pointer and layout must match a live allocation on this thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __spacewasm_dealloc(pointer: *mut u8, size: usize, align: usize) {
    if pointer.is_null() || size == 0 {
        return;
    }
    if let Ok(layout) = alloc::Layout::from_size_align(size, align) {
        unsafe { alloc::dealloc(pointer, layout) };
        STATE.with(|state| {
            let mut current = state.get();
            current.usage.allocated -= size;
            if is_ir(layout) {
                current.usage.ir_allocated -= size;
            }
            state.set(current);
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn __spacewasm_memory_statistics() -> ::spacewasm::MemoryStatistics {
    STATE.with(|state| ::spacewasm::MemoryStatistics {
        total_bytes: state.get().usage.allocated as i32,
        pad_bytes: 0,
    })
}

/// Linear memory has its own cap and is not charged to the metadata budget.
pub struct GuestAllocator {
    max_bytes: usize,
    allocated: cell::Cell<usize>,
}

impl GuestAllocator {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            allocated: cell::Cell::new(0),
        }
    }

    pub fn usage(&self) -> usize {
        self.allocated.get()
    }
}

impl ::spacewasm::WasmMemoryAllocator for GuestAllocator {
    fn allocate(&self, layout: alloc::Layout) -> Result<ptr::NonNull<u8>, ::spacewasm::AllocError> {
        if layout.size() > self.max_bytes - self.allocated.get() {
            return Err(::spacewasm::AllocError::OutOfMemory);
        }
        if layout.size() == 0 {
            return ptr::NonNull::new(layout.align() as *mut u8).ok_or(::spacewasm::AllocError::AllocationFailed);
        }
        let pointer =
            ptr::NonNull::new(unsafe { alloc::alloc(layout) }).ok_or(::spacewasm::AllocError::AllocationFailed)?;
        self.allocated.set(self.allocated.get() + layout.size());
        Ok(pointer)
    }

    fn reallocate(
        &self,
        _pointer: ptr::NonNull<u8>,
        _old_layout: alloc::Layout,
        _layout: alloc::Layout,
    ) -> Result<ptr::NonNull<u8>, ::spacewasm::AllocError> {
        Err(::spacewasm::AllocError::OutOfMemory)
    }

    fn deallocate(&self, pointer: ptr::NonNull<u8>, layout: alloc::Layout) {
        if layout.size() != 0 {
            unsafe { alloc::dealloc(pointer.as_ptr(), layout) };
            self.allocated.set(self.allocated.get() - layout.size());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn metadata_cap_and_outstanding_allocations_fence_contexts() {
        let budget = super::Budget::enter(8).unwrap();
        assert!(matches!(super::Budget::enter(8), Err(super::BudgetError::NestedSession)));
        let mut error = 0;
        let pointer = unsafe { super::__spacewasm_alloc(8, 8, &mut error) };
        assert!(!pointer.is_null());
        assert!(unsafe { super::__spacewasm_alloc(1, 1, &mut error) }.is_null());
        assert_eq!(error, 1);
        assert_eq!(budget.usage().allocated, 8);
        assert_eq!(budget.usage().peak, 8);
        drop(budget);
        assert!(matches!(super::Budget::enter(8), Err(super::BudgetError::OutstandingAllocations)));
        unsafe { super::__spacewasm_dealloc(pointer, 8, 8) };
        let fresh = super::Budget::enter(4).unwrap();
        assert_eq!(fresh.usage().allocated, 0);
        assert_eq!(fresh.usage().peak, 0);
    }

    #[test]
    fn guest_cap_is_separate_and_growth_preserves_existing_bytes() {
        let guest = super::GuestAllocator::new(8);
        let layout = std::alloc::Layout::from_size_align(8, 8).unwrap();
        let pointer = ::spacewasm::WasmMemoryAllocator::allocate(&guest, layout).unwrap();
        unsafe { pointer.as_ptr().write(37) };
        assert!(::spacewasm::WasmMemoryAllocator::allocate(&guest, layout).is_err());
        assert!(
            ::spacewasm::WasmMemoryAllocator::reallocate(
                &guest,
                pointer,
                layout,
                std::alloc::Layout::from_size_align(16, 8).unwrap(),
            )
            .is_err()
        );
        assert_eq!(unsafe { pointer.as_ptr().read() }, 37);
        ::spacewasm::WasmMemoryAllocator::deallocate(&guest, pointer, layout);
        assert_eq!(guest.usage(), 0);
    }

    #[test]
    fn reserved_box_succeeds_at_full_budget_and_unused_reservation_is_freed() {
        let budget = super::Budget::enter(512).unwrap();
        let value = [42_u16; 256];
        budget.limit_ir(511).unwrap();
        assert!(matches!(super::reserve(&value), Err(super::BudgetError::OutOfMemory)));
        budget.limit_ir(512).unwrap();
        super::reserve(&value).unwrap();
        assert_eq!(budget.usage().ir_allocated, 512);
        assert!(matches!(super::reserve(&value), Err(super::BudgetError::ReservationOutstanding)));
        let boxed = ::spacewasm::Box::new(value).unwrap();
        assert_eq!(*boxed, value);
        assert_eq!(budget.usage().allocations, 1);
        drop(boxed);
        assert_eq!(budget.usage().ir_allocated, 0);
        super::reserve(&value).unwrap();
        drop(budget);
        let fresh = super::Budget::enter(0).unwrap();
        assert_eq!(fresh.usage().allocated, 0);
    }
}
