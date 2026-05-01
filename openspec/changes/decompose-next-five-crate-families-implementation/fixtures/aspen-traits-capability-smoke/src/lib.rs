use aspen_traits::{KeyValueStoreError, KvRead, ReadRequest, ReadResult};

struct ReadOnlyStore;

#[async_trait::async_trait]
impl KvRead for ReadOnlyStore {
    async fn read(&self, _request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        Err(KeyValueStoreError::NotFound { key: "fixture".to_string() })
    }
}

fn accepts_read<T: KvRead>(_store: &T) {}

pub fn prove_narrow_read_capability() {
    let store = ReadOnlyStore;
    accepts_read(&store);
}
