//! Byte-bound resource certificates for the selected revision-3 I64 net.
//!
//! A certificate is checked against the exact net encoding and exact rule-table bytes supplied
//! to the checker. The checker decodes both artifacts, independently validates the decoded net
//! against the admitted graph and input row, and derives its interaction and initial-agent
//! claims from decoded symbols. It does not call the certificate producer or net lowerer.
//!
//! The interaction claim is an abstract rewrite count. The live-agent claim is committed graph
//! storage measured as one cell per live agent. Neither claim includes wire arena reservations,
//! allocate-before-free replacement storage, worklists, fragmentation, source data, decoding,
//! readback, cleanup, or process RSS. The digest's collision resistance is supplied by the
//! caller's [`crate::identity::IdentityScheme`]; the decoder and checker do not prove properties
//! of that hash implementation.

use crate::identity::IdentityScheme;

/// Wire format revision for the bounded resource certificate.
pub const CERTIFICATE_REVISION: &str = "ngraph-net-resource/1";
/// Independent net decoder/checker revision.
pub const CHECKER_REVISION: &str = "ngraph-net-resource-checker/0";
/// Kernel model revision whose local cost obligations support this certificate profile.
pub const PROOF_MODEL_REVISION: &str = "NobleFoundation.NetI64Resource/0";
/// Revision of the fixed cost and committed-agent model.
pub const COST_MODEL_REVISION: &str = "ngraph-net-resource-cost/0";

/// Exact selected revision-3 rule table. A checker compares the received bytes with this
/// complete table before accepting its digest, so stale, omitted, or extended rules refuse.
pub const RULE_TABLE_BYTES: &[u8] = b"ngraph-net-i64/3\n\
op2-num: op2(k) + num(a) -> op1(k,a); cost=1\n\
op1-num: op1(k,a) + num(b) -> num(apply64(k,a,b)); cost=1\n\
dup-num: dup + num(v) -> num(v) + num(v); cost=1\n\
era-num: era + num(v) -> empty; cost=1\n\
apply64: add/sub/mul wrap modulo 2^64; eq is 1 iff equal else 0\n\
reverse-orientation: inets mirror\n";

/// Fixed resource functions, admitted-net premise, and explicit exclusions bound into each
/// certificate. They are not producer-supplied annotations.
pub const COST_MODEL_BYTES: &[u8] = b"ngraph-net-resource-cost/0\n\
premise: exact decoded fully-connected admitted ngraph-net-i64/3 net\n\
time-potential: num=0; op2=2; op1=1; dup=1; era=1\n\
interaction-charge: one per selected rule firing; each arithmetic node uses two stages\n\
space-potential: one committed cell per live agent; initial bound is decoded agent count\n\
boundary: ordered output endpoints are byte-bound, but are not agent cells\n\
excluded: wires, input materialization time, replacement reservation, worklists, fragmentation, source data, decode, readback, cleanup, process RSS\n";

const CERTIFICATE_MAGIC: &[u8; 8] = b"NGRSC1\0\0";
const NET_MAGIC: &[u8; 8] = b"NGRNET3\0";
const DIGEST_BYTES: usize = 32;
const MAX_CERTIFICATE_BYTES: usize = 512;
const MAX_NET_BYTES: usize = 64 * 1024 * 1024;
const MAX_NET_ITEMS: usize = 1_048_576;
const MAX_HASH_SCHEME_BYTES: usize = 32;

/// Why a byte-bound resource certificate was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Certificate or net encoding was malformed, truncated, oversized, or non-canonical.
    Malformed,
    /// Certificate format revision is not recognized.
    StaleCertificateRevision,
    /// The certificate names a different byte-checker revision.
    StaleCheckerRevision,
    /// The certificate names a different kernel resource-model revision.
    StaleProofRevision,
    /// The certificate names a different cost-model revision.
    StaleCostModelRevision,
    /// The certificate names a different identity-hash scheme.
    StaleHashScheme,
    /// The raw net bytes differ from the digest bound by the certificate.
    NetDigestMismatch,
    /// The raw rule-table bytes differ from the digest bound by the certificate.
    RuleDigestMismatch,
    /// Rule bytes are not exactly the selected revision-3 rule table.
    StaleRuleTable,
    /// The certificate's cost-model digest differs from the checker's fixed profile.
    WrongCostModel,
    /// The decoded net is not the admitted checked graph/input translation.
    InvalidNet,
    /// Resource arithmetic or a bounded length conversion overflowed.
    ArithmeticOverflow,
    /// The declared interaction count is not the independently derived count.
    WrongInteractionCount,
    /// The declared live-agent bound is not the decoded initial population.
    WrongLiveAgentBound,
}

