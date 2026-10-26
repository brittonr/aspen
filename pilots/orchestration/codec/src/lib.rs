// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
//! Canonical semantic identity is Preserves + BLAKE3. JSON is only a bounded
//! reconstruction projection and adapter frame, verified against that identity.
use molten_batch_core::Id;
use molten_batch_core::MAX_FRAME_BYTES;
use molten_batch_core::MAX_STATE_BYTES;
use preserves::IOValue;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Codec(String),
    Bound,
    Identity,
    NonCanonical,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
fn error(e: impl std::fmt::Display) -> Error {
    Error::Codec(e.to_string())
}
pub struct Encoded {
    pub canonical: Vec<u8>,
    pub projection: Vec<u8>,
    pub id: Id,
}
pub fn digest(bytes: &[u8]) -> Id {
    Id(*blake3::hash(bytes).as_bytes())
}
fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    IOValue::record(IOValue::symbol(label), fields)
}
fn string(s: &str) -> IOValue {
    IOValue::new(s.to_owned())
}
fn sequence(v: Vec<IOValue>) -> IOValue {
    IOValue::new(v)
}
fn project(v: &Value, depth: usize, budget: &mut usize) -> Result<IOValue, Error> {
    if depth > 48 || *budget == 0 {
        return Err(Error::Bound);
    }
    *budget -= 1;
    Ok(match v {
        Value::Null => record("o.null", vec![]),
        Value::Bool(true) => record("o.true", vec![]),
        Value::Bool(false) => record("o.false", vec![]),
        Value::Number(n) => {
            if n.as_i64().is_none() && n.as_u64().is_none() {
                return Err(Error::Codec("floats are not in this schema".into()));
            }
            record("o.integer", vec![string(&n.to_string())])
        }
        Value::String(s) => record("o.string", vec![string(s)]),
        Value::Array(a) => record("o.sequence", vec![sequence(
            a.iter().map(|x| project(x, depth + 1, budget)).collect::<Result<Vec<_>, _>>()?,
        )]),
        Value::Object(m) => {
            let mut fields = m.iter().collect::<Vec<_>>();
            fields.sort_by(|a, b| a.0.cmp(b.0));
            record("o.map", vec![sequence(
                fields
                    .into_iter()
                    .map(|(k, v)| Ok(record("o.field", vec![string(k), project(v, depth + 1, budget)?])))
                    .collect::<Result<Vec<_>, Error>>()?,
            )])
        }
    })
}
pub fn encode<T: Serialize>(schema: &str, t: &T) -> Result<Encoded, Error> {
    if schema.is_empty() || schema.len() > 128 {
        return Err(Error::Bound);
    }
    let projection = serde_json::to_vec(t).map_err(error)?;
    if projection.len() > MAX_STATE_BYTES {
        return Err(Error::Bound);
    }
    let json = serde_json::from_slice(&projection).map_err(error)?;
    let mut budget = 1_000_000;
    let value = record("molten.orchestration.document.v1", vec![string(schema), project(&json, 0, &mut budget)?]);
    let canonical = preserves::write_iovalue_packed(&value, false).map_err(error)?;
    if canonical.len() > MAX_STATE_BYTES {
        return Err(Error::Bound);
    }
    let id = digest(&canonical);
    Ok(Encoded {
        canonical,
        projection,
        id,
    })
}
pub fn decode<T: Serialize + DeserializeOwned>(schema: &str, canonical: &[u8], projection: &[u8]) -> Result<T, Error> {
    if canonical.len() > MAX_STATE_BYTES || projection.len() > MAX_STATE_BYTES {
        return Err(Error::Bound);
    }
    // JSON's default recursive parse limit remains enabled. Exact re-encoding
    // rejects duplicate fields, alternate spellings, unknown fields and trailing
    // bytes. Packed bytes are compared, not parsed with an unbounded decoder.
    let parsed: T = serde_json::from_slice(projection).map_err(error)?;
    let expected = encode(schema, &parsed)?;
    if expected.canonical != canonical || expected.projection != projection {
        return Err(Error::NonCanonical);
    }
    Ok(parsed)
}
pub fn identity<T: Serialize>(schema: &str, t: &T) -> Result<Id, Error> {
    Ok(encode(schema, t)?.id)
}
#[derive(Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet<T> {
    schema: String,
    value_ref: Id,
    body: T,
}
pub fn frame<T: Serialize>(schema: &str, t: &T) -> Result<Vec<u8>, Error> {
    let bytes = serde_json::to_vec(&Packet {
        schema: schema.to_owned(),
        value_ref: identity(schema, t)?,
        body: t,
    })
    .map_err(error)?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(Error::Bound);
    }
    Ok(bytes)
}
pub fn unframe<T: Serialize + DeserializeOwned>(schema: &str, bytes: &[u8]) -> Result<T, Error> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(Error::Bound);
    }
    let p: Packet<T> = serde_json::from_slice(bytes).map_err(error)?;
    if p.schema != schema || identity(schema, &p.body)? != p.value_ref {
        return Err(Error::Identity);
    }
    if serde_json::to_vec(&p).map_err(error)? != bytes {
        return Err(Error::NonCanonical);
    }
    Ok(p.body)
}
