use aspen_traits::ReadRequest;
use aspen_traits::WriteRequest;

pub fn accepts_portable_types(read: ReadRequest, write: WriteRequest) -> (ReadRequest, WriteRequest) {
    (read, write)
}
