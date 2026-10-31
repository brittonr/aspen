pub(super) trait Wire: Sized {
    fn encode(&self) -> crate::error::Result<preserves::IOValue>;
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self>;
}

pub(super) fn error(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

macro_rules! wire_struct {
    ($ty:ty, $label:literal, $($field:ident),+ $(,)?) => {
        impl $crate::node_service_host::codec::wire::Wire for $ty {
            fn encode(&self) -> crate::error::Result<preserves::IOValue> {
                Ok(crate::preserves_rail::record($label, vec![
                    $({
                        let value = $crate::node_service_host::codec::wire::Wire::encode(&self.$field)?;
                        $crate::node_service_host::codec::wire::field(&value, stringify!($field))?;
                        value
                    }),+
                ]))
            }
            fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
                let count = [$(stringify!($field)),+].len();
                let fields = value.collect_simple_record($label, Some(count))
                    .ok_or_else(|| $crate::node_service_host::codec::wire::error(concat!("expected ", $label)))?;
                let mut fields = fields.fields_iter();
                Ok(Self { $($field: {
                    let value = fields.next().ok_or_else(||
                        $crate::node_service_host::codec::wire::error("missing record field"))?;
                    let value = crate::preserves_rail::value_to_iovalue(value);
                    $crate::node_service_host::codec::wire::field(&value, stringify!($field))?;
                    $crate::node_service_host::codec::wire::Wire::decode(&value)?
                }),+ })
            }
        }
    };
}

macro_rules! wire_enum {
    ($ty:ty, $($variant:ident),+ $(,)?) => {
        impl $crate::node_service_host::codec::wire::Wire for $ty {
            fn encode(&self) -> crate::error::Result<preserves::IOValue> {
                Ok(crate::preserves_rail::string(self.as_str()))
            }
            fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
                let name = value.as_string().ok_or_else(||
                    $crate::node_service_host::codec::wire::error("expected enum string"))?;
                $(if name == Self::$variant.as_str() { return Ok(Self::$variant); })+
                Err($crate::node_service_host::codec::wire::error("unknown enum variant"))
            }
        }
    };
}

impl Wire for String {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        if self.len() > 16_384 {
            return Err(error("bundle text bound exceeded"));
        }
        Ok(crate::preserves_rail::string(self))
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        let text = value.as_string().ok_or_else(|| error("expected string"))?;
        if text.len() > 16_384 {
            return Err(error("bundle text bound exceeded"));
        }
        Ok(text.to_string())
    }
}

impl Wire for bool {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        Ok(preserves::IOValue::new(*self))
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        if *value == preserves::IOValue::new(true) {
            return Ok(true);
        }
        if *value == preserves::IOValue::new(false) {
            return Ok(false);
        }
        Err(error("expected boolean"))
    }
}

impl Wire for u64 {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        Ok(preserves::IOValue::new(*self))
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        value
            .as_u64()
            .ok_or_else(|| error("expected unsigned integer"))?
            .map_err(|_| error("unsigned integer overflow"))
    }
}

impl Wire for usize {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        u64::try_from(*self).map_err(|_| error("integer overflow"))?.encode()
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        Self::try_from(u64::decode(value)?).map_err(|_| error("integer overflow"))
    }
}

impl Wire for i32 {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        Ok(preserves::IOValue::new(*self))
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        let number = value
            .as_i64()
            .ok_or_else(|| error("expected signed integer"))?
            .map_err(|_| error("signed integer overflow"))?;
        Self::try_from(number).map_err(|_| error("exit code overflow"))
    }
}

impl Wire for u8 {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        u64::from(*self).encode()
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        Self::try_from(u64::decode(value)?).map_err(|_| error("byte overflow"))
    }
}

impl<T: Wire> Wire for Vec<T> {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        if self.len() > 256 {
            return Err(error("bundle collection bound exceeded"));
        }
        Ok(crate::preserves_rail::sequence(self.iter().map(Wire::encode).collect::<crate::error::Result<_>>()?))
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        let items = value.collect_sequence().ok_or_else(|| error("expected sequence"))?;
        if items.len() > 256 {
            return Err(error("bundle collection bound exceeded"));
        }
        items.iter().map(|v| T::decode(&crate::preserves_rail::value_to_iovalue(v))).collect()
    }
}

impl<T: Wire> Wire for Option<T> {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        match self {
            None => Ok(crate::preserves_rail::record("none", vec![])),
            Some(value) => Ok(crate::preserves_rail::record("some", vec![value.encode()?])),
        }
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        if value.collect_simple_record("none", Some(0)).is_some() {
            return Ok(None);
        }
        let fields = value.collect_simple_record("some", Some(1)).ok_or_else(|| error("expected optional value"))?;
        Ok(Some(T::decode(&crate::preserves_rail::value_to_iovalue(&fields[0]))?))
    }
}

impl Wire for std::path::PathBuf {
    fn encode(&self) -> crate::error::Result<preserves::IOValue> {
        self.to_str().ok_or_else(|| error("non UTF-8 artifact path"))?.to_string().encode()
    }
    fn decode(value: &preserves::IOValue) -> crate::error::Result<Self> {
        Ok(Self::from(String::decode(value)?))
    }
}

pub(super) fn field(value: &preserves::IOValue, name: &str) -> crate::error::Result<()> {
    // `max_*` fields are numeric bounds on reference collections, not references themselves.
    if (name.ends_with("_ref") || name.ends_with("_refs")) && !name.starts_with("max_") {
        reference_value(value)?;
    }
    Ok(())
}

/// Matches the bundle collection bound: a reference field never holds more pending nested values.
const MAX_PENDING_REFERENCES: usize = 256;

fn reference_value(value: &preserves::IOValue) -> crate::error::Result<()> {
    let mut pending = Vec::with_capacity(MAX_PENDING_REFERENCES);
    pending.push(value.clone());
    while let Some(value) = pending.pop() {
        if let Some(text) = value.as_string() {
            crate::preserves_rail::ContentRef::parse(text)?;
            continue;
        }
        if value.collect_simple_record("none", Some(0)).is_some() {
            continue;
        }
        let nested = match value.collect_simple_record("some", Some(1)) {
            Some(fields) => vec![crate::preserves_rail::value_to_iovalue(&fields[0])],
            None => value
                .collect_sequence()
                .ok_or_else(|| error("expected content reference field"))?
                .iter()
                .map(crate::preserves_rail::value_to_iovalue)
                .collect(),
        };
        if nested.len() > MAX_PENDING_REFERENCES - pending.len() {
            return Err(error("bundle collection bound exceeded"));
        }
        pending.extend(nested);
    }
    Ok(())
}
