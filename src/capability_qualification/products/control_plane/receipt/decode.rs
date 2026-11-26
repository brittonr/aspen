//! Strict field readers of the canonical replica-process receipts.

use preserves::ValueImpl as _;

const MAX_RECEIPT_MEMBERS: usize = 8;

pub(super) fn fields(
    value: &preserves::IOValue,
    label: &str,
    arity: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    fields_of(value, label, arity)
}

pub(super) fn fields_of(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    arity: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let fields = value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| crate::capability_qualification::invalid("malformed live Raft replica receipt"))?;
    Ok(fields.iter().collect())
}

pub(super) fn text(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::capability_qualification::invalid("live Raft replica receipt expected a string"))
}

pub(super) fn number(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::capability_qualification::invalid("live Raft replica receipt expected a number"))?
        .map_err(|_| crate::capability_qualification::invalid("live Raft replica receipt number is out of range"))
}

pub(super) fn flag(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| crate::capability_qualification::invalid("live Raft replica receipt expected a boolean"))
}

pub(super) fn texts(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    let members = value
        .collect_sequence()
        .ok_or_else(|| crate::capability_qualification::invalid("live Raft replica receipt expected a sequence"))?;
    if members.len() > MAX_RECEIPT_MEMBERS {
        return Err(crate::capability_qualification::invalid("live Raft replica receipt sequence exceeds its bound"));
    }
    members.iter().map(|member| text(&member)).collect()
}
