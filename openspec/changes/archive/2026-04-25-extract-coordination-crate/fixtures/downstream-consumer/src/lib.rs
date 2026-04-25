use aspen_coordination::{DistributedLock, LeaderElection, QueueManager};
use aspen_kv_types::ReadRequest;
use aspen_traits::KeyValueStore;

type LockHandle = DistributedLock<dyn KeyValueStore>;
type ElectionHandle = LeaderElection<dyn KeyValueStore>;
type QueueHandle = QueueManager<dyn KeyValueStore>;

pub fn prove_coordination_types_are_available() {
    assert_send_sync::<LockHandle>();
    assert_send_sync::<ElectionHandle>();
    assert_send_sync::<QueueHandle>();

    let _request = ReadRequest::new(String::from("downstream-proof"));
}

fn assert_send_sync<T: Send + Sync>() {}
