//! Transaction types for atomic multi-key operations with conditions.

use serde::Deserialize;
use serde::Serialize;

use crate::KeyValueWithRevision;

/// Comparison target for transaction conditions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareTarget {
    Value,
    Version,
    CreateRevision,
    ModRevision,
}

/// Comparison operator for transaction conditions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Greater,
    Less,
}

/// A comparison condition for transactions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TxnCompare {
    pub key: String,
    pub target: CompareTarget,
    pub op: CompareOp,
    pub value: String,
}

/// Operations that can be performed in a transaction branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxnOp {
    Put { key: String, value: String },
    Delete { key: String },
    Get { key: String },
    Range { prefix: String, limit: u32 },
}

/// Result of a single transaction operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxnOpResult {
    Put { revision: u64 },
    Delete { deleted: u32 },
    Get { kv: Option<KeyValueWithRevision> },
    Range { kvs: Vec<KeyValueWithRevision>, more: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_target_clone_and_eq() {
        assert_eq!(CompareTarget::Value, CompareTarget::Value.clone());
        assert_eq!(CompareTarget::Version, CompareTarget::Version.clone());
        assert_eq!(CompareTarget::CreateRevision, CompareTarget::CreateRevision.clone());
        assert_eq!(CompareTarget::ModRevision, CompareTarget::ModRevision.clone());
        assert_ne!(CompareTarget::Value, CompareTarget::Version);
    }

    #[test]
    fn compare_op_clone_and_eq() {
        assert_eq!(CompareOp::Equal, CompareOp::Equal.clone());
        assert_eq!(CompareOp::NotEqual, CompareOp::NotEqual.clone());
        assert_eq!(CompareOp::Greater, CompareOp::Greater.clone());
        assert_eq!(CompareOp::Less, CompareOp::Less.clone());
        assert_ne!(CompareOp::Equal, CompareOp::NotEqual);
    }

    #[test]
    fn txn_compare_clone_and_eq() {
        let cmp = TxnCompare {
            key: "k".into(),
            target: CompareTarget::Value,
            op: CompareOp::Equal,
            value: "v".into(),
        };
        assert_eq!(cmp, cmp.clone());
    }

    #[test]
    fn txn_op_clone_and_eq() {
        let put = TxnOp::Put {
            key: "k".into(),
            value: "v".into(),
        };
        let delete = TxnOp::Delete { key: "k".into() };
        let get = TxnOp::Get { key: "k".into() };
        let range = TxnOp::Range {
            prefix: "p".into(),
            limit: 10,
        };

        assert_eq!(put, put.clone());
        assert_eq!(delete, delete.clone());
        assert_eq!(get, get.clone());
        assert_eq!(range, range.clone());
        assert_ne!(put, delete);
    }

    #[test]
    fn txn_op_result_clone_and_eq() {
        let put_result = TxnOpResult::Put { revision: 42 };
        let delete_result = TxnOpResult::Delete { deleted: 1 };
        let get_result = TxnOpResult::Get { kv: None };
        let range_result = TxnOpResult::Range {
            kvs: vec![],
            more: false,
        };

        assert_eq!(put_result, put_result.clone());
        assert_eq!(delete_result, delete_result.clone());
        assert_eq!(get_result, get_result.clone());
        assert_eq!(range_result, range_result.clone());
    }

    #[test]
    fn compare_target_debug() {
        assert_eq!(format!("{:?}", CompareTarget::Value), "Value");
        assert_eq!(format!("{:?}", CompareTarget::Version), "Version");
        assert_eq!(format!("{:?}", CompareTarget::CreateRevision), "CreateRevision");
        assert_eq!(format!("{:?}", CompareTarget::ModRevision), "ModRevision");
    }

    #[test]
    fn compare_op_debug() {
        assert_eq!(format!("{:?}", CompareOp::Equal), "Equal");
        assert_eq!(format!("{:?}", CompareOp::NotEqual), "NotEqual");
        assert_eq!(format!("{:?}", CompareOp::Greater), "Greater");
        assert_eq!(format!("{:?}", CompareOp::Less), "Less");
    }

    #[test]
    fn txn_compare_serialization_roundtrip() {
        let cmp = TxnCompare {
            key: "k".into(),
            target: CompareTarget::Version,
            op: CompareOp::Greater,
            value: "5".into(),
        };
        let json = serde_json::to_string(&cmp).unwrap();
        let deserialized: TxnCompare = serde_json::from_str(&json).unwrap();
        assert_eq!(cmp, deserialized);
    }
}