/// Producer output. The rule bytes are the public [`RULE_TABLE_BYTES`] constant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    /// Exact canonical bytes for the fully connected lowered net.
    pub net_bytes: alloc::vec::Vec<u8>,
    /// Encoded certificate whose digests bind `net_bytes` and [`RULE_TABLE_BYTES`].
    pub certificate_bytes: alloc::vec::Vec<u8>,
}

/// Borrowed exact bytes supplied to the independent checker.
#[derive(Clone, Copy)]
pub struct CheckBytes<'a> {
    /// Producer-declared binary certificate, parsed before any claim is trusted.
    pub certificate: &'a [u8],
    /// Canonically encoded connected net, decoded and revalidated against the source graph.
    pub net: &'a [u8],
    /// Complete selected rule table, compared byte-for-byte against the checker's fixed table.
    pub rules: &'a [u8],
}

/// Independently recomputed claims accepted by [`check`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedResources {
    /// Exact abstract interaction count for the initial net.
    pub interactions: u64,
    /// Initial and maximum committed live-agent population.
    pub live_agent_bound: u64,
    /// Initial undirected wire plus output-interface count, when representable.
    pub initial_wires: u64,
}

#[derive(Clone, Copy)]
struct Claims {
    net_digest: crate::ir::Digest,
    rule_digest: crate::ir::Digest,
    cost_digest: crate::ir::Digest,
    interactions: u64,
    live_agent_bound: u64,
}

#[cfg(test)]
const RESOURCE_CLAIMS_OFFSET: usize = CERTIFICATE_MAGIC.len()
    + 2
    + CERTIFICATE_REVISION.len()
    + 2
    + CHECKER_REVISION.len()
    + 2
    + PROOF_MODEL_REVISION.len()
    + 2
    + COST_MODEL_REVISION.len()
    + DIGEST_BYTES * 3;
#[cfg(test)]
const RESOURCE_INTERACTIONS_OFFSET: usize = RESOURCE_CLAIMS_OFFSET;
#[cfg(test)]
const RESOURCE_LIVE_BOUND_OFFSET: usize = RESOURCE_CLAIMS_OFFSET + 8;
#[cfg(test)]
const RESOURCE_COST_DIGEST_OFFSET: usize = CERTIFICATE_MAGIC.len()
    + 2
    + CERTIFICATE_REVISION.len()
    + 2
    + CHECKER_REVISION.len()
    + 2
    + PROOF_MODEL_REVISION.len()
    + 2
    + COST_MODEL_REVISION.len()
    + DIGEST_BYTES * 2;

/// Creates the producer-side artifact for an already lowered net.
///
/// This function only emits claims. Consumers MUST call [`check`] on the returned bytes and
/// the exact rule bytes rather than trusting this producer.
pub fn issue(
    ids: &dyn IdentityScheme,
    checked: &crate::check::CheckedGraph,
    net: &crate::net::InteractionGraph,
    inputs: &[crate::sem::Value],
) -> Result<Artifact, Refusal> {
    crate::net::validate(ids, checked, net, inputs, &crate::net::Limits::default())
        .map_err(|_| Refusal::InvalidNet)?;
    let net_bytes = encode_net(net)?;
    let interactions = net
        .expected_interactions()
        .ok_or(Refusal::ArithmeticOverflow)?;
    let live_agent_bound =
        u64::try_from(net.agents.len()).map_err(|_| Refusal::ArithmeticOverflow)?;
    let claims = Claims {
        net_digest: digest(ids, b"noble.ngraph.resource.net.v0", &net_bytes),
        rule_digest: digest(ids, b"noble.ngraph.resource.rules.v0", RULE_TABLE_BYTES),
        cost_digest: digest(ids, b"noble.ngraph.resource.cost.v0", COST_MODEL_BYTES),
        interactions,
        live_agent_bound,
    };
    Ok(Artifact {
        certificate_bytes: encode_certificate(ids.name(), claims)?,
        net_bytes,
    })
}

