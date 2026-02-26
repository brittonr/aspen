//! Batch operation types for atomic multi-key writes.

use serde::Deserialize;
use serde::Serialize;

/// A single operation within a batch write.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchOperation {
    Set { key: String, value: String },
    Delete { key: String },
}

/// A condition for conditional batch writes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchCondition {
    ValueEquals { key: String, expected: String },
    KeyExists { key: String },
    KeyNotExists { key: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_operation_clone_and_eq() {
        let op = BatchOperation::Set {
            key: "k".into(),
            value: "v".into(),
        };
        assert_eq!(op, op.clone());

        let op2 = BatchOperation::Delete { key: "k".into() };
        assert_eq!(op2, op2.clone());
        assert_ne!(op, op2);
    }

    #[test]
    fn batch_condition_clone_and_eq() {
        let c1 = BatchCondition::ValueEquals {
            key: "k".into(),
            expected: "v".into(),
        };
        let c2 = BatchCondition::KeyExists { key: "k".into() };
        let c3 = BatchCondition::KeyNotExists { key: "k".into() };

        assert_eq!(c1, c1.clone());
        assert_eq!(c2, c2.clone());
        assert_eq!(c3, c3.clone());
        assert_ne!(c1, c2);
        assert_ne!(c2, c3);
    }
}
