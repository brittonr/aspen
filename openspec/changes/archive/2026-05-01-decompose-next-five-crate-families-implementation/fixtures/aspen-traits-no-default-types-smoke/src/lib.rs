use aspen_traits::{ReadRequest, WriteRequest};

pub fn accepts_portable_types(read: ReadRequest, write: WriteRequest) -> (ReadRequest, WriteRequest) {
    (read, write)
}