/// Checks a certificate using only its bytes, the decoded checked graph, and the selected
/// rule-table bytes. The certificate producer and its claims are not called or trusted.
pub fn check(
    ids: &dyn IdentityScheme,
    checked: &crate::check::CheckedGraph,
    inputs: &[crate::sem::Value],
    bytes: CheckBytes<'_>,
) -> Result<VerifiedResources, Refusal> {
    let CheckBytes {
        certificate: certificate_bytes,
        net: net_bytes,
        rules: rule_bytes,
    } = bytes;
    if net_bytes.len() > MAX_NET_BYTES {
        return Err(Refusal::Malformed);
    }
    let claims = decode_certificate(certificate_bytes, ids.name())?;
    if claims.net_digest != digest(ids, b"noble.ngraph.resource.net.v0", net_bytes) {
        return Err(Refusal::NetDigestMismatch);
    }
    if rule_bytes != RULE_TABLE_BYTES {
        return Err(Refusal::StaleRuleTable);
    }
    if claims.rule_digest != digest(ids, b"noble.ngraph.resource.rules.v0", rule_bytes) {
        return Err(Refusal::RuleDigestMismatch);
    }
    if claims.cost_digest != digest(ids, b"noble.ngraph.resource.cost.v0", COST_MODEL_BYTES) {
        return Err(Refusal::WrongCostModel);
    }
    let decoded = decode_net(net_bytes)?;
    crate::net::validate(
        ids,
        checked,
        &decoded,
        inputs,
        &crate::net::Limits::default(),
    )
    .map_err(|_| Refusal::InvalidNet)?;
    // Derive the charged interaction potential directly from the decoded agent symbols,
    // independently of the producer's graph-level rule-count formula.
    let interactions = decoded.agents.iter().try_fold(0_u64, |total, agent| {
        let charge = match agent.symbol {
            crate::net::Symbol::Num(_) => 0_u64,
            crate::net::Symbol::Op2(_) => 2,
            crate::net::Symbol::Op1(..) | crate::net::Symbol::Dup | crate::net::Symbol::Era => 1,
        };
        total.checked_add(charge).ok_or(Refusal::ArithmeticOverflow)
    })?;
    if decoded.expected_interactions() != Some(interactions) {
        return Err(Refusal::InvalidNet);
    }
    let live_agent_bound =
        u64::try_from(decoded.agents.len()).map_err(|_| Refusal::ArithmeticOverflow)?;
    let initial_wires = decoded.wire_count().ok_or(Refusal::Malformed)?;
    if claims.interactions != interactions {
        return Err(Refusal::WrongInteractionCount);
    }
    if claims.live_agent_bound != live_agent_bound {
        return Err(Refusal::WrongLiveAgentBound);
    }
    Ok(VerifiedResources {
        interactions,
        live_agent_bound,
        initial_wires,
    })
}

