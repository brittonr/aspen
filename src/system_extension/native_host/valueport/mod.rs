//! Capability-relative byte storage for the existing callback value port.
//! Publication is process-visible, not a durability or transaction receipt.

use super::NativeCallbackValuePort;

pub struct CapabilityCallbackValuePort {
    namespace: crate::node_state::Scope,
}

impl CapabilityCallbackValuePort {
    pub fn new(namespace: crate::node_state::Scope) -> crate::error::Result<Self> {
        if namespace.kind() != crate::node_state::NamespaceKind::Ledger {
            return Err(crate::error::Failure::invalid_harness("callback values require a ledger namespace"));
        }
        namespace.create_dir_all(&crate::node_state::RelativePath::parse("values")?)?;
        Ok(Self { namespace })
    }
}

impl NativeCallbackValuePort for CapabilityCallbackValuePort {
    fn materialize(
        &mut self,
        value_ref: &str,
        maximum_bytes: u64,
    ) -> std::result::Result<crate::system_extension::NativeCallbackValue, super::ValuePortFailure> {
        let path = value_path(value_ref)?;
        let bytes = self.namespace.read(&path, maximum_bytes).map_err(read_failure)?;
        let value = crate::system_extension::NativeCallbackValue {
            value_ref: value_ref.to_string(),
            bytes,
        };
        super::admit_callback_value(&value, maximum_bytes)?;
        Ok(value)
    }

    fn publish(
        &mut self,
        value: &crate::system_extension::NativeCallbackValue,
        maximum_bytes: u64,
    ) -> std::result::Result<super::ValuePublicationReceipt, super::ValuePortFailure> {
        super::admit_callback_value(value, maximum_bytes)?;
        let path = value_path(&value.value_ref)?;
        match self.namespace.observe_file(&path).map_err(read_failure)? {
            crate::node_state::FileObservation::Regular(file) => {
                let existing = file.read_bounded(maximum_bytes).map_err(read_failure)?;
                if existing != value.bytes {
                    return Err(super::ValuePortFailure::new(
                        super::ValuePortFailureKind::IdentityMismatch,
                        "callback value path contains different bytes",
                    ));
                }
            }
            crate::node_state::FileObservation::Missing => {
                self.namespace.write(&path, &value.bytes).map_err(|_| {
                    super::ValuePortFailure::new(
                        super::ValuePortFailureKind::UnknownAfterAcceptance,
                        "callback value publication failed",
                    )
                })?;
            }
            crate::node_state::FileObservation::NonRegular(_) => {
                return Err(super::ValuePortFailure::new(
                    super::ValuePortFailureKind::RejectedBeforeAcceptance,
                    "callback value leaf is not regular",
                ));
            }
        }
        let byte_count = u64::try_from(value.bytes.len()).map_err(|_| {
            super::ValuePortFailure::new(
                super::ValuePortFailureKind::BoundExceeded,
                "callback value length is not representable",
            )
        })?;
        let receipt = crate::preserves_rail::record("callback-value-publication-v1", vec![
            crate::preserves_rail::string(&value.value_ref),
            crate::preserves_rail::u64_value(byte_count),
            crate::preserves_rail::string("process-visible-not-durable"),
        ]);
        Ok(super::ValuePublicationReceipt {
            value_ref: value.value_ref.clone(),
            publication_ref: crate::preserves_rail::canonical_hash(&receipt).map_err(read_failure)?,
            byte_count,
        })
    }
}

fn value_path(value_ref: &str) -> std::result::Result<crate::node_state::RelativePath, super::ValuePortFailure> {
    crate::preserves_rail::validate_content_ref(value_ref).map_err(read_failure)?;
    let digest = value_ref.strip_prefix("blake3:").ok_or_else(|| {
        super::ValuePortFailure::new(
            super::ValuePortFailureKind::IdentityMismatch,
            "callback value reference is not BLAKE3",
        )
    })?;
    crate::node_state::RelativePath::parse(&format!("values/{digest}.preserves")).map_err(read_failure)
}

fn read_failure(_error: crate::error::Failure) -> super::ValuePortFailure {
    super::ValuePortFailure::new(
        super::ValuePortFailureKind::RejectedBeforeAcceptance,
        "capability callback value access failed",
    )
}
