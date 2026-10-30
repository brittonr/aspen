pub fn encode_node_observer_trust(
    trust: &crate::node_service_host::authority::wire::Trust,
) -> crate::error::Result<Vec<u8>> {
    wrap("linux-observation-trust-v1", crate::node_service_host::authority::wire::encode_trust(trust)?)
}

pub fn decode_node_observer_trust(
    bytes: &[u8],
) -> crate::error::Result<crate::node_service_host::authority::wire::Trust> {
    crate::node_service_host::authority::wire::decode_trust(&unwrap(bytes, "linux-observation-trust-v1", 262144)?)
}

pub fn encode_node_observer_credential(
    credential: &crate::node_service_host::authority::wire::Credential,
) -> crate::error::Result<Vec<u8>> {
    wrap(
        "linux-observation-credential-v1",
        crate::node_service_host::authority::wire::encode_credential(credential)?,
    )
}

pub fn decode_node_observer_credential(
    bytes: &[u8],
) -> crate::error::Result<crate::node_service_host::authority::wire::Credential> {
    crate::node_service_host::authority::wire::decode_credential(&unwrap(
        bytes,
        "linux-observation-credential-v1",
        2228224,
    )?)
}

fn wrap(tag: &'static str, bytes: Vec<u8>) -> crate::error::Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(tag, vec![preserves::IOValue::bytes(bytes)]))
}

fn unwrap(bytes: &[u8], tag: &str, bound: usize) -> crate::error::Result<Vec<u8>> {
    if bytes.len() > bound {
        return Err(super::denied());
    }
    let value = crate::preserves_rail::strict_canonical_decode(bytes)?.value;
    let fields = crate::preserves_rail::simple_record_fields(&value, tag, 1)?;
    Ok(fields[0].as_bytestring().ok_or_else(super::denied)?.as_ref().to_vec())
}
