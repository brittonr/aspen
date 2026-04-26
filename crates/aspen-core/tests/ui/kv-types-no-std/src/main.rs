use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadConsistency;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;

fn main() {
    let _req = ReadRequest::new("test-key");
    let _err = KeyValueStoreError::EmptyKey;
    let _consistency = ReadConsistency::default();
    let _cmd = WriteCommand::Set {
        key: "k".into(),
        value: "v".into(),
    };
}
