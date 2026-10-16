use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValuePortFailureKind {
    Missing,
    IdentityMismatch,
    BoundExceeded,
    RejectedBeforeAcceptance,
    UnknownAfterAcceptance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePortFailure {
    pub kind: ValuePortFailureKind,
    pub message: String,
}

impl ValuePortFailure {
    pub fn new(kind: ValuePortFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub const fn may_have_published(&self) -> bool {
        matches!(self.kind, ValuePortFailureKind::UnknownAfterAcceptance)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePublicationReceipt {
    pub value_ref: String,
    pub publication_ref: String,
    pub byte_count: u64,
}

pub trait NativeCallbackValuePort: Send {
    fn materialize(&mut self, value_ref: &str, maximum_bytes: u64) -> Result<NativeCallbackValue, ValuePortFailure>;

    fn publish(
        &mut self,
        value: &NativeCallbackValue,
        maximum_bytes: u64,
    ) -> Result<ValuePublicationReceipt, ValuePortFailure>;
}

pub type SharedCallbackValuePort = std::sync::Arc<std::sync::Mutex<Box<dyn NativeCallbackValuePort>>>;

pub fn shared_callback_value_port(port: impl NativeCallbackValuePort + 'static) -> SharedCallbackValuePort {
    std::sync::Arc::new(std::sync::Mutex::new(Box::new(port)))
}

#[derive(Debug)]
pub struct InMemoryCallbackValuePort {
    values: std::collections::BTreeMap<String, Vec<u8>>,
    next_publication_failure: Option<ValuePortFailureKind>,
}

impl Default for InMemoryCallbackValuePort {
    fn default() -> Self {
        Self::empty()
    }
}

impl InMemoryCallbackValuePort {
    /// A port with no stored values and no scheduled publication failure.
    pub const fn empty() -> Self {
        Self {
            values: std::collections::BTreeMap::new(),
            next_publication_failure: None,
        }
    }

    pub fn from_values(values: impl IntoIterator<Item = Vec<u8>>) -> Self {
        let mut port = Self::default();
        for bytes in values {
            let value_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
            port.values.insert(value_ref, bytes);
        }
        port
    }

    pub fn insert_exact(&mut self, value: NativeCallbackValue) -> Result<(), ValuePortFailure> {
        admit_callback_value(&value, u64::MAX)?;
        if let Some(existing) = self.values.get(&value.value_ref)
            && existing != &value.bytes
        {
            return Err(ValuePortFailure::new(
                ValuePortFailureKind::IdentityMismatch,
                "native value reference already names different bytes",
            ));
        }
        self.values.insert(value.value_ref, value.bytes);
        Ok(())
    }

    pub fn fail_next_publication(&mut self, kind: ValuePortFailureKind) {
        self.next_publication_failure = Some(kind);
    }

    pub fn contains(&self, value_ref: &str) -> bool {
        self.values.contains_key(value_ref)
    }
}

impl NativeCallbackValuePort for InMemoryCallbackValuePort {
    fn materialize(&mut self, value_ref: &str, maximum_bytes: u64) -> Result<NativeCallbackValue, ValuePortFailure> {
        let bytes = self.values.get(value_ref).cloned().ok_or_else(|| {
            ValuePortFailure::new(ValuePortFailureKind::Missing, "native callback value is not available")
        })?;
        let value = NativeCallbackValue {
            value_ref: value_ref.to_string(),
            bytes,
        };
        admit_callback_value(&value, maximum_bytes)?;
        Ok(value)
    }

    fn publish(
        &mut self,
        value: &NativeCallbackValue,
        maximum_bytes: u64,
    ) -> Result<ValuePublicationReceipt, ValuePortFailure> {
        admit_callback_value(value, maximum_bytes)?;
        if let Some(kind) = self.next_publication_failure.take() {
            if kind == ValuePortFailureKind::UnknownAfterAcceptance {
                self.values.insert(value.value_ref.clone(), value.bytes.clone());
            }
            return Err(ValuePortFailure::new(kind, "injected native value publication failure"));
        }
        if let Some(existing) = self.values.get(&value.value_ref)
            && existing != &value.bytes
        {
            return Err(ValuePortFailure::new(
                ValuePortFailureKind::IdentityMismatch,
                "native value reference already names different bytes",
            ));
        }
        self.values.insert(value.value_ref.clone(), value.bytes.clone());
        let byte_count = u64::try_from(value.bytes.len()).map_err(|_| {
            ValuePortFailure::new(ValuePortFailureKind::BoundExceeded, "native value byte count does not fit u64")
        })?;
        Ok(ValuePublicationReceipt {
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
pub fn admit_callback_value(value: &NativeCallbackValue, maximum_bytes: u64) -> Result<(), ValuePortFailure> {
    let byte_count = u64::try_from(value.bytes.len()).map_err(|_| {
        ValuePortFailure::new(ValuePortFailureKind::BoundExceeded, "native callback value byte count does not fit u64")
    })?;
    if byte_count > maximum_bytes {
        return Err(ValuePortFailure::new(
            ValuePortFailureKind::BoundExceeded,
            format!("native callback value exceeds {maximum_bytes} bytes"),
        ));
    }
    let observed_ref = crate::preserves_rail::content_ref_from_bytes(&value.bytes);
    if observed_ref != value.value_ref {
        return Err(ValuePortFailure::new(
            ValuePortFailureKind::IdentityMismatch,
            "native callback value bytes do not match their reference",
        ));
    }
    Ok(())
}
