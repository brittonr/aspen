use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeValuePortFailureKind {
    Missing,
    IdentityMismatch,
    BoundExceeded,
    RejectedBeforeAcceptance,
    UnknownAfterAcceptance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeValuePortFailure {
    pub kind: NativeValuePortFailureKind,
    pub message: String,
}

impl NativeValuePortFailure {
    pub fn new(kind: NativeValuePortFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub const fn may_have_published(&self) -> bool {
        matches!(self.kind, NativeValuePortFailureKind::UnknownAfterAcceptance)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeValuePublicationReceipt {
    pub value_ref: String,
    pub publication_ref: String,
    pub byte_count: u64,
}

pub trait NativeCallbackValuePort: Send {
    fn materialize(
        &mut self,
        value_ref: &str,
        maximum_bytes: u64,
    ) -> Result<NativeCallbackValue, NativeValuePortFailure>;

    fn publish(
        &mut self,
        value: &NativeCallbackValue,
        maximum_bytes: u64,
    ) -> Result<NativeValuePublicationReceipt, NativeValuePortFailure>;
}

pub type SharedNativeCallbackValuePort = Arc<Mutex<Box<dyn NativeCallbackValuePort>>>;

pub fn shared_native_callback_value_port(
    port: impl NativeCallbackValuePort + 'static,
) -> SharedNativeCallbackValuePort {
    Arc::new(Mutex::new(Box::new(port)))
}

#[derive(Debug, Default)]
pub struct InMemoryNativeCallbackValuePort {
    values: BTreeMap<String, Vec<u8>>,
    next_publication_failure: Option<NativeValuePortFailureKind>,
}

impl InMemoryNativeCallbackValuePort {
    pub fn from_values(values: impl IntoIterator<Item = Vec<u8>>) -> Self {
        let mut port = Self::default();
        for bytes in values {
            let value_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
            port.values.insert(value_ref, bytes);
        }
        port
    }

    pub fn insert_exact(&mut self, value: NativeCallbackValue) -> Result<(), NativeValuePortFailure> {
        admit_native_callback_value(&value, u64::MAX)?;
        if let Some(existing) = self.values.get(&value.value_ref)
            && existing != &value.bytes
        {
            return Err(NativeValuePortFailure::new(
                NativeValuePortFailureKind::IdentityMismatch,
                "native value reference already names different bytes",
            ));
        }
        self.values.insert(value.value_ref, value.bytes);
        Ok(())
    }

    pub fn fail_next_publication(&mut self, kind: NativeValuePortFailureKind) {
        self.next_publication_failure = Some(kind);
    }

    pub fn contains(&self, value_ref: &str) -> bool {
        self.values.contains_key(value_ref)
    }
}

impl NativeCallbackValuePort for InMemoryNativeCallbackValuePort {
    fn materialize(
        &mut self,
        value_ref: &str,
        maximum_bytes: u64,
    ) -> Result<NativeCallbackValue, NativeValuePortFailure> {
        let bytes = self.values.get(value_ref).cloned().ok_or_else(|| {
            NativeValuePortFailure::new(NativeValuePortFailureKind::Missing, "native callback value is not available")
        })?;
        let value = NativeCallbackValue {
            value_ref: value_ref.to_string(),
            bytes,
        };
        admit_native_callback_value(&value, maximum_bytes)?;
        Ok(value)
    }

    fn publish(
        &mut self,
        value: &NativeCallbackValue,
        maximum_bytes: u64,
    ) -> Result<NativeValuePublicationReceipt, NativeValuePortFailure> {
        admit_native_callback_value(value, maximum_bytes)?;
        if let Some(kind) = self.next_publication_failure.take() {
            if kind == NativeValuePortFailureKind::UnknownAfterAcceptance {
                self.values.insert(value.value_ref.clone(), value.bytes.clone());
            }
            return Err(NativeValuePortFailure::new(kind, "injected native value publication failure"));
        }
        if let Some(existing) = self.values.get(&value.value_ref)
            && existing != &value.bytes
        {
            return Err(NativeValuePortFailure::new(
                NativeValuePortFailureKind::IdentityMismatch,
                "native value reference already names different bytes",
            ));
        }
        self.values.insert(value.value_ref.clone(), value.bytes.clone());
        let byte_count = u64::try_from(value.bytes.len()).map_err(|_| {
            NativeValuePortFailure::new(
                NativeValuePortFailureKind::BoundExceeded,
                "native value byte count does not fit u64",
            )
        })?;
        Ok(NativeValuePublicationReceipt {
            value_ref: value.value_ref.clone(),
            publication_ref: native_identity_ref(&[
                "native-value-publication-v2",
                &value.value_ref,
                &byte_count.to_string(),
            ]),
            byte_count,
        })
    }
}

// r[impl molten.system_extension.native_host.value_materialization]
// r[impl molten.system_extension.native_host.value_publication]
pub fn admit_native_callback_value(
    value: &NativeCallbackValue,
    maximum_bytes: u64,
) -> Result<(), NativeValuePortFailure> {
    let byte_count = u64::try_from(value.bytes.len()).map_err(|_| {
        NativeValuePortFailure::new(
            NativeValuePortFailureKind::BoundExceeded,
            "native callback value byte count does not fit u64",
        )
    })?;
    if byte_count > maximum_bytes {
        return Err(NativeValuePortFailure::new(
            NativeValuePortFailureKind::BoundExceeded,
            format!("native callback value exceeds {maximum_bytes} bytes"),
        ));
    }
    let observed_ref = crate::preserves_rail::content_ref_from_bytes(&value.bytes);
    if observed_ref != value.value_ref {
        return Err(NativeValuePortFailure::new(
            NativeValuePortFailureKind::IdentityMismatch,
            "native callback value bytes do not match their reference",
        ));
    }
    Ok(())
}
