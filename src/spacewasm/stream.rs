use std::alloc;
use std::ptr;

#[derive(Debug, PartialEq, Eq)]
pub enum StreamError {
    InvalidChunkSize,
    AllocationFailed,
}

/// Borrows the artifact and loans one reusable, budget-charged chunk to NASA.
/// `InnerVec` has no destructor: `return_` returns the loan, not its allocation.
pub struct BorrowedStream<'a> {
    input: &'a [u8],
    position: usize,
    pointer: ptr::NonNull<u8>,
    capacity: u32,
    loaned: bool,
    poisoned: bool,
}

impl<'a> BorrowedStream<'a> {
    pub fn new(input: &'a [u8], chunk_size: usize) -> Result<Self, StreamError> {
        if chunk_size == 0 || chunk_size > u32::MAX as usize {
            return Err(StreamError::InvalidChunkSize);
        }
        let capacity = chunk_size.min(input.len()) as u32;
        let pointer = if capacity == 0 {
            ptr::NonNull::dangling()
        } else {
            let layout = alloc::Layout::array::<u8>(capacity as usize).map_err(|_| StreamError::InvalidChunkSize)?;
            let raw = unsafe { ::spacewasm::Allocator::alloc(&::spacewasm::GlobalAllocator, layout) }
                .map_err(|_| StreamError::AllocationFailed)?;
            ptr::NonNull::new(raw).ok_or(StreamError::AllocationFailed)?
        };
        Ok(Self {
            input,
            position: 0,
            pointer,
            capacity,
            loaned: false,
            poisoned: false,
        })
    }
}

impl ::spacewasm::WasmStream for BorrowedStream<'_> {
    fn read(&mut self) -> Result<Option<::spacewasm::InnerVec<u8>>, u8> {
        if self.loaned || self.poisoned {
            return Err(1);
        }
        let count = (self.input.len() - self.position).min(self.capacity as usize);
        if count == 0 {
            return Ok(None);
        }
        unsafe {
            ptr::copy_nonoverlapping(self.input.as_ptr().add(self.position), self.pointer.as_ptr(), count);
        }
        self.position += count;
        self.loaned = true;
        Ok(Some(::spacewasm::InnerVec {
            ptr: self.pointer.as_ptr(),
            capacity: self.capacity,
            len: count as u32,
        }))
    }

    fn return_(&mut self, chunk: ::spacewasm::InnerVec<u8>) {
        // Reader also returns an initial/terminal empty chunk.
        if chunk.capacity == 0 && chunk.len == 0 {
            return;
        }
        if !self.loaned
            || chunk.ptr != self.pointer.as_ptr()
            || chunk.capacity != self.capacity
            || chunk.len > chunk.capacity
        {
            self.poisoned = true;
            return;
        }
        self.loaned = false;
    }
}

impl Drop for BorrowedStream<'_> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            // The allocation is always owned here, even while NASA has its loan.
            let layout = unsafe { alloc::Layout::from_size_align_unchecked(self.capacity as usize, 1) };
            unsafe {
                ::spacewasm::Allocator::dealloc(&::spacewasm::GlobalAllocator, self.pointer.as_ptr(), layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exact_input_reuses_chunk_without_allocating_during_reads() {
        let budget = super::super::allocation::Budget::enter(3).unwrap();
        let mut stream = super::BorrowedStream::new(b"abcdefgh", 3).unwrap();
        let mut previous: *mut u8 = std::ptr::null_mut();
        for expected in [b"abc".as_slice(), b"def".as_slice(), b"gh".as_slice()] {
            let chunk = ::spacewasm::WasmStream::read(&mut stream).unwrap().unwrap();
            assert_eq!(&*chunk, expected);
            if !previous.is_null() {
                assert_eq!(chunk.ptr, previous);
            }
            previous = chunk.ptr;
            assert!(::spacewasm::WasmStream::read(&mut stream).is_err());
            ::spacewasm::WasmStream::return_(&mut stream, chunk);
        }
        assert!(::spacewasm::WasmStream::read(&mut stream).unwrap().is_none());
        assert_eq!(budget.usage().allocations, 1);
        drop(stream);
        assert_eq!(budget.usage().allocated, 0);
    }

    #[test]
    fn chunk_allocation_is_fallible_and_empty_input_needs_none() {
        let budget = super::super::allocation::Budget::enter(0).unwrap();
        assert!(super::BorrowedStream::new(b"a", 1).is_err());
        let mut stream = super::BorrowedStream::new(b"", 1).unwrap();
        assert!(::spacewasm::WasmStream::read(&mut stream).unwrap().is_none());
        assert_eq!(budget.usage().allocated, 0);
        assert!(super::BorrowedStream::new(b"", 0).is_err());
    }
}