/// Encodes all identity-bearing fields and every agent, port, and interface endpoint of a
/// lowered net. The exact output is the byte sequence whose digest a certificate records.
pub fn encode_net(net: &crate::net::InteractionGraph) -> Result<alloc::vec::Vec<u8>, Refusal> {
    if net.outputs.len() != net.output_types.len() {
        return Err(Refusal::Malformed);
    }
    let encoded_bytes = net_encoded_bytes(net).ok_or(Refusal::Malformed)?;
    if encoded_bytes > MAX_NET_BYTES {
        return Err(Refusal::Malformed);
    }
    let mut bytes = alloc::vec::Vec::with_capacity(encoded_bytes);
    bytes.extend_from_slice(NET_MAGIC);
    bytes.extend_from_slice(&net.graph_id);
    bytes.extend_from_slice(&net.build_key);
    push_string(&mut bytes, net.contract_revision)?;
    push_string(&mut bytes, net.lowering_revision)?;
    push_count(&mut bytes, net.input_values.len())?;
    for value in &net.input_values {
        bytes.extend_from_slice(&(*value as u64).to_le_bytes());
    }
    push_count(&mut bytes, net.outputs.len())?;
    for (output, ty) in net.outputs.iter().zip(&net.output_types) {
        bytes.extend_from_slice(&output.agent.to_le_bytes());
        bytes.push(output.port_index.into());
        bytes.push(match ty {
            crate::ir::ScalarType::I64 => 0,
            crate::ir::ScalarType::Bool => 1,
            crate::ir::ScalarType::F32 => return Err(Refusal::Malformed),
        });
    }
    push_count(&mut bytes, net.agents.len())?;
    for agent in &net.agents {
        match agent.origin {
            crate::net::Origin::Node(node) => {
                bytes.push(0);
                bytes.extend_from_slice(&node.0.to_le_bytes());
            }
            crate::net::Origin::Dup { value, ordinal } => {
                bytes.push(1);
                bytes.extend_from_slice(&value.0.to_le_bytes());
                bytes.extend_from_slice(&ordinal.to_le_bytes());
            }
            crate::net::Origin::Era(value) => {
                bytes.push(2);
                bytes.extend_from_slice(&value.0.to_le_bytes());
            }
        }
        match agent.symbol {
            crate::net::Symbol::Num(value) => {
                bytes.push(0);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            crate::net::Symbol::Op2(op) => {
                bytes.push(1);
                bytes.push(encode_op(op));
            }
            crate::net::Symbol::Op1(op, first) => {
                bytes.push(2);
                bytes.push(encode_op(op));
                bytes.extend_from_slice(&first.to_le_bytes());
            }
            crate::net::Symbol::Dup => bytes.push(3),
            crate::net::Symbol::Era => bytes.push(4),
        }
        for endpoint in agent.ports {
            match endpoint {
                None => bytes.push(0),
                Some(crate::net::Endpoint::Port(port)) => {
                    bytes.push(1);
                    bytes.extend_from_slice(&port.agent.to_le_bytes());
                    bytes.push(port.port_index.into());
                }
                Some(crate::net::Endpoint::Output(output)) => {
                    bytes.push(2);
                    bytes.extend_from_slice(&output.to_le_bytes());
                }
            }
        }
    }
    if bytes.len() > MAX_NET_BYTES {
        return Err(Refusal::Malformed);
    }
    Ok(bytes)
}

fn net_encoded_bytes(net: &crate::net::InteractionGraph) -> Option<usize> {
    let parts_bytes = NET_MAGIC
        .len()
        .checked_add(DIGEST_BYTES.checked_mul(2)?)?
        .checked_add(2 + net.contract_revision.len())?
        .checked_add(2 + net.lowering_revision.len())?
        .checked_add(3 * core::mem::size_of::<u32>())?
        .checked_add(
            net.input_values
                .len()
                .checked_mul(core::mem::size_of::<u64>())?,
        )?
        .checked_add(net.outputs.len().checked_mul(6)?)?;
    net.agents
        .iter()
        .try_fold(parts_bytes, |total_bytes, agent| {
            let origin_bytes = match agent.origin {
                crate::net::Origin::Dup { .. } => 9,
                crate::net::Origin::Node(_) | crate::net::Origin::Era(_) => 5,
            };
            let symbol_bytes = match agent.symbol {
                crate::net::Symbol::Num(_) => 9,
                crate::net::Symbol::Op2(_) => 2,
                crate::net::Symbol::Op1(..) => 10,
                crate::net::Symbol::Dup | crate::net::Symbol::Era => 1,
            };
            let ports_bytes: usize = agent
                .ports
                .iter()
                .map(|endpoint| match endpoint {
                    None => 1,
                    Some(crate::net::Endpoint::Port(_)) => 6,
                    Some(crate::net::Endpoint::Output(_)) => 5,
                })
                .sum();
            total_bytes.checked_add(origin_bytes + symbol_bytes + ports_bytes)
        })
}

fn encode_op(op: crate::net::Op) -> u8 {
    match op {
        crate::net::Op::Add => 0,
        crate::net::Op::Sub => 1,
        crate::net::Op::Mul => 2,
        crate::net::Op::Eq => 3,
    }
}

fn decode_op(value: u8) -> Result<crate::net::Op, Refusal> {
    match value {
        0 => Ok(crate::net::Op::Add),
        1 => Ok(crate::net::Op::Sub),
        2 => Ok(crate::net::Op::Mul),
        3 => Ok(crate::net::Op::Eq),
        _ => Err(Refusal::Malformed),
    }
}

fn decode_net(bytes: &[u8]) -> Result<crate::net::InteractionGraph, Refusal> {
    if bytes.len() > MAX_NET_BYTES {
        return Err(Refusal::Malformed);
    }
    let mut reader = Reader::new(bytes);
    if reader.take(NET_MAGIC.len())? != NET_MAGIC {
        return Err(Refusal::Malformed);
    }
    let graph_id = reader.digest()?;
    let build_key = reader.digest()?;
    let contract_revision =
        reader.static_string(crate::net::CONTRACT_REVISION, Refusal::Malformed)?;
    let lowering_revision =
        reader.static_string(crate::net::LOWERING_REVISION, Refusal::Malformed)?;
    let input_count = reader.count()?;
    if input_count > reader.remaining() / 8 {
        return Err(Refusal::Malformed);
    }
    let mut input_values = alloc::vec::Vec::with_capacity(input_count);
    for _ in 0..input_count {
        input_values.push(reader.u64()? as i64);
    }
    let output_count = reader.count()?;
    if output_count > reader.remaining() / 6 {
        return Err(Refusal::Malformed);
    }
    let mut outputs = alloc::vec::Vec::with_capacity(output_count);
    let mut output_types = alloc::vec::Vec::with_capacity(output_count);
    for _ in 0..output_count {
        outputs.push(crate::net::PortRef {
            agent: reader.u32()?,
            port_index: reader.byte()?.into(),
        });
        output_types.push(match reader.byte()? {
            0 => crate::ir::ScalarType::I64,
            1 => crate::ir::ScalarType::Bool,
            _ => return Err(Refusal::Malformed),
        });
    }
    let agent_count = reader.count()?;
    if agent_count > reader.remaining() / 9 {
        return Err(Refusal::Malformed);
    }
    let mut agents = alloc::vec::Vec::with_capacity(agent_count);
    for _ in 0..agent_count {
        let origin = match reader.byte()? {
            0 => crate::net::Origin::Node(crate::ir::NodeId(reader.u32()?)),
            1 => crate::net::Origin::Dup {
                value: crate::ir::NodeId(reader.u32()?),
                ordinal: reader.u32()?,
            },
            2 => crate::net::Origin::Era(crate::ir::NodeId(reader.u32()?)),
            _ => return Err(Refusal::Malformed),
        };
        let symbol = match reader.byte()? {
            0 => crate::net::Symbol::Num(reader.u64()?),
            1 => crate::net::Symbol::Op2(decode_op(reader.byte()?)?),
            2 => crate::net::Symbol::Op1(decode_op(reader.byte()?)?, reader.u64()?),
            3 => crate::net::Symbol::Dup,
            4 => crate::net::Symbol::Era,
            _ => return Err(Refusal::Malformed),
        };
        let mut ports = [None; 3];
        for (index, endpoint) in ports.iter_mut().enumerate() {
            *endpoint = match reader.byte()? {
                0 => None,
                1 => Some(crate::net::Endpoint::Port(crate::net::PortRef {
                    agent: reader.u32()?,
                    port_index: reader.byte()?.into(),
                })),
                2 => Some(crate::net::Endpoint::Output(reader.u32()?)),
                _ => return Err(Refusal::Malformed),
            };
            if index < symbol.port_count().min(3) && endpoint.is_none() {
                return Err(Refusal::Malformed);
            }
            if index >= symbol.port_count() && endpoint.is_some() {
                return Err(Refusal::Malformed);
            }
        }
        agents.push(crate::net::Agent {
            origin,
            symbol,
            ports,
        });
    }
    if !reader.is_finished() {
        return Err(Refusal::Malformed);
    }
    Ok(crate::net::InteractionGraph {
        graph_id,
        contract_revision,
        lowering_revision,
        build_key,
        input_values,
        agents,
        outputs,
        output_types,
    })
}

fn encode_certificate(hash_scheme: &str, claims: Claims) -> Result<alloc::vec::Vec<u8>, Refusal> {
    let scheme = hash_scheme.as_bytes();
    if scheme.len() > MAX_HASH_SCHEME_BYTES {
        return Err(Refusal::Malformed);
    }
    let encoded_bytes = CERTIFICATE_MAGIC.len()
        + 2
        + CERTIFICATE_REVISION.len()
        + 2
        + CHECKER_REVISION.len()
        + 2
        + PROOF_MODEL_REVISION.len()
        + 2
        + COST_MODEL_REVISION.len()
        + DIGEST_BYTES * 3
        + 16
        + 1
        + scheme.len();
    let mut bytes = alloc::vec::Vec::with_capacity(encoded_bytes);
    bytes.extend_from_slice(CERTIFICATE_MAGIC);
    push_string(&mut bytes, CERTIFICATE_REVISION)?;
    push_string(&mut bytes, CHECKER_REVISION)?;
    push_string(&mut bytes, PROOF_MODEL_REVISION)?;
    push_string(&mut bytes, COST_MODEL_REVISION)?;
    bytes.extend_from_slice(&claims.net_digest);
    bytes.extend_from_slice(&claims.rule_digest);
    bytes.extend_from_slice(&claims.cost_digest);
    bytes.extend_from_slice(&claims.interactions.to_le_bytes());
    bytes.extend_from_slice(&claims.live_agent_bound.to_le_bytes());
    bytes.push(u8::try_from(scheme.len()).map_err(|_| Refusal::Malformed)?);
    bytes.extend_from_slice(scheme);
    Ok(bytes)
}

fn decode_certificate(bytes: &[u8], hash_scheme: &str) -> Result<Claims, Refusal> {
    if bytes.len() > MAX_CERTIFICATE_BYTES {
        return Err(Refusal::Malformed);
    }
    let mut reader = Reader::new(bytes);
    if reader.take(CERTIFICATE_MAGIC.len())? != CERTIFICATE_MAGIC {
        return Err(Refusal::StaleCertificateRevision);
    }
    reader.static_string(CERTIFICATE_REVISION, Refusal::StaleCertificateRevision)?;
    reader.static_string(CHECKER_REVISION, Refusal::StaleCheckerRevision)?;
    reader.static_string(PROOF_MODEL_REVISION, Refusal::StaleProofRevision)?;
    reader.static_string(COST_MODEL_REVISION, Refusal::StaleCostModelRevision)?;
    let claims = Claims {
        net_digest: reader.digest()?,
        rule_digest: reader.digest()?,
        cost_digest: reader.digest()?,
        interactions: reader.u64()?,
        live_agent_bound: reader.u64()?,
    };
    let scheme_len = usize::from(reader.byte()?);
    if scheme_len > MAX_HASH_SCHEME_BYTES || reader.take(scheme_len)? != hash_scheme.as_bytes() {
        return Err(Refusal::StaleHashScheme);
    }
    if !reader.is_finished() {
        return Err(Refusal::Malformed);
    }
    Ok(claims)
}

fn digest(ids: &dyn IdentityScheme, domain: &[u8], bytes: &[u8]) -> crate::ir::Digest {
    let mut hasher = ids.start();
    hasher.update(domain);
    hasher.update(&(u64::try_from(bytes.len()).unwrap_or(u64::MAX)).to_le_bytes());
    hasher.update(bytes);
    hasher.finish()
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Appending encoded fields requires a mutable owning Vec, not a fixed-size byte slice"
)]
fn push_count(bytes: &mut alloc::vec::Vec<u8>, count: usize) -> Result<(), Refusal> {
    if count > MAX_NET_ITEMS {
        return Err(Refusal::Malformed);
    }
    bytes.extend_from_slice(
        &u32::try_from(count)
            .map_err(|_| Refusal::Malformed)?
            .to_le_bytes(),
    );
    Ok(())
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Appending a variable-length string requires a mutable owning Vec, not a fixed-size byte slice"
)]
fn push_string(bytes: &mut alloc::vec::Vec<u8>, value: &str) -> Result<(), Refusal> {
    let raw = value.as_bytes();
    let length_bytes = u16::try_from(raw.len()).map_err(|_| Refusal::Malformed)?;
    bytes.extend_from_slice(&length_bytes.to_le_bytes());
    bytes.extend_from_slice(raw);
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset_bytes: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset_bytes: 0,
        }
    }

    fn take(&mut self, length_bytes: usize) -> Result<&'a [u8], Refusal> {
        let end = self
            .offset_bytes
            .checked_add(length_bytes)
            .ok_or(Refusal::Malformed)?;
        let bytes = self
            .bytes
            .get(self.offset_bytes..end)
            .ok_or(Refusal::Malformed)?;
        self.offset_bytes = end;
        Ok(bytes)
    }

    fn byte(&mut self) -> Result<u8, Refusal> {
        Ok(*self.take(1)?.first().ok_or(Refusal::Malformed)?)
    }

    fn u16(&mut self) -> Result<u16, Refusal> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, Refusal> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, Refusal> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn digest(&mut self) -> Result<crate::ir::Digest, Refusal> {
        let bytes = self.take(DIGEST_BYTES)?;
        let mut digest = [0; DIGEST_BYTES];
        digest.copy_from_slice(bytes);
        Ok(digest)
    }

    fn count(&mut self) -> Result<usize, Refusal> {
        let count = usize::try_from(self.u32()?).map_err(|_| Refusal::Malformed)?;
        if count > MAX_NET_ITEMS {
            return Err(Refusal::Malformed);
        }
        Ok(count)
    }

    fn static_string(
        &mut self,
        expected: &'static str,
        stale: Refusal,
    ) -> Result<&'static str, Refusal> {
        let length_bytes = usize::from(self.u16()?);
        if self.take(length_bytes)? != expected.as_bytes() {
            return Err(stale);
        }
        Ok(expected)
    }

    const fn is_finished(&self) -> bool {
        self.offset_bytes == self.bytes.len()
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHasher {
        state: crate::ir::Digest,
        position: usize,
    }

    impl crate::identity::IdentityHasher for TestHasher {
        fn update(&mut self, bytes: &[u8]) {
            for byte in bytes {
                let index = self.position;
                self.state[index] = self.state[index]
                    .wrapping_add(*byte)
                    .rotate_left(u32::try_from(index % 7).expect("small rotation"));
                self.position = if index + 1 == DIGEST_BYTES {
                    0
                } else {
                    index + 1
                };
            }
        }

        fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest {
            self.state
        }
    }

    struct TestIds;

    impl IdentityScheme for TestIds {
        fn name(&self) -> &'static str {
            "test-noncryptographic"
        }

        fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
            alloc::boxed::Box::new(TestHasher {
                state: [0; DIGEST_BYTES],
                position: 0,
            })
        }
    }

    const IDS: TestIds = TestIds;

    fn sample() -> (
        crate::check::CheckedGraph,
        crate::net::InteractionGraph,
        [crate::sem::Value; 2],
    ) {
        let subject = crate::ir::Subject::authored(&IDS, "resource-certificate-sample");
        let mut builder = crate::ir::GraphBuilder::new(crate::ir::SubProfile::I64, subject);
        let left = builder.input(crate::ir::ScalarType::I64);
        let right = builder.input(crate::ir::ScalarType::I64);
        let sum = builder.op(crate::ir::Op::I64Add(left, right));
        let difference = builder.op(crate::ir::Op::I64Sub(left, right));
        builder.output(sum);
        builder.output(difference);
        let checked = crate::check::check(&IDS, builder.finish()).expect("graph admission");
        let inputs = [crate::sem::Value::I64(7), crate::sem::Value::I64(3)];
        let net = crate::net::lower(&IDS, &checked, &inputs, &crate::net::Limits::default())
            .expect("net lowering");
        (checked, net, inputs)
    }

    fn artifact() -> (crate::check::CheckedGraph, [crate::sem::Value; 2], Artifact) {
        let (checked, net, inputs) = sample();
        let artifact = issue(&IDS, &checked, &net, &inputs).expect("certificate producer");
        (checked, inputs, artifact)
    }

    fn check_case(
        checked: &crate::check::CheckedGraph,
        inputs: &[crate::sem::Value],
        certificate: &[u8],
        net: &[u8],
        rules: &[u8],
    ) -> Result<VerifiedResources, Refusal> {
        check(
            &IDS,
            checked,
            inputs,
            CheckBytes {
                certificate,
                net,
                rules,
            },
        )
    }

    #[test]
    fn decoded_certificate_binds_exact_net_and_rule_bytes() {
        let (checked, inputs, artifact) = artifact();
        let resources = check_case(
            &checked,
            &inputs,
            &artifact.certificate_bytes,
            &artifact.net_bytes,
            RULE_TABLE_BYTES,
        )
        .expect("independent byte decoder and checker");
        assert_eq!(resources.interactions, 6);
        assert_eq!(resources.live_agent_bound, 6);
        assert_eq!(resources.initial_wires, 8);
        assert_eq!(
            decode_net(&artifact.net_bytes).expect("net bytes"),
            sample().1
        );
    }

    #[test]
    fn checker_refuses_wrong_count_and_live_bound() {
        let (checked, inputs, artifact) = artifact();
        let mut wrong_count = artifact.certificate_bytes.clone();
        wrong_count[RESOURCE_INTERACTIONS_OFFSET..RESOURCE_LIVE_BOUND_OFFSET]
            .copy_from_slice(&3_u64.to_le_bytes());
        assert_eq!(
            check_case(
                &checked,
                &inputs,
                &wrong_count,
                &artifact.net_bytes,
                RULE_TABLE_BYTES,
            ),
            Err(Refusal::WrongInteractionCount)
        );

        let mut wrong_bound = artifact.certificate_bytes.clone();
        wrong_bound[RESOURCE_LIVE_BOUND_OFFSET..RESOURCE_LIVE_BOUND_OFFSET + 8]
            .copy_from_slice(&4_u64.to_le_bytes());
        assert_eq!(
            check_case(
                &checked,
                &inputs,
                &wrong_bound,
                &artifact.net_bytes,
                RULE_TABLE_BYTES,
            ),
            Err(Refusal::WrongLiveAgentBound)
        );
    }

    #[test]
    fn checker_refuses_mismatched_net_stale_rules_and_truncation() {
        let (checked, inputs, artifact) = artifact();
        let mut changed_net = artifact.net_bytes.clone();
        changed_net[8] ^= 1;
        assert_eq!(
            check_case(
                &checked,
                &inputs,
                &artifact.certificate_bytes,
                &changed_net,
                RULE_TABLE_BYTES,
            ),
            Err(Refusal::NetDigestMismatch)
        );

        let mut stale_rules = RULE_TABLE_BYTES.to_vec();
        stale_rules[0] ^= 1;
        assert_eq!(
            check_case(
                &checked,
                &inputs,
                &artifact.certificate_bytes,
                &artifact.net_bytes,
                &stale_rules,
            ),
            Err(Refusal::StaleRuleTable)
        );

        assert_eq!(
            check_case(
                &checked,
                &inputs,
                &artifact.certificate_bytes[..artifact.certificate_bytes.len() - 1],
                &artifact.net_bytes,
                RULE_TABLE_BYTES,
            ),
            Err(Refusal::Malformed)
        );
    }

    #[test]
    fn checker_refuses_forged_cost_rule_and_model_identities() {
        let (checked, inputs, artifact) = artifact();
        let mut cost = artifact.certificate_bytes.clone();
        cost[RESOURCE_COST_DIGEST_OFFSET] ^= 1;
        assert_eq!(
            check_case(
                &checked,
                &inputs,
                &cost,
                &artifact.net_bytes,
                RULE_TABLE_BYTES
            ),
            Err(Refusal::WrongCostModel)
        );

        let mut omitted_erase = RULE_TABLE_BYTES.to_vec();
        let at = omitted_erase
            .windows(7)
            .position(|window| window == b"era-num")
            .expect("rule table includes erasure");
        omitted_erase.drain(at..at + 7);
        assert_eq!(
            check_case(
                &checked,
                &inputs,
                &artifact.certificate_bytes,
                &artifact.net_bytes,
                &omitted_erase
            ),
            Err(Refusal::StaleRuleTable)
        );

        let mut stale_checker = artifact.certificate_bytes.clone();
        let at = stale_checker
            .windows(CHECKER_REVISION.len())
            .position(|window| window == CHECKER_REVISION.as_bytes())
            .expect("certificate binds checker identity");
        stale_checker[at] ^= 1;
        assert_eq!(
            check_case(
                &checked,
                &inputs,
                &stale_checker,
                &artifact.net_bytes,
                RULE_TABLE_BYTES
            ),
            Err(Refusal::StaleCheckerRevision)
        );
    }

    #[test]
    fn decoder_refuses_malformed_nets_even_with_recomputed_digest() {
        let (checked, net, inputs) = sample();
        let mut bytes = encode_net(&net).expect("encode net");
        bytes[8] ^= 1;
        let claims = Claims {
            net_digest: digest(&IDS, b"noble.ngraph.resource.net.v0", &bytes),
            rule_digest: digest(&IDS, b"noble.ngraph.resource.rules.v0", RULE_TABLE_BYTES),
            cost_digest: digest(&IDS, b"noble.ngraph.resource.cost.v0", COST_MODEL_BYTES),
            interactions: 6,
            live_agent_bound: 6,
        };
        let certificate = encode_certificate(IDS.name(), claims).expect("encode claims");
        assert_eq!(
            check_case(&checked, &inputs, &certificate, &bytes, RULE_TABLE_BYTES),
            Err(Refusal::InvalidNet)
        );
    }
}
