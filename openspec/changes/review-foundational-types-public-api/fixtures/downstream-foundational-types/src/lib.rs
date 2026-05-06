use aspen_cluster_types::ClusterNode;
use aspen_cluster_types::NodeAddress;
use aspen_cluster_types::NodeId;
use aspen_cluster_types::NodeTransportAddr;
use aspen_constants::api::MAX_KEY_SIZE;
use aspen_constants::api::MAX_VALUE_SIZE;
use aspen_hlc::SerializableTimestamp;
use aspen_hlc::create_hlc;
use aspen_hlc::new_timestamp;
use aspen_storage_types::KvEntry;
use aspen_time::TimeProvider;
use aspen_traits::ReadRequest;
use aspen_traits::WriteRequest;

struct FixtureClock;

impl TimeProvider for FixtureClock {
    fn now_unix_ms(&self) -> u64 {
        42
    }

    fn now_unix_secs(&self) -> u64 {
        0
    }
}

pub fn exercise_foundational_types() -> usize {
    let entry = KvEntry {
        value: "value".to_string(),
        version: 1,
        create_revision: 1,
        mod_revision: 1,
        expires_at_ms: Some(42),
        lease_id: None,
    };

    let node_id = NodeId::new(7);
    let address =
        NodeAddress::from_parts("node-key", [NodeTransportAddr::Relay("https://relay.example.invalid".to_string())]);
    let cluster_node = ClusterNode::new(node_id.0, address.endpoint_id().to_string(), None);

    let hlc = create_hlc("fixture-node");
    let timestamp = SerializableTimestamp::new(new_timestamp(&hlc));
    let clock = FixtureClock;

    let read = ReadRequest::new("key");
    let _write = WriteRequest::set("key", "value");

    entry.value.len()
        + cluster_node.addr.len()
        + timestamp.id().len()
        + clock.now_unix_ms() as usize
        + read.key.len()
        + MAX_KEY_SIZE as usize
        + MAX_VALUE_SIZE as usize
}

#[cfg(test)]
mod tests {
    use super::exercise_foundational_types;

    #[test]
    fn fixture_uses_only_foundational_public_apis() {
        assert!(exercise_foundational_types() > 0);
    }
}
