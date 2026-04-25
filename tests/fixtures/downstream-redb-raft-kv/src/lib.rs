//! Downstream consumer fixture proving canonical APIs work without `aspen` package.

use aspen_kv_types::WriteCommand;
use aspen_raft_kv_types::{RaftKvTypeConfig, RaftKvRequest, RaftKvResponse, BatchWriteOp};
use aspen_redb_storage::raft_storage::RedbKvStorage;
use aspen_redb_storage::verified::{compute_kv_versions, check_cas_condition};
use aspen_raft_kv::KeyValueStore;
use aspen_raft_kv::ClusterController;
use aspen_raft_kv::RaftKvNodeBuilder;
use aspen_raft_kv::RaftKvStorageConfig;
use aspen_raft_kv::RaftKvMembershipConfig;

pub fn smoke_test_types() {
    let _req = RaftKvRequest::Set {
        key: "k".into(),
        value: "v".into(),
    };
    let _batch = BatchWriteOp::Put {
        key: "k".into(),
        value: "v".into(),
    };
    let versions = compute_kv_versions(None, 1);
    assert_eq!(versions.version, 1);
    assert!(check_cas_condition(None, None));
}

pub fn smoke_test_config() {
    let storage = RaftKvStorageConfig::new("/tmp/test.redb".into());
    let membership = RaftKvMembershipConfig::new(1, Default::default());
    let _builder = RaftKvNodeBuilder::new(storage, membership);
}
