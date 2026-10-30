//! Strict OTel-WASI value admission and payload-minimized diagnostic projection.
use wasmtime_rpc::component;

pub fn context(receipt: &str) -> component::Val {
    let digest = blake3::hash(receipt.as_bytes()).to_hex().to_string();
    component::Val::Record(vec![
        ("trace-id".into(), component::Val::String(digest[..32].into())),
        ("span-id".into(), component::Val::String(digest[32..48].into())),
        ("trace-flags".into(), component::Val::Flags(vec!["sampled".into()])),
        ("is-remote".into(), component::Val::Bool(false)),
        ("trace-state".into(), component::Val::List(Vec::new())),
    ])
}
fn field<'a>(value: &'a component::Val, name: &str) -> super::Result<&'a component::Val> {
    let component::Val::Record(fields) = value else {
        return Err("OTel value is not a record".into());
    };
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .ok_or_else(|| format!("OTel field {name} absent"))
}
fn string(value: &component::Val, expected: &str) -> super::Result<()> {
    match value {
        component::Val::String(value) if value == expected => Ok(()),
        _ => Err("OTel string field is not approved".into()),
    }
}
fn empty(value: &component::Val) -> super::Result<()> {
    match value {
        component::Val::List(value) if value.is_empty() => Ok(()),
        _ => Err("OTel payload/array field is not empty".into()),
    }
}
fn attributes(value: &component::Val, receipt: &str) -> super::Result<()> {
    let component::Val::List(values) = value else {
        return Err("OTel attributes are not a list".into());
    };
    if values.len() > 1 {
        return Err("OTel attributes exceed reviewed allowlist".into());
    }
    for value in values {
        string(field(value, "key")?, "molten.receipt")?;
        let component::Val::String(value) = field(value, "value")? else {
            return Err("OTel attribute value must be JSON text".into());
        };
        if value.len() > super::super::MAX_BYTES {
            return Err("OTel attribute exceeds byte bound".into());
        }
        let value: serde_json::Value = serde_json::from_str(value).map_err(super::super::failure)?;
        if value.as_str() != Some(receipt) {
            return Err("OTel receipt link differs from admitted canonical receipt".into());
        }
    }
    Ok(())
}

// r[impl molten.component_rpc.telemetry]
pub fn span(value: &component::Val, receipt: &str) -> super::Result<preserves::IOValue> {
    let component::Val::Record(fields) = value else {
        return Err("span-data must be a record".into());
    };
    if fields.len() != 14 || field(value, "span-context")? != &context(receipt) {
        return Err("span shape or trusted host context differs".into());
    }
    string(field(value, "name")?, "component-rpc-call")?;
    string(field(value, "parent-span-id")?, "")?;
    match field(value, "span-kind")? {
        component::Val::Enum(kind) if kind == "client" => {}
        _ => return Err("unapproved span kind".into()),
    }
    attributes(field(value, "attributes")?, receipt)?;
    empty(field(value, "events")?)?;
    empty(field(value, "links")?)?;
    let instrumentation = field(value, "instrumentation-scope")?;
    string(field(instrumentation, "name")?, "molten-component-rpc")?;
    empty(field(instrumentation, "attributes")?)?;
    for name in ["version", "schema-url"] {
        if !matches!(field(instrumentation, name)?, component::Val::Option(None)) {
            return Err("unapproved instrumentation metadata".into());
        }
    }
    if !matches!(field(value, "status")?, component::Val::Variant(name, None) if name == "unset") {
        return Err("OTel status payload or application acceptance claim denied".into());
    }
    for name in ["dropped-attributes", "dropped-events", "dropped-links"] {
        if !matches!(field(value, name)?, component::Val::U32(0)) {
            return Err("OTel producer dropped data".into());
        }
    }
    Ok(crate::codec::record("component-rpc-otel-span-diagnostic-v1", vec![
        crate::codec::string("diagnostic-only"),
        crate::codec::string(receipt),
        crate::codec::string("component-rpc-call"),
        crate::codec::string("ended"),
        crate::codec::string("not-application-acceptance-not-canonical-evidence"),
    ]))
}
