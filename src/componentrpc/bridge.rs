//! Exact hello unary mapping. WIT bytes and Preserves identities stay distinct.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub caller: String,
    pub callee: String,
    pub wit: String,
    pub bundle: String,
    pub policy: String,
    pub resources: String,
    pub peer: String,
    pub caller_peer: String,
    pub session: String,
    pub delivery: String,
    pub dedup: String,
    pub generation: u64,
    pub deadline_tick: u64,
}

impl Scope {
    pub fn value(&self) -> preserves::IOValue {
        let mut fields = vec![
            crate::codec::string(super::PROFILE),
            crate::codec::string(super::REVISION),
        ];
        fields.extend(self.references().into_iter().map(crate::codec::string));
        fields.push(crate::codec::string(self.generation.to_string()));
        fields.push(crate::codec::string(self.deadline_tick.to_string()));
        crate::codec::record("component-rpc-scope-v1", fields)
    }

    pub fn references(&self) -> [&str; 11] {
        [
            &self.caller,
            &self.callee,
            &self.wit,
            &self.bundle,
            &self.policy,
            &self.resources,
            &self.peer,
            &self.caller_peer,
            &self.session,
            &self.delivery,
            &self.dedup,
        ]
    }

    pub fn validate(&self, tick: u64) -> super::Result<()> {
        for reference in self.references() {
            crate::codec::validate_content_ref(reference).map_err(super::failure)?;
        }
        if self.generation == 0 || tick >= self.deadline_tick {
            return Err("stale invocation generation or deadline".into());
        }
        if self.resources != super::profile::resources()? || self.delivery != super::profile::delivery()? {
            return Err("resource or delivery contract differs from closed unary profile".into());
        }
        Ok(())
    }
}

// r[impl molten.component_rpc.bridge]
// r[impl molten.component_rpc.functional_core]
pub fn request(scope: &Scope, wire: &[u8]) -> super::Result<preserves::IOValue> {
    if !wire.is_empty() {
        return Err("hello has no arguments; nonempty wire is malformed".into());
    }
    Ok(crate::codec::record("component-rpc-request-v1", vec![
        scope.value(),
        crate::codec::string(super::INTERFACE),
        crate::codec::string(super::FUNCTION),
        crate::codec::string(super::reference(wire)),
    ]))
}

pub fn invocation(scope: &Scope) -> super::Result<String> {
    super::hash(&request(scope, &[])?)
}

/// Strict canonical unsigned LEB32 length followed by UTF-8, with no trailing bytes.
pub fn decode_result(wire: &[u8]) -> super::Result<String> {
    if wire.len() > super::MAX_BYTES {
        return Err("result exceeds byte budget".into());
    }
    let mut length = 0_u32;
    for index in 0..5 {
        let byte = *wire.get(index).ok_or("truncated string length")?;
        if index == 4 && byte > 15 {
            return Err("string length overflows u32".into());
        }
        length |= u32::from(byte & 127) << (index * 7);
        if byte & 128 == 0 {
            if index > 0 && byte == 0 {
                return Err("nonminimal string length".into());
            }
            let payload = &wire[index + 1..];
            if usize::try_from(length).ok() != Some(payload.len()) {
                return Err("string length or trailing bytes mismatch".into());
            }
            return std::str::from_utf8(payload).map(str::to_owned).map_err(super::failure);
        }
    }
    Err("invalid string length".into())
}

pub fn result(invocation: &str, wire: &[u8]) -> super::Result<preserves::IOValue> {
    crate::codec::validate_content_ref(invocation).map_err(super::failure)?;
    Ok(crate::codec::record("component-rpc-result-v1", vec![
        crate::codec::string(invocation),
        crate::codec::string(decode_result(wire)?),
        crate::codec::string(super::reference(wire)),
    ]))
}

// r[impl molten.component_rpc.async_scope]
pub fn admit_shape(interface: &str, function: &str, paths: u32) -> super::Result<()> {
    if interface != super::INTERFACE || function != super::FUNCTION || paths != 0 {
        return Err("only pinned hello unary is supported; streams/futures/extensions denied".into());
    }
    Ok(())
}

pub fn encode_result(value: &str) -> super::Result<Vec<u8>> {
    if value.len() > super::MAX_BYTES - 5 {
        return Err("result exceeds byte budget".into());
    }
    let mut length = value.len() as u32;
    let mut wire = Vec::with_capacity(value.len() + 5);
    loop {
        let byte = u8::try_from(length & 127).map_err(super::failure)?;
        length >>= 7;
        wire.push(if length == 0 { byte } else { byte | 128 });
        if length == 0 {
            break;
        }
    }
    wire.extend_from_slice(value.as_bytes());
    Ok(wire)
}

pub fn decode_envelope(value: &preserves::IOValue, invocation: &str) -> super::Result<Vec<u8>> {
    let fields = crate::codec::simple_record_fields(value, "component-rpc-result-v1", 3).map_err(super::failure)?;
    let text = crate::codec::required_string_field(&fields[1], "result").map_err(super::failure)?;
    let wire = encode_result(&text)?;
    if result(invocation, &wire)? != *value {
        return Err("cross-call or malformed result envelope".into());
    }
    Ok(wire)
}
