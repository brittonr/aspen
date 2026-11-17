#![expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the canonical persistence codec handles untrusted bytes and bounded exports through explicit fallible checks and typed refusals, so assertions would panic instead of validating stored artifacts."
)]

use super::*;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

const DOMAIN_ID: u64 = 0x4e_50_52_4f_43_00_00_01;
const DOMAIN_STATE: u64 = 0x4e_50_52_4f_43_00_00_02;
const DOMAIN_LIBRARY: u64 = 0x4e_50_52_4f_43_00_00_03;
const MAX_STRING_BYTES: u32 = 4_096;
const MAX_TOOLCHAINS: u32 = 16;
const MAX_BINDINGS: u32 = 16;
const MAX_RECIPE_NODES: u32 = chunks::MAX_RECIPE_NODES;
const MAX_DEPTH_LIST: u32 = chunks::MAX_STACK_HEIGHT;
const MAGIC: &[u8; 8] = b"NPLIB001";

#[derive(Clone, Copy)]
struct JsonLimitDiagnostic {
    field: &'static str,
    message: &'static str,
}

const MANIFEST_LIMIT_DIAGNOSTIC: JsonLimitDiagnostic = JsonLimitDiagnostic {
    field: "manifest.bytes",
    message: "identity manifest exceeds its bound",
};

const METRICS_LIMIT_DIAGNOSTIC: JsonLimitDiagnostic = JsonLimitDiagnostic {
    field: "metrics.bytes",
    message: "metrics record exceeds its bound",
};

struct BoundedJson {
    text: String,
    maximum_bytes: usize,
    exceeded_limit: bool,
}

impl BoundedJson {
    fn new(maximum_bytes: usize) -> Self {
        Self {
            text: String::new(),
            maximum_bytes,
            exceeded_limit: false,
        }
    }

    fn push_str(&mut self, value: &str) {
        if self.exceeded_limit {
            return;
        }
        let Some(next_byte_len) = self.text.len().checked_add(value.len()) else {
            self.exceeded_limit = true;
            return;
        };
        if next_byte_len > self.maximum_bytes {
            self.exceeded_limit = true;
            return;
        }
        self.text.push_str(value);
    }

    fn push(&mut self, value: char) {
        if self.exceeded_limit {
            return;
        }
        let Some(next_byte_len) = self.text.len().checked_add(value.len_utf8()) else {
            self.exceeded_limit = true;
            return;
        };
        if next_byte_len > self.maximum_bytes {
            self.exceeded_limit = true;
            return;
        }
        self.text.push(value);
    }

    fn push_u64(&mut self, mut value: u64) {
        let mut digits = [0u8; 20];
        let mut start = digits.len();
        loop {
            start -= 1;
            let Ok(digit) = u8::try_from(value % 10) else {
                self.exceeded_limit = true;
                return;
            };
            digits[start] = b"0123456789"[usize::from(digit)];
            value /= 10;
            if value == 0 {
                break;
            }
        }
        for digit in &digits[start..] {
            self.push(char::from(*digit));
        }
    }

    fn push_u32(&mut self, value: u32) {
        self.push_u64(u64::from(value));
    }

    fn push_usize(&mut self, value: usize) {
        match u64::try_from(value) {
            Ok(value) => self.push_u64(value),
            Err(_) => self.exceeded_limit = true,
        }
    }

    fn push_bool(&mut self, value: bool) {
        self.push_str(if value { "true" } else { "false" });
    }

    fn push_hex(&mut self, bytes: &[u8]) {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        for byte in bytes {
            if self.exceeded_limit {
                break;
            }
            self.push(char::from(DIGITS[usize::from(byte >> 4)]));
            self.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
    }

    fn push_json_string(&mut self, value: &str) {
        self.push('"');
        for character in value.chars() {
            if self.exceeded_limit {
                break;
            }
            match character {
                '"' => self.push_str("\\\""),
                '\\' => self.push_str("\\\\"),
                '\n' => self.push_str("\\n"),
                '\r' => self.push_str("\\r"),
                '\t' => self.push_str("\\t"),
                character if character <= '\u{001f}' => {
                    self.push_str("\\u00");
                    if let Ok(ascii) = u8::try_from(u32::from(character)) {
                        self.push_hex(&[ascii]);
                    } else {
                        self.exceeded_limit = true;
                    }
                }
                character => self.push(character),
            }
        }
        self.push('"');
    }

    fn ensure_within_limit(&self, diagnostic: JsonLimitDiagnostic) -> Result<(), Refusal> {
        if self.exceeded_limit {
            Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: diagnostic.field,
                    message: diagnostic.message,
                },
            ))
        } else {
            Ok(())
        }
    }

    fn finish(self, diagnostic: JsonLimitDiagnostic) -> Result<Vec<u8>, Refusal> {
        self.ensure_within_limit(diagnostic)?;
        Ok(self.text.into_bytes())
    }
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn bool(&mut self, value: bool) {
        self.byte(u8::from(value));
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn sized(
        &mut self,
        value: &[u8],
        maximum_bytes: u32,
        field: &'static str,
    ) -> Result<(), Refusal> {
        let length_bytes = u32::try_from(value.len()).map_err(|_| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field,
                    message: "string exceeds canonical record limit",
                },
            )
        })?;
        if length_bytes > maximum_bytes {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field,
                    message: "string exceeds canonical record limit",
                },
            ));
        }
        self.u32(length_bytes);
        self.raw(value);
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), Refusal> {
        self.sized(value.as_bytes(), MAX_STRING_BYTES, "serialization.string")
    }

    fn bounded_count<MaximumItems>(
        &mut self,
        item_count: usize,
        maximum_items: MaximumItems,
        field: &'static str,
    ) -> Result<(), Refusal>
    where
        MaximumItems: TryInto<u64>,
    {
        let maximum_wire_items = maximum_items.try_into().map_err(|_| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field,
                    message: "collection limit exceeds the wire count domain",
                },
            )
        })?;
        let wire_item_count = u32::try_from(item_count).map_err(|_| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field,
                    message: "collection count exceeds the wire domain",
                },
            )
        })?;
        if u64::from(wire_item_count) > maximum_wire_items {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field,
                    message: "collection exceeds its declared bound",
                },
            ));
        }
        self.u32(wire_item_count);
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    byte_offset_bytes: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_offset_bytes: 0,
        }
    }

    fn read<const N: usize>(&mut self, field: &'static str) -> Result<[u8; N], Refusal> {
        let end_byte_offset_bytes = self.byte_offset_bytes.checked_add(N).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field,
                    message: "canonical offset overflow",
                },
            )
        })?;
        let value = self
            .bytes
            .get(self.byte_offset_bytes..end_byte_offset_bytes)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field,
                        message: "truncated canonical record",
                    },
                )
            })?;
        let mut out = [0; N];
        out.copy_from_slice(value);
        self.byte_offset_bytes = end_byte_offset_bytes;
        Ok(out)
    }

    fn byte(&mut self, field: &'static str) -> Result<u8, Refusal> {
        Ok(self.read::<1>(field)?[0])
    }

    fn bool(&mut self, field: &'static str) -> Result<bool, Refusal> {
        match self.byte(field)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field,
                    message: "noncanonical Boolean tag",
                },
            )),
        }
    }

    fn u16(&mut self, field: &'static str) -> Result<u16, Refusal> {
        Ok(u16::from_be_bytes(self.read::<2>(field)?))
    }

    fn u32(&mut self, field: &'static str) -> Result<u32, Refusal> {
        Ok(u32::from_be_bytes(self.read::<4>(field)?))
    }

    fn u64(&mut self, field: &'static str) -> Result<u64, Refusal> {
        Ok(u64::from_be_bytes(self.read::<8>(field)?))
    }

    fn i64(&mut self, field: &'static str) -> Result<i64, Refusal> {
        Ok(i64::from_be_bytes(self.read::<8>(field)?))
    }

    fn count<MaximumItems>(
        &mut self,
        field: &'static str,
        maximum_items: MaximumItems,
    ) -> Result<usize, Refusal>
    where
        MaximumItems: TryInto<u64>,
    {
        let maximum_wire_items = maximum_items.try_into().map_err(|_| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field,
                    message: "collection limit exceeds the wire count domain",
                },
            )
        })?;
        let item_count = self.u32(field)?;
        if u64::from(item_count) > maximum_wire_items {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field,
                    message: "canonical collection exceeds its declared bound",
                },
            ));
        }
        usize::try_from(item_count).map_err(|_| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field,
                    message: "count cannot fit host index",
                },
            )
        })
    }

    fn raw<const N: usize>(&mut self, field: &'static str) -> Result<[u8; N], Refusal> {
        self.read::<N>(field)
    }

    fn sized(&mut self, field: &'static str, maximum_bytes: u32) -> Result<&'a [u8], Refusal> {
        let length_bytes = self.count(field, maximum_bytes)?;
        let end_byte_offset_bytes = self
            .byte_offset_bytes
            .checked_add(length_bytes)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field,
                        message: "canonical offset overflow",
                    },
                )
            })?;
        let value = self
            .bytes
            .get(self.byte_offset_bytes..end_byte_offset_bytes)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field,
                        message: "truncated canonical byte string",
                    },
                )
            })?;
        self.byte_offset_bytes = end_byte_offset_bytes;
        Ok(value)
    }

    fn string(&mut self, field: &'static str) -> Result<String, Refusal> {
        let bytes = self.sized(field, MAX_STRING_BYTES)?;
        let value = core::str::from_utf8(bytes).map_err(|_| {
            Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field,
                    message: "canonical string is not UTF-8",
                },
            )
        })?;
        Ok(String::from(value))
    }

    fn finished(&self) -> bool {
        self.byte_offset_bytes == self.bytes.len()
    }
}

fn digest32(domain: u64, bytes: &[u8]) -> ProcedureId {
    let mut output = [0; 32];
    let lane_domains = [
        domain,
        domain ^ 0x9e37_79b9_7f4a_7c15,
        domain ^ 0x3c6e_f372_fe94_f82a,
        domain ^ 0xdaa6_6d2c_7ddf_743f,
    ];
    for (output_lane, lane_domain) in output.chunks_exact_mut(8).zip(lane_domains) {
        let mut fold = crate::companion::digest::Fold::new(lane_domain);
        fold.absorb_bytes(bytes);
        output_lane.copy_from_slice(&fold.finish().to_be_bytes());
    }
    output
}

pub(super) fn identity_bytes(record: &ProcedureRecord) -> Result<Vec<u8>, Refusal> {
    subject_bytes(
        &record.encoder,
        record.semantic_revision,
        &record.chunk,
        &record.provenance,
        &record.library_dependencies,
    )
}

fn subject_bytes(
    encoder: &Encoder,
    semantic_revision: u32,
    chunk: &chunks::Envelope,
    provenance: &Provenance,
    dependencies: &[LibraryDependency],
) -> Result<Vec<u8>, Refusal> {
    if dependencies.len() > MAX_LIBRARY_DEPENDENCIES {
        return Err(Refusal::new(
            RefusalKind::Exhausted,
            super::Diagnostic {
                field: "library_dependencies",
                message: "identity dependency closure exceeds its declared bound",
            },
        ));
    }
    let mut writer = Writer::new();
    write_encoder(&mut writer, encoder)?;
    writer.u32(semantic_revision);
    write_chunk(&mut writer, chunk)?;
    write_provenance(&mut writer, provenance)?;
    writer.bounded_count(
        dependencies.len(),
        MAX_LIBRARY_DEPENDENCIES,
        "library_dependencies",
    )?;
    for dependency in dependencies {
        writer.raw(&dependency.procedure_id);
        writer.raw(&dependency.chunk_id);
        writer.raw(&dependency.source_revision);
    }
    Ok(writer.bytes)
}

pub(super) fn identity(
    encoder: &Encoder,
    semantic_revision: u32,
    chunk: &chunks::Envelope,
    provenance: &Provenance,
    dependencies: &[LibraryDependency],
) -> Result<ProcedureId, Refusal> {
    Ok(digest32(
        DOMAIN_ID,
        &subject_bytes(encoder, semantic_revision, chunk, provenance, dependencies)?,
    ))
}

pub(super) fn state_digest(record: &ProcedureRecord) -> Result<ProcedureId, Refusal> {
    let mut writer = Writer::new();
    write_state(&mut writer, record)?;
    Ok(digest32(DOMAIN_STATE, &writer.bytes))
}

pub(super) fn empty_procedure_revision() -> ProcedureId {
    digest32(DOMAIN_LIBRARY, &[])
}

pub(super) fn procedure_revision(records: &[ProcedureRecord]) -> Result<ProcedureId, Refusal> {
    let mut writer = Writer::new();
    for record in records {
        writer.raw(&record.procedure_id);
        writer.raw(&state_digest(record)?);
    }
    Ok(digest32(DOMAIN_LIBRARY, &writer.bytes))
}

pub(super) fn compare_evidence(left: &EvidenceRecord, right: &EvidenceRecord) -> Ordering {
    left.obligation
        .cmp(&right.obligation)
        .then_with(|| left.class.rank().cmp(&right.class.rank()))
        .then_with(|| left.outcome.code().cmp(right.outcome.code()))
        .then_with(|| left.policy_revision.cmp(&right.policy_revision))
        .then_with(|| left.toolchain.cmp(&right.toolchain))
        .then_with(|| left.claim.cmp(&right.claim))
        .then_with(|| left.subject_binding.cmp(&right.subject_binding))
}

fn write_encoder(writer: &mut Writer, encoder: &Encoder) -> Result<(), Refusal> {
    writer.string(&encoder.id)?;
    writer.u32(encoder.encoding_revision);
    writer.bool(encoder.experimental);
    Ok(())
}

fn write_type(writer: &mut Writer, ty: &Ty) -> Result<(), Refusal> {
    match ty {
        Ty::I64 => writer.byte(0),
        Ty::Bool => writer.byte(1),
        _ => {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                super::Diagnostic {
                    field: "chunk.interface",
                    message: "canonical procedure profile supports only I64 and Bool",
                },
            ))
        }
    }
    Ok(())
}

fn read_type(reader: &mut Reader<'_>, field: &'static str) -> Result<Ty, Refusal> {
    match reader.byte(field)? {
        0 => Ok(Ty::I64),
        1 => Ok(Ty::Bool),
        _ => Err(Refusal::new(
            RefusalKind::Unsupported,
            super::Diagnostic {
                field,
                message: "unsupported type tag",
            },
        )),
    }
}

fn write_types(writer: &mut Writer, types: &[Ty]) -> Result<(), Refusal> {
    if u32::try_from(types.len()).map_or(true, |type_count| type_count > MAX_DEPTH_LIST) {
        return Err(Refusal::new(
            RefusalKind::Exhausted,
            super::Diagnostic {
                field: "chunk.interface",
                message: "stack exceeds profile",
            },
        ));
    }
    writer.bounded_count(types.len(), MAX_DEPTH_LIST, "interface.types")?;
    for ty in types {
        write_type(writer, ty)?;
    }
    Ok(())
}

fn read_types(reader: &mut Reader<'_>, field: &'static str) -> Result<Vec<Ty>, Refusal> {
    let count = reader.count(field, MAX_DEPTH_LIST)?;
    let mut types = Vec::with_capacity(count);
    for _ in 0..count {
        types.push(read_type(reader, field)?);
    }
    Ok(types)
}

fn write_scalar(writer: &mut Writer, scalar: chunks::Scalar) {
    match scalar {
        chunks::Scalar::I64(value) => {
            writer.byte(0);
            writer.i64(value);
        }
        chunks::Scalar::Bool(value) => {
            writer.byte(1);
            writer.bool(value);
        }
    }
}

fn read_scalar(reader: &mut Reader<'_>, field: &'static str) -> Result<chunks::Scalar, Refusal> {
    match reader.byte(field)? {
        0 => Ok(chunks::Scalar::I64(reader.i64(field)?)),
        1 => Ok(chunks::Scalar::Bool(reader.bool(field)?)),
        _ => Err(Refusal::new(
            RefusalKind::Invalid,
            super::Diagnostic {
                field,
                message: "unknown scalar tag",
            },
        )),
    }
}

fn write_op(writer: &mut Writer, op: chunks::Op) {
    writer.byte(match op {
        chunks::Op::Dup => 0,
        chunks::Op::Drop => 1,
        chunks::Op::Swap => 2,
        chunks::Op::Add => 3,
        chunks::Op::Sub => 4,
        chunks::Op::Mul => 5,
        chunks::Op::Eq => 6,
        chunks::Op::Run => 7,
    });
}

fn read_op(reader: &mut Reader<'_>, field: &'static str) -> Result<chunks::Op, Refusal> {
    match reader.byte(field)? {
        0 => Ok(chunks::Op::Dup),
        1 => Ok(chunks::Op::Drop),
        2 => Ok(chunks::Op::Swap),
        3 => Ok(chunks::Op::Add),
        4 => Ok(chunks::Op::Sub),
        5 => Ok(chunks::Op::Mul),
        6 => Ok(chunks::Op::Eq),
        7 => Ok(chunks::Op::Run),
        _ => Err(Refusal::new(
            RefusalKind::Invalid,
            super::Diagnostic {
                field,
                message: "unknown operation tag",
            },
        )),
    }
}

fn write_chunk(writer: &mut Writer, chunk: &chunks::Envelope) -> Result<(), Refusal> {
    writer.u32(chunk.format);
    writer.u32(chunk.semantic_revision);
    writer.string(&chunk.profile)?;
    writer.raw(&chunk.chunk_id);
    writer.raw(&chunk.source_revision);
    write_types(writer, &chunk.interface.stack_in)?;
    write_types(writer, &chunk.interface.stack_out)?;
    if !chunk.interface.effects.is_empty() {
        return Err(Refusal::new(
            RefusalKind::Unsupported,
            super::Diagnostic {
                field: "chunk.interface.effects",
                message: "only the empty effect set is serializable in this profile",
            },
        ));
    }
    writer.u32(0);
    write_chunk_captures_and_dependencies(writer, chunk)?;
    write_chunk_instantiations(writer, chunk)?;
    write_chunk_body(writer, chunk)
}

fn write_chunk_captures_and_dependencies(
    writer: &mut Writer,
    chunk: &chunks::Envelope,
) -> Result<(), Refusal> {
    writer.bounded_count(chunk.captures.len(), chunks::MAX_CAPTURES, "chunk.captures")?;
    for capture in &chunk.captures {
        writer.u16(capture.index);
        write_scalar(writer, capture.value);
    }
    writer.bounded_count(
        chunk.dependencies.len(),
        MAX_LIBRARY_DEPENDENCIES,
        "chunk.dependencies",
    )?;
    for dependency in &chunk.dependencies {
        writer.u64(dependency.identity);
        writer.u32(dependency.format);
        writer.u32(dependency.semantic_revision);
        writer.raw(&dependency.source_revision);
    }
    Ok(())
}

fn variable_kind_tag(kind: noble_kernel::words::VariableKind) -> Result<u8, Refusal> {
    if kind == noble_kernel::words::VariableKind::Stack {
        return Ok(0);
    }
    if kind == noble_kernel::words::VariableKind::Value {
        return Ok(1);
    }
    if kind == noble_kernel::words::VariableKind::Effect {
        return Ok(2);
    }
    Err(Refusal::new(
        RefusalKind::Unsupported,
        super::Diagnostic {
            field: "chunk.instantiations.kind",
            message: "variable kind is outside the persisted profile",
        },
    ))
}

fn write_chunk_instantiations(
    writer: &mut Writer,
    chunk: &chunks::Envelope,
) -> Result<(), Refusal> {
    writer.bounded_count(
        chunk.instantiations.len(),
        MAX_RECIPE_NODES,
        "chunk.instantiations",
    )?;
    for instantiation in &chunk.instantiations {
        writer.bounded_count(
            instantiation.bindings.len(),
            MAX_BINDINGS,
            "chunk.instantiations.bindings",
        )?;
        for binding in &instantiation.bindings {
            writer.u16(binding.variable);
            writer.byte(variable_kind_tag(binding.kind)?);
            write_type(writer, &binding.ty)?;
        }
    }
    Ok(())
}

fn write_chunk_body(writer: &mut Writer, chunk: &chunks::Envelope) -> Result<(), Refusal> {
    writer.bounded_count(chunk.body.nodes.len(), MAX_RECIPE_NODES, "chunk.body.nodes")?;
    for node in &chunk.body.nodes {
        match node {
            chunks::Node::I64(value) => {
                writer.byte(0);
                writer.i64(*value);
            }
            chunks::Node::Bool(value) => {
                writer.byte(1);
                writer.bool(*value);
            }
            chunks::Node::Capture { index } => {
                writer.byte(2);
                writer.u16(*index);
            }
            chunks::Node::Operation { op, instantiation } => {
                writer.byte(3);
                write_op(writer, *op);
                writer.u16(*instantiation);
            }
            chunks::Node::Invoke {
                dependency,
                instantiation,
            } => {
                writer.byte(4);
                writer.u16(*dependency);
                writer.u16(*instantiation);
            }
            chunks::Node::Quotation {
                body,
                instantiation,
            } => {
                writer.byte(5);
                writer.bounded_count(body.len(), MAX_RECIPE_NODES, "chunk.body.quotation")?;
                for node in body {
                    writer.u32(*node);
                }
                writer.u16(*instantiation);
            }
        }
    }
    writer.bounded_count(chunk.body.entry.len(), MAX_RECIPE_NODES, "chunk.body.entry")?;
    for node in &chunk.body.entry {
        writer.u32(*node);
    }
    Ok(())
}

fn read_chunk(reader: &mut Reader<'_>) -> Result<chunks::Envelope, Refusal> {
    let format = reader.u32("chunk.format")?;
    let semantic_revision = reader.u32("chunk.semantic_revision")?;
    let profile = reader.string("chunk.profile")?;
    let chunk_id = reader.raw("chunk.chunk_id")?;
    let source_revision = reader.raw("chunk.source_revision")?;
    let stack_in = read_types(reader, "chunk.interface.stack_in")?;
    let stack_out = read_types(reader, "chunk.interface.stack_out")?;
    if reader.u32("chunk.interface.effects")? != 0 {
        return Err(Refusal::new(
            RefusalKind::Unsupported,
            super::Diagnostic {
                field: "chunk.interface.effects",
                message: "nonempty effect set is outside the stored profile",
            },
        ));
    }
    let captures = read_chunk_captures(reader)?;
    let dependencies = read_chunk_dependencies(reader)?;
    let instantiations = read_chunk_instantiations(reader)?;
    let body = read_chunk_body(reader)?;
    let envelope = chunks::Envelope {
        format,
        semantic_revision,
        profile,
        chunk_id,
        source_revision,
        interface: chunks::Interface {
            stack_in,
            stack_out,
            effects: EffSet::empty(),
        },
        captures,
        dependencies,
        instantiations,
        body,
    };
    debug_assert!(envelope.interface.effects.is_empty());
    Ok(envelope)
}

fn read_chunk_captures(reader: &mut Reader<'_>) -> Result<Vec<chunks::Capture>, Refusal> {
    let capture_count = reader.count("chunk.captures", chunks::MAX_CAPTURES)?;
    let mut captures = Vec::with_capacity(capture_count);
    for _ in 0..capture_count {
        captures.push(chunks::Capture {
            index: reader.u16("chunk.captures.index")?,
            value: read_scalar(reader, "chunk.captures.value")?,
        });
    }
    Ok(captures)
}

fn read_chunk_dependencies(reader: &mut Reader<'_>) -> Result<Vec<chunks::DefinitionRef>, Refusal> {
    let dependency_count = reader.count("chunk.dependencies", MAX_LIBRARY_DEPENDENCIES)?;
    let mut dependencies = Vec::with_capacity(dependency_count);
    for _ in 0..dependency_count {
        dependencies.push(chunks::DefinitionRef {
            identity: reader.u64("chunk.dependencies.identity")?,
            format: reader.u32("chunk.dependencies.format")?,
            semantic_revision: reader.u32("chunk.dependencies.semantic_revision")?,
            source_revision: reader.raw("chunk.dependencies.source_revision")?,
        });
    }
    Ok(dependencies)
}

fn read_chunk_instantiations(
    reader: &mut Reader<'_>,
) -> Result<Vec<chunks::Instantiation>, Refusal> {
    let instantiation_count = reader.count("chunk.instantiations", MAX_RECIPE_NODES)?;
    let mut instantiations = Vec::with_capacity(instantiation_count);
    for _ in 0..instantiation_count {
        let binding_count = reader.count("chunk.instantiations.bindings", MAX_BINDINGS)?;
        let mut bindings = Vec::with_capacity(binding_count);
        for _ in 0..binding_count {
            let variable = reader.u16("chunk.instantiations.variable")?;
            let kind = match reader.byte("chunk.instantiations.kind")? {
                0 => noble_kernel::words::VariableKind::Stack,
                1 => noble_kernel::words::VariableKind::Value,
                2 => noble_kernel::words::VariableKind::Effect,
                _ => {
                    return Err(Refusal::new(
                        RefusalKind::Invalid,
                        super::Diagnostic {
                            field: "chunk.instantiations.kind",
                            message: "unknown variable kind tag",
                        },
                    ))
                }
            };
            let ty = read_type(reader, "chunk.instantiations.type")?;
            bindings.push(chunks::Binding { variable, kind, ty });
        }
        instantiations.push(chunks::Instantiation { bindings });
    }
    Ok(instantiations)
}

fn read_chunk_body(reader: &mut Reader<'_>) -> Result<chunks::Recipe, Refusal> {
    let nodes = read_chunk_body_nodes(reader)?;
    let entry = read_chunk_body_entry(reader)?;
    Ok(chunks::Recipe { nodes, entry })
}

fn read_chunk_body_nodes(reader: &mut Reader<'_>) -> Result<Vec<chunks::Node>, Refusal> {
    let node_count = reader.count("chunk.body.nodes", MAX_RECIPE_NODES)?;
    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        let node = match reader.byte("chunk.body.node")? {
            0 => chunks::Node::I64(reader.i64("chunk.body.i64")?),
            1 => chunks::Node::Bool(reader.bool("chunk.body.bool")?),
            2 => chunks::Node::Capture {
                index: reader.u16("chunk.body.capture")?,
            },
            3 => chunks::Node::Operation {
                op: read_op(reader, "chunk.body.operation")?,
                instantiation: reader.u16("chunk.body.instantiation")?,
            },
            4 => chunks::Node::Invoke {
                dependency: reader.u16("chunk.body.dependency")?,
                instantiation: reader.u16("chunk.body.instantiation")?,
            },
            5 => {
                let body_count = reader.count("chunk.body.quotation", MAX_RECIPE_NODES)?;
                let mut body = Vec::with_capacity(body_count);
                for _ in 0..body_count {
                    body.push(reader.u32("chunk.body.quotation.node")?);
                }
                chunks::Node::Quotation {
                    body,
                    instantiation: reader.u16("chunk.body.instantiation")?,
                }
            }
            _ => {
                return Err(Refusal::new(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field: "chunk.body.node",
                        message: "unknown node tag",
                    },
                ))
            }
        };
        nodes.push(node);
    }
    Ok(nodes)
}

fn read_chunk_body_entry(reader: &mut Reader<'_>) -> Result<Vec<u32>, Refusal> {
    let entry_count = reader.count("chunk.body.entry", MAX_RECIPE_NODES)?;
    let mut entry = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        entry.push(reader.u32("chunk.body.entry.node")?);
    }
    Ok(entry)
}

fn write_provenance(writer: &mut Writer, provenance: &Provenance) -> Result<(), Refusal> {
    writer.byte(match provenance.producer {
        Producer::Chunk => 0,
        Producer::Synthesis => 1,
        Producer::Composition => 2,
    });
    writer.u32(provenance.producer_tool_revision);
    match provenance.task_id {
        Some(task_id) => {
            writer.byte(1);
            writer.raw(&task_id);
        }
        None => writer.byte(0),
    }
    match provenance.grammar_revision {
        Some(revision) => {
            writer.byte(1);
            writer.u32(revision);
        }
        None => writer.byte(0),
    }
    Ok(())
}

fn read_provenance(reader: &mut Reader<'_>) -> Result<Provenance, Refusal> {
    let producer = match reader.byte("provenance.producer")? {
        0 => Producer::Chunk,
        1 => Producer::Synthesis,
        2 => Producer::Composition,
        _ => {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "provenance.producer",
                    message: "unknown producer tag",
                },
            ))
        }
    };
    let producer_tool_revision = reader.u32("provenance.producer_tool_revision")?;
    let task_id = if reader.bool("provenance.task_id.present")? {
        Some(reader.raw("provenance.task_id")?)
    } else {
        None
    };
    let grammar_revision = if reader.bool("provenance.grammar_revision.present")? {
        Some(reader.u32("provenance.grammar_revision")?)
    } else {
        None
    };
    Ok(Provenance {
        producer,
        producer_tool_revision,
        task_id,
        grammar_revision,
    })
}

fn write_evidence_class(writer: &mut Writer, class: EvidenceClass) {
    writer.byte(class.rank());
}

fn read_evidence_class(reader: &mut Reader<'_>) -> Result<EvidenceClass, Refusal> {
    match reader.byte("evidence.class")? {
        0 => Ok(EvidenceClass::Lean),
        1 => Ok(EvidenceClass::AeneasLean),
        2 => Ok(EvidenceClass::Verus),
        3 => Ok(EvidenceClass::Validation),
        4 => Ok(EvidenceClass::Test),
        5 => Ok(EvidenceClass::Review),
        6 => Ok(EvidenceClass::Assumption),
        _ => Err(Refusal::new(
            RefusalKind::Invalid,
            super::Diagnostic {
                field: "evidence.class",
                message: "unknown evidence class",
            },
        )),
    }
}

fn write_outcome(writer: &mut Writer, outcome: EvidenceOutcome) {
    writer.byte(match outcome {
        EvidenceOutcome::Proved => 0,
        EvidenceOutcome::Disproved => 1,
        EvidenceOutcome::Unknown => 2,
        EvidenceOutcome::Timeout => 3,
        EvidenceOutcome::Unsupported => 4,
        EvidenceOutcome::Error => 5,
        EvidenceOutcome::NotRun => 6,
        EvidenceOutcome::Passed => 7,
        EvidenceOutcome::Failed => 8,
    });
}

fn read_outcome(reader: &mut Reader<'_>) -> Result<EvidenceOutcome, Refusal> {
    match reader.byte("evidence.outcome")? {
        0 => Ok(EvidenceOutcome::Proved),
        1 => Ok(EvidenceOutcome::Disproved),
        2 => Ok(EvidenceOutcome::Unknown),
        3 => Ok(EvidenceOutcome::Timeout),
        4 => Ok(EvidenceOutcome::Unsupported),
        5 => Ok(EvidenceOutcome::Error),
        6 => Ok(EvidenceOutcome::NotRun),
        7 => Ok(EvidenceOutcome::Passed),
        8 => Ok(EvidenceOutcome::Failed),
        _ => Err(Refusal::new(
            RefusalKind::Invalid,
            super::Diagnostic {
                field: "evidence.outcome",
                message: "unknown outcome",
            },
        )),
    }
}

fn write_status(writer: &mut Writer, status: Status) {
    writer.byte(match status {
        Status::Candidate => 0,
        Status::Tested => 1,
        Status::Promoted => 2,
        Status::Superseded => 3,
    });
}

fn read_status(reader: &mut Reader<'_>) -> Result<Status, Refusal> {
    match reader.byte("status")? {
        0 => Ok(Status::Candidate),
        1 => Ok(Status::Tested),
        2 => Ok(Status::Promoted),
        3 => Ok(Status::Superseded),
        _ => Err(Refusal::new(
            RefusalKind::Invalid,
            super::Diagnostic {
                field: "status",
                message: "unknown status",
            },
        )),
    }
}

fn write_state(writer: &mut Writer, record: &ProcedureRecord) -> Result<(), Refusal> {
    write_library_dependencies(writer, &record.library_dependencies)?;
    write_evidence_records(writer, &record.evidence)?;
    write_status(writer, record.status);
    write_promotion(writer, record.promotion.as_ref())?;
    write_cost(writer, record.cost);
    write_retrieval_signature(writer, &record.retrieval)?;
    write_supersedes(writer, record.supersedes);
    write_alias(writer, record.alias_of);
    Ok(())
}

fn write_library_dependencies(
    writer: &mut Writer,
    dependencies: &[LibraryDependency],
) -> Result<(), Refusal> {
    writer.bounded_count(
        dependencies.len(),
        MAX_LIBRARY_DEPENDENCIES,
        "library_dependencies",
    )?;
    for dependency in dependencies {
        writer.raw(&dependency.procedure_id);
        writer.raw(&dependency.chunk_id);
        writer.raw(&dependency.source_revision);
    }
    Ok(())
}

fn write_evidence_records(
    writer: &mut Writer,
    evidence_records: &[EvidenceRecord],
) -> Result<(), Refusal> {
    writer.bounded_count(evidence_records.len(), MAX_EVIDENCE, "evidence")?;
    for evidence in evidence_records {
        writer.u32(evidence.obligation);
        write_evidence_class(writer, evidence.class);
        write_outcome(writer, evidence.outcome);
        writer.bounded_count(
            evidence.toolchain.len(),
            MAX_TOOLCHAINS,
            "evidence.toolchain",
        )?;
        for toolchain in &evidence.toolchain {
            writer.string(toolchain)?;
        }
        writer.string(&evidence.claim)?;
        writer.raw(&evidence.subject_binding);
        writer.u32(evidence.policy_revision);
        match evidence.capability {
            None => writer.byte(0),
            Some(CapabilityMarker::LiveCapability) => writer.byte(1),
            Some(CapabilityMarker::ServiceHandle) => writer.byte(2),
            Some(CapabilityMarker::Session) => writer.byte(3),
        }
    }
    Ok(())
}

fn write_promotion(writer: &mut Writer, promotion: Option<&Promotion>) -> Result<(), Refusal> {
    match promotion {
        Some(promotion) => {
            writer.byte(1);
            writer.u32(promotion.policy_revision);
            writer.bounded_count(
                promotion.obligations.len(),
                MAX_EVIDENCE,
                "promotion.obligations",
            )?;
            for obligation in &promotion.obligations {
                writer.u32(*obligation);
            }
        }
        None => writer.byte(0),
    }
    Ok(())
}

fn write_cost(writer: &mut Writer, cost: Cost) {
    writer.u32(cost.expanded_nodes);
    writer.u32(cost.emitted_source_bytes);
    writer.u32(cost.evaluation_steps);
}

fn write_retrieval_signature(
    writer: &mut Writer,
    retrieval: &RetrievalSignature,
) -> Result<(), Refusal> {
    write_types(writer, &retrieval.stack_in)?;
    write_types(writer, &retrieval.stack_out)?;
    if !retrieval.effects.is_empty() {
        return Err(Refusal::new(
            RefusalKind::Unsupported,
            super::Diagnostic {
                field: "retrieval.effects",
                message: "nonempty effects are unsupported",
            },
        ));
    }
    writer.u32(0);
    writer.byte(retrieval.status_rank);
    for key in retrieval.cost_key {
        writer.u32(key);
    }
    Ok(())
}

fn write_supersedes(writer: &mut Writer, supersedes: Option<ProcedureId>) {
    match supersedes {
        Some(id) => {
            writer.byte(1);
            writer.raw(&id);
        }
        None => writer.byte(0),
    }
}

fn write_alias(writer: &mut Writer, alias: Option<Alias>) {
    match alias {
        Some(alias) => {
            writer.byte(1);
            writer.raw(&alias.procedure_id);
            writer.raw(&alias.certificate);
        }
        None => writer.byte(0),
    }
}

fn read_state(reader: &mut Reader<'_>, record: &mut ProcedureRecord) -> Result<(), Refusal> {
    record.library_dependencies = read_library_dependencies(reader)?;
    record.evidence = read_evidence_records(reader)?;
    record.status = read_status(reader)?;
    record.promotion = read_promotion(reader)?;
    record.cost = read_cost(reader)?;
    record.retrieval = read_retrieval_signature(reader)?;
    record.supersedes = read_supersedes(reader)?;
    record.alias_of = read_alias(reader)?;
    Ok(())
}

fn read_library_dependencies(reader: &mut Reader<'_>) -> Result<Vec<LibraryDependency>, Refusal> {
    let dependency_count = reader.count("library_dependencies", MAX_LIBRARY_DEPENDENCIES)?;
    let mut dependencies = Vec::with_capacity(dependency_count);
    for _ in 0..dependency_count {
        dependencies.push(LibraryDependency {
            procedure_id: reader.raw("library_dependencies.procedure_id")?,
            chunk_id: reader.raw("library_dependencies.chunk_id")?,
            source_revision: reader.raw("library_dependencies.source_revision")?,
        });
    }
    Ok(dependencies)
}

fn read_evidence_records(reader: &mut Reader<'_>) -> Result<Vec<EvidenceRecord>, Refusal> {
    let evidence_count = reader.count("evidence", MAX_EVIDENCE)?;
    let mut evidence = Vec::with_capacity(evidence_count);
    for _ in 0..evidence_count {
        evidence.push(read_evidence_record(reader)?);
    }
    Ok(evidence)
}

fn read_evidence_record(reader: &mut Reader<'_>) -> Result<EvidenceRecord, Refusal> {
    let obligation = reader.u32("evidence.obligation")?;
    let class = read_evidence_class(reader)?;
    let outcome = read_outcome(reader)?;
    let toolchain_count = reader.count("evidence.toolchain", MAX_TOOLCHAINS)?;
    let mut toolchain = Vec::with_capacity(toolchain_count);
    for _ in 0..toolchain_count {
        toolchain.push(reader.string("evidence.toolchain.name")?);
    }
    let claim = reader.string("evidence.claim")?;
    let subject_binding = reader.raw("evidence.subject_binding")?;
    let policy_revision = reader.u32("evidence.policy_revision")?;
    if reader.byte("evidence.capability")? != 0 {
        return Err(Refusal::new(
            RefusalKind::LiveCapabilityInEvidence,
            super::Diagnostic {
                field: "evidence.capability",
                message: "capability-bearing evidence is not retained",
            },
        ));
    }
    Ok(EvidenceRecord {
        obligation,
        class,
        outcome,
        toolchain,
        claim,
        subject_binding,
        policy_revision,
        capability: None,
    })
}

fn read_promotion(reader: &mut Reader<'_>) -> Result<Option<Promotion>, Refusal> {
    if reader.bool("promotion.present")? {
        let policy_revision = reader.u32("promotion.policy_revision")?;
        let count = reader.count("promotion.obligations", MAX_EVIDENCE)?;
        let mut obligations = Vec::with_capacity(count);
        for _ in 0..count {
            obligations.push(reader.u32("promotion.obligation")?);
        }
        Ok(Some(Promotion {
            policy_revision,
            obligations,
        }))
    } else {
        Ok(None)
    }
}

fn read_cost(reader: &mut Reader<'_>) -> Result<Cost, Refusal> {
    Ok(Cost {
        expanded_nodes: reader.u32("cost.expanded_nodes")?,
        emitted_source_bytes: reader.u32("cost.emitted_source_bytes")?,
        evaluation_steps: reader.u32("cost.evaluation_steps")?,
    })
}

fn read_retrieval_signature(reader: &mut Reader<'_>) -> Result<RetrievalSignature, Refusal> {
    let stack_in = read_types(reader, "retrieval.stack_in")?;
    let stack_out = read_types(reader, "retrieval.stack_out")?;
    if reader.u32("retrieval.effects")? != 0 {
        return Err(Refusal::new(
            RefusalKind::Unsupported,
            super::Diagnostic {
                field: "retrieval.effects",
                message: "nonempty effects are unsupported",
            },
        ));
    }
    let status_rank = reader.byte("retrieval.status_rank")?;
    let cost_key = [
        reader.u32("retrieval.cost_key.0")?,
        reader.u32("retrieval.cost_key.1")?,
        reader.u32("retrieval.cost_key.2")?,
    ];
    Ok(RetrievalSignature {
        stack_in,
        stack_out,
        effects: EffSet::empty(),
        status_rank,
        cost_key,
    })
}

fn read_supersedes(reader: &mut Reader<'_>) -> Result<Option<ProcedureId>, Refusal> {
    let supersedes = if reader.bool("supersedes.present")? {
        Some(reader.raw("supersedes")?)
    } else {
        None
    };
    Ok(supersedes)
}

fn read_alias(reader: &mut Reader<'_>) -> Result<Option<Alias>, Refusal> {
    let alias = if reader.bool("alias.present")? {
        Some(Alias {
            procedure_id: reader.raw("alias.procedure_id")?,
            certificate: reader.raw("alias.certificate")?,
        })
    } else {
        None
    };
    Ok(alias)
}

fn write_record(writer: &mut Writer, record: &ProcedureRecord) -> Result<(), Refusal> {
    writer.u32(record.format);
    writer.u32(record.semantic_revision);
    write_encoder(writer, &record.encoder)?;
    writer.raw(&record.procedure_id);
    write_chunk(writer, &record.chunk)?;
    write_provenance(writer, &record.provenance)?;
    write_state(writer, record)
}

fn read_record(reader: &mut Reader<'_>) -> Result<ProcedureRecord, Refusal> {
    let format = reader.u32("format")?;
    let semantic_revision = reader.u32("semantic_revision")?;
    let encoder = Encoder {
        id: reader.string("encoder.id")?,
        encoding_revision: reader.u32("encoder.encoding_revision")?,
        experimental: reader.bool("encoder.experimental")?,
    };
    let procedure_id = reader.raw("procedure_id")?;
    let chunk = read_chunk(reader)?;
    let provenance = read_provenance(reader)?;
    let retrieval = RetrievalSignature {
        stack_in: chunk.interface.stack_in.clone(),
        stack_out: chunk.interface.stack_out.clone(),
        effects: chunk.interface.effects.clone(),
        status_rank: Status::Candidate.rank(),
        cost_key: Cost::default().key(),
    };
    let mut record = ProcedureRecord {
        format,
        semantic_revision,
        encoder,
        procedure_id,
        chunk,
        provenance,
        library_dependencies: Vec::new(),
        evidence: Vec::new(),
        status: Status::Candidate,
        promotion: None,
        cost: Cost::default(),
        retrieval,
        supersedes: None,
        alias_of: None,
    };
    read_state(reader, &mut record)?;
    Ok(record)
}

fn write_generation(writer: &mut Writer, snapshot: &Snapshot) -> Result<(), Refusal> {
    let record = &snapshot.record;
    writer.u32(record.generation);
    match record.parent_generation {
        Some(parent) => {
            writer.byte(1);
            writer.u32(parent);
        }
        None => writer.byte(0),
    }
    match record.rollback_of {
        Some(abandoned) => {
            writer.byte(1);
            writer.u32(abandoned);
        }
        None => writer.byte(0),
    }
    writer.byte(record.kind.code());
    writer.raw(&record.chunk_library_revision);
    writer.raw(&record.procedure_library_revision);
    writer.bounded_count(record.closure.len(), MAX_RECORDS, "generation.closure")?;
    for entry in &record.closure {
        writer.raw(&entry.procedure_id);
        writer.raw(&entry.state_digest);
    }
    writer.bounded_count(snapshot.records.len(), MAX_RECORDS, "generation.records")?;
    for procedure in &snapshot.records {
        write_record(writer, procedure)?;
    }
    Ok(())
}

pub(super) fn encode_parts(
    chunk_revision: [u8; chunks::REVISION_BYTES],
    generations: &[Snapshot],
    current: u32,
) -> Result<Vec<u8>, Refusal> {
    if generations.len() > MAX_GENERATIONS {
        return Err(Refusal::new(
            RefusalKind::Exhausted,
            super::Diagnostic {
                field: "generations",
                message: "generation count exceeds bound",
            },
        ));
    }
    let mut writer = Writer::new();
    writer.raw(MAGIC);
    writer.u32(RECORD_FORMAT);
    writer.raw(&chunk_revision);
    writer.u32(current);
    writer.bounded_count(generations.len(), MAX_GENERATIONS, "generations")?;
    for generation in generations {
        write_generation(&mut writer, generation)?;
    }
    if writer.bytes.len() > MAX_STORAGE_BYTES {
        return Err(Refusal::new(
            RefusalKind::Exhausted,
            super::Diagnostic {
                field: "library.bytes",
                message: "canonical library exceeds storage bound",
            },
        ));
    }
    Ok(writer.bytes)
}

fn read_generation(reader: &mut Reader<'_>) -> Result<Snapshot, Refusal> {
    let generation = reader.u32("generation")?;
    let parent_generation = if reader.bool("generation.parent.present")? {
        Some(reader.u32("generation.parent")?)
    } else {
        None
    };
    let rollback_of = if reader.bool("generation.rollback_of.present")? {
        Some(reader.u32("generation.rollback_of")?)
    } else {
        None
    };
    let kind = match reader.byte("generation.kind")? {
        0 => GenerationKind::Publish,
        1 => GenerationKind::Rollback,
        _ => {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "generation.kind",
                    message: "unknown generation kind",
                },
            ))
        }
    };
    let chunk_library_revision = reader.raw("chunk_library_revision")?;
    let procedure_library_revision = reader.raw("procedure_library_revision")?;
    let closure_count = reader.count("generation.closure", MAX_RECORDS)?;
    let mut closure = Vec::with_capacity(closure_count);
    for _ in 0..closure_count {
        closure.push(ClosureEntry {
            procedure_id: reader.raw("generation.closure.procedure_id")?,
            state_digest: reader.raw("generation.closure.state_digest")?,
        });
    }
    let record_count = reader.count("generation.records", MAX_RECORDS)?;
    let mut records = Vec::with_capacity(record_count);
    for _ in 0..record_count {
        records.push(read_record(reader)?);
    }
    Ok(Snapshot {
        record: GenerationRecord {
            generation,
            parent_generation,
            rollback_of,
            kind,
            chunk_library_revision,
            procedure_library_revision,
            closure,
        },
        records,
    })
}

impl ProcedureLibrary {
    pub fn encode(&self) -> Result<Vec<u8>, Refusal> {
        encode_parts(
            self.chunk_revision,
            &self.generations,
            self.current_generation,
        )
    }

    pub fn load(bytes: &[u8], namespace: &super::super::Session) -> Result<Self, Refusal> {
        if bytes.len() > MAX_STORAGE_BYTES {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "library.bytes",
                    message: "persisted store exceeds its bound",
                },
            ));
        }
        let mut reader = Reader::new(bytes);
        if reader.raw::<8>("magic")? != *MAGIC {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "magic",
                    message: "unsupported procedure library encoding",
                },
            ));
        }
        if reader.u32("format")? != RECORD_FORMAT {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                super::Diagnostic {
                    field: "format",
                    message: "unsupported procedure library format",
                },
            ));
        }
        let chunk_revision = reader.raw("chunk_library_revision")?;
        let current_generation = reader.u32("current_generation")?;
        let count = reader.count("generations", MAX_GENERATIONS)?;
        if count == 0 {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "generations",
                    message: "persisted store has no initial generation",
                },
            ));
        }
        let mut generations = Vec::with_capacity(count);
        for _ in 0..count {
            generations.push(read_generation(&mut reader)?);
        }
        if !reader.finished() {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "serialization.trailing_bytes",
                    message: "trailing bytes are not canonical",
                },
            ));
        }
        for (index, snapshot) in generations.iter().enumerate() {
            let generation_index = u32::try_from(index).map_err(|_| {
                Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "generation",
                        message: "generation index exceeds the wire domain",
                    },
                )
            })?;
            if snapshot.record.generation != generation_index {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field: "generation",
                        message: "generation sequence is not contiguous",
                    },
                    generation_index,
                ));
            }
            if index == 0 {
                let is_initial_generation = matches!(
                    (
                        snapshot.record.parent_generation,
                        snapshot.record.rollback_of,
                        snapshot.record.kind,
                        snapshot.records.is_empty(),
                    ),
                    (None, None, GenerationKind::Publish, true)
                );
                if !is_initial_generation {
                    return Err(Refusal::new(
                        RefusalKind::Invalid,
                        super::Diagnostic {
                            field: "generation.zero",
                            message: "initial generation must be empty and parentless",
                        },
                    ));
                }
            } else {
                let previous_index = index.checked_sub(1).ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::Internal,
                        super::Diagnostic {
                            field: "generation",
                            message: "noninitial generation has no previous index",
                        },
                    )
                })?;
                let previous = u32::try_from(previous_index).map_err(|_| {
                    Refusal::new(
                        RefusalKind::Exhausted,
                        super::Diagnostic {
                            field: "generation",
                            message: "previous generation index exceeds the wire domain",
                        },
                    )
                })?;
                let is_ancestry_valid = match snapshot.record.kind {
                    GenerationKind::Publish => {
                        snapshot.record.parent_generation == Some(previous)
                            && snapshot.record.rollback_of.is_none()
                    }
                    GenerationKind::Rollback => {
                        snapshot.record.parent_generation.is_some_and(|parent| {
                            parent <= previous
                                && usize::try_from(parent).ok().is_some_and(|parent_index| {
                                    generations.get(parent_index).is_some()
                                })
                        }) && snapshot.record.rollback_of == Some(previous)
                    }
                };
                if !is_ancestry_valid {
                    return Err(Refusal::at(
                        RefusalKind::Invalid,
                        super::Diagnostic {
                            field: "generation.parent",
                            message: "generation parent or rollback ancestry is not canonical",
                        },
                        generation_index,
                    ));
                }
            }
            if snapshot.record.chunk_library_revision != chunk_revision {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field: "chunk_library_revision",
                        message: "generation chunk revision differs from pinned #73 revision",
                    },
                    generation_index,
                ));
            }
            if snapshot.records.windows(2).any(|pair| {
                pair.first().zip(pair.get(1)).is_some_and(|(left, right)| {
                    super::retrieval::compare_records(left, right) == Ordering::Greater
                })
            }) {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field: "records.order",
                        message: "records are not in canonical retrieval order",
                    },
                    generation_index,
                ));
            }
            for record in &snapshot.records {
                ProcedureLibrary::check_dependencies(&snapshot.records, record)?;
                let actual_id = identity(
                    &record.encoder,
                    record.semantic_revision,
                    &record.chunk,
                    &record.provenance,
                    &record.library_dependencies,
                )?;
                if actual_id != record.procedure_id {
                    return Err(Refusal::new(
                        RefusalKind::Invalid,
                        super::Diagnostic {
                            field: "procedure_id",
                            message: "persisted identity differs from recomputed immutable fields",
                        },
                    ));
                }
                if record.retrieval != super::retrieval::signature(record) {
                    return Err(Refusal::new(
                        RefusalKind::Invalid,
                        super::Diagnostic {
                            field: "retrieval",
                            message: "persisted retrieval signature differs from recomputed fields",
                        },
                    ));
                }
                for evidence in &record.evidence {
                    if evidence.capability.is_some() {
                        return Err(Refusal::new(
                            RefusalKind::LiveCapabilityInEvidence,
                            super::Diagnostic {
                                field: "evidence.capability",
                                message: "capability-bearing evidence cannot be loaded",
                            },
                        ));
                    }
                }
            }
            ProcedureLibrary::verify_snapshot(snapshot)?;
            let rebuilt =
                ProcedureLibrary::rebuild_chunks(namespace, chunk_revision, &snapshot.records)?;
            if rebuilt.revision() != &snapshot.record.chunk_library_revision {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field: "chunk_library_revision",
                        message: "#73 chunk view does not reproduce the retained revision",
                    },
                    generation_index,
                ));
            }
        }
        let last_generation_index = generations.len().checked_sub(1).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Internal,
                super::Diagnostic {
                    field: "generations",
                    message: "validated generation list unexpectedly became empty",
                },
            )
        })?;
        if usize::try_from(current_generation).ok() != Some(last_generation_index) {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "current_generation",
                    message: "selected generation is not the append-only head",
                },
            ));
        }
        let current = generations
            .iter()
            .find(|snapshot| snapshot.record.generation == current_generation)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Internal,
                    super::Diagnostic {
                        field: "current_generation",
                        message: "verified generation disappeared",
                    },
                )
            })?;
        let chunk_library =
            ProcedureLibrary::rebuild_chunks(namespace, chunk_revision, &current.records)?;
        Ok(ProcedureLibrary {
            chunk_revision,
            chunk_library,
            generations,
            current_generation,
        })
    }

    pub fn identity_manifest(&self) -> Result<Vec<u8>, Refusal> {
        let mut out = BoundedJson::new(MAX_MANIFEST_BYTES);
        out.push_str("{\"schema\":\"noble-procedure-library-export/1\",\"schema_revision\":0,");
        out.push_str("\"profile\":\"pure-i64-bool/1\",\"semantic_revision\":0,");
        out.push_str("\"encoder\":{\"id\":");
        out.push_json_string(ENCODER_ID);
        out.push_str(",\"encoding_revision\":");
        out.push_u32(ENCODING_REVISION);
        out.push_str(",\"experimental\":true},\"library\":{\"generation\":");
        let current = self.current();
        out.push_u32(current.record.generation);
        out.push_str(",\"kind\":");
        out.push_json_string(generation_kind_code(current.record.kind));
        out.push_str(",\"parent_generation\":");
        match current.record.parent_generation {
            Some(parent) => out.push_u32(parent),
            None => out.push_str("null"),
        }
        out.push_str(",\"rollback_of\":");
        match current.record.rollback_of {
            Some(abandoned) => out.push_u32(abandoned),
            None => out.push_str("null"),
        }
        out.push_str(",\"chunk_library_revision\":\"");
        out.push_hex(&current.record.chunk_library_revision);
        out.push_str("\",\"procedure_library_revision\":\"");
        out.push_hex(&current.record.procedure_library_revision);
        out.push_str("\",\"closure\":[");
        for (index, entry) in current.record.closure.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push_str("{\"procedure_id\":\"");
            out.push_hex(&entry.procedure_id);
            out.push_str("\",\"state_digest\":\"");
            out.push_hex(&entry.state_digest);
            out.push_str("\"}");
        }
        out.push_str("]},\"entries\":[");
        for (index, record) in current.records.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            write_manifest_record(&mut out, record)?;
        }
        out.push_str("],\"retrieval_order\":[");
        for (index, record) in current.records.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push('"');
            out.push_hex(&record.procedure_id);
            out.push('"');
        }
        out.push_str("],\"non_claims\":[\"experimental library-local identity; not a canonical Noble hash\"]}");
        out.finish(MANIFEST_LIMIT_DIAGNOSTIC)
    }

    pub fn metrics_record(&self, metrics: &Metrics) -> Result<Vec<u8>, Refusal> {
        if metrics.arms.is_empty() {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "metrics.arms",
                    message: "transfer metrics require at least one arm",
                },
            ));
        }
        if metrics
            .arms
            .iter()
            .any(|arm| !arm.has_consistent_transfer_counts())
        {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "metrics.arms",
                    message: "transfer metrics contain inconsistent task or query counts",
                },
            ));
        }
        let aggregate = ArmAggregate::from_arms(&metrics.arms)?;
        let manifest = self.identity_manifest()?;
        let manifest_digest = digest32(DOMAIN_LIBRARY ^ 0x4d, &manifest);
        let mut out = BoundedJson::new(MAX_METRICS_BYTES);
        out.push_str("{\"schema\":\"noble-procedure-library-metrics/1\",\"schema_revision\":0,\"manifest_digest\":\"");
        out.push_hex(&manifest_digest);
        out.push_str("\",\"generation\":");
        out.push_u32(self.current_generation);
        out.push_str(",\"index\":{\"entries\":");
        out.push_usize(self.records().len());
        out.push_str(",\"bytes\":");
        out.push_usize(self.records().len().checked_mul(32).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "index.bytes",
                    message: "retrieval index byte size overflow",
                },
            )
        })?);
        out.push_str(",\"build_steps\":");
        out.push_u32(metrics.index_build_steps);
        out.push_str(",\"build_monotonic_ns\":");
        out.push_u64(metrics.index_build_monotonic_ns);
        out.push_str("},\"queries\":{\"count\":");
        out.push_u32(aggregate.retrieval_queries);
        out.push_str(",\"examined_total\":");
        out.push_u32(aggregate.retrieval_examined);
        out.push_str(",\"max_examined\":");
        out.push_u32(aggregate.max_examined.ok_or_else(|| {
            Refusal::new(
                RefusalKind::Internal,
                super::Diagnostic {
                    field: "metrics.arms",
                    message: "nonempty transfer arms have no maximum retrieval count",
                },
            )
        })?);
        out.push_str("},\"growth\":{\"entries_added\":");
        out.push_u32(aggregate.growth_entries_added);
        out.push_str(",\"bytes_added\":");
        out.push_u32(aggregate.growth_bytes_added);
        out.push_str("},\"acquisition\":[");
        for (index, arm) in metrics.arms.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push_str("{\"arm\":");
            out.push_json_string(&arm.arm);
            out.push_str(",\"seed\":");
            out.push_u64(arm.seed);
            out.push_str(",\"entries\":");
            out.push_u32(arm.acquisition_entries);
            out.push_str(",\"tasks\":");
            out.push_u32(arm.acquisition_tasks);
            out.push('}');
            out.ensure_within_limit(METRICS_LIMIT_DIAGNOSTIC)?;
        }
        out.push_str("],\"retention\":[");
        for (index, arm) in metrics.arms.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push_str("{\"arm\":");
            out.push_json_string(&arm.arm);
            out.push_str(",\"seed\":");
            out.push_u64(arm.seed);
            out.push_str(",\"surviving_generations\":");
            out.push_u32(arm.surviving_generations);
            out.push('}');
            out.ensure_within_limit(METRICS_LIMIT_DIAGNOSTIC)?;
        }
        let is_transfer_valid = metrics
            .arms
            .iter()
            .all(|arm| arm.proposer_calls == 0 && arm.teacher_calls == 0);
        out.push_str("],\"transfer_valid\":");
        out.push_str(if is_transfer_valid { "true" } else { "false" });
        out.push_str(",\"transfer\":[");
        if is_transfer_valid {
            for (index, arm) in metrics.arms.iter().enumerate() {
                if index != 0 {
                    out.push(',');
                }
                out.push_str("{\"arm\":");
                out.push_json_string(&arm.arm);
                out.push_str(",\"seed\":");
                out.push_u64(arm.seed);
                out.push_str(",\"held_out_tasks\":");
                out.push_u32(arm.held_out_tasks);
                out.push_str(",\"passed\":");
                out.push_u32(arm.transfer_passed);
                out.push('}');
                out.ensure_within_limit(METRICS_LIMIT_DIAGNOSTIC)?;
            }
        }
        out.push_str("],\"regression\":[");
        for (index, arm) in metrics.arms.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push_str("{\"arm\":");
            out.push_json_string(&arm.arm);
            out.push_str(",\"seed\":");
            out.push_u64(arm.seed);
            out.push_str(",\"earlier_tasks\":");
            out.push_u32(arm.earlier_tasks);
            out.push_str(",\"failed\":");
            out.push_u32(arm.regression_failed);
            out.push('}');
            out.ensure_within_limit(METRICS_LIMIT_DIAGNOSTIC)?;
        }
        out.push_str("],\"restart\":{\"identity_match\":");
        out.push_str(
            if metrics.arms.iter().all(|arm| arm.restart_identity_match) {
                "true"
            } else {
                "false"
            },
        );
        out.push_str("},\"arms\":[");
        for (index, arm) in metrics.arms.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push_str("{\"arm\":");
            out.push_json_string(&arm.arm);
            out.push_str(",\"seed\":");
            out.push_u64(arm.seed);
            out.push_str(",\"growth_entries_added\":");
            out.push_u32(arm.growth_entries_added);
            out.push_str(",\"growth_bytes_added\":");
            out.push_u32(arm.growth_bytes_added);
            out.push_str(",\"retrieval_queries\":");
            out.push_u32(arm.retrieval_queries);
            out.push_str(",\"retrieval_examined\":");
            out.push_u32(arm.retrieval_examined);
            out.push_str(",\"retrieval_overhead_ns\":");
            out.push_u64(arm.retrieval_overhead_ns);
            out.push_str(",\"restart_identity_match\":");
            out.push_bool(arm.restart_identity_match);
            out.push_str(",\"break_even_invocations\":");
            match arm.break_even_invocations {
                Some(value) => out.push_u32(value),
                None => out.push_str("null"),
            }
            out.push_str(",\"break_even_reason\":");
            out.push_json_string(&arm.break_even_reason);
            out.push('}');
            out.ensure_within_limit(METRICS_LIMIT_DIAGNOSTIC)?;
        }
        out.push_str("],\"proposer_calls\":");
        out.push_u32(aggregate.proposer_calls);
        out.push_str(",\"teacher_calls\":");
        out.push_u32(aggregate.teacher_calls);
        out.push_str(",\"claim_separation\":{\"model_synthesis\":");
        out.push_json_string(&metrics.model_synthesis);
        out.push_str(",\"executed_mechanics\":");
        out.push_json_string(&metrics.executed_mechanics);
        out.push_str(",\"correspondence\":");
        out.push_json_string(&metrics.correspondence);
        out.push_str(",\"assumptions\":");
        out.push_json_string(&metrics.assumptions);
        out.push_str("}}");
        out.finish(METRICS_LIMIT_DIAGNOSTIC)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metrics {
    pub index_build_steps: u32,
    pub index_build_monotonic_ns: u64,
    pub arms: Vec<ArmMetrics>,
    pub model_synthesis: String,
    pub executed_mechanics: String,
    pub correspondence: String,
    pub assumptions: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmMetrics {
    pub arm: String,
    pub seed: u64,
    pub acquisition_entries: u32,
    pub acquisition_tasks: u32,
    pub surviving_generations: u32,
    pub held_out_tasks: u32,
    pub transfer_passed: u32,
    pub earlier_tasks: u32,
    pub regression_failed: u32,
    pub growth_entries_added: u32,
    pub growth_bytes_added: u32,
    pub retrieval_queries: u32,
    pub retrieval_examined: u32,
    pub max_examined: u32,
    pub retrieval_overhead_ns: u64,
    pub restart_identity_match: bool,
    pub proposer_calls: u32,
    pub teacher_calls: u32,
    pub break_even_invocations: Option<u32>,
    pub break_even_reason: String,
}

impl ArmMetrics {
    fn has_consistent_transfer_counts(&self) -> bool {
        self.transfer_passed <= self.held_out_tasks
            && self.regression_failed <= self.earlier_tasks
            && self.max_examined <= self.retrieval_examined
    }
}

struct ArmAggregate {
    retrieval_queries: u32,
    retrieval_examined: u32,
    max_examined: Option<u32>,
    growth_entries_added: u32,
    growth_bytes_added: u32,
    proposer_calls: u32,
    teacher_calls: u32,
}

impl ArmAggregate {
    fn from_arms(arms: &[ArmMetrics]) -> Result<Self, Refusal> {
        let mut aggregate = Self {
            retrieval_queries: 0,
            retrieval_examined: 0,
            max_examined: None,
            growth_entries_added: 0,
            growth_bytes_added: 0,
            proposer_calls: 0,
            teacher_calls: 0,
        };
        for arm in arms {
            aggregate.retrieval_queries = aggregate
                .retrieval_queries
                .checked_add(arm.retrieval_queries)
                .ok_or_else(|| metric_sum_overflow("metrics.retrieval_queries"))?;
            aggregate.retrieval_examined = aggregate
                .retrieval_examined
                .checked_add(arm.retrieval_examined)
                .ok_or_else(|| metric_sum_overflow("metrics.retrieval_examined"))?;
            aggregate.max_examined = Some(
                aggregate
                    .max_examined
                    .map_or(arm.max_examined, |current_maximum| {
                        current_maximum.max(arm.max_examined)
                    }),
            );
            aggregate.growth_entries_added = aggregate
                .growth_entries_added
                .checked_add(arm.growth_entries_added)
                .ok_or_else(|| metric_sum_overflow("metrics.growth_entries_added"))?;
            aggregate.growth_bytes_added = aggregate
                .growth_bytes_added
                .checked_add(arm.growth_bytes_added)
                .ok_or_else(|| metric_sum_overflow("metrics.growth_bytes_added"))?;
            aggregate.proposer_calls = aggregate
                .proposer_calls
                .checked_add(arm.proposer_calls)
                .ok_or_else(|| metric_sum_overflow("metrics.proposer_calls"))?;
            aggregate.teacher_calls = aggregate
                .teacher_calls
                .checked_add(arm.teacher_calls)
                .ok_or_else(|| metric_sum_overflow("metrics.teacher_calls"))?;
        }
        Ok(aggregate)
    }
}

fn metric_sum_overflow(field: &'static str) -> Refusal {
    Refusal::new(
        RefusalKind::Exhausted,
        super::Diagnostic {
            field,
            message: "transfer metric total exceeds u32",
        },
    )
}

fn write_manifest_record(out: &mut BoundedJson, record: &ProcedureRecord) -> Result<(), Refusal> {
    write_manifest_identity(out, record)?;
    write_manifest_provenance(out, record);
    write_manifest_dependencies(out, record);
    write_manifest_promotion(out, record);
    write_manifest_evidence(out, record);
    write_manifest_cost_and_links(out, record);
    out.push('}');
    Ok(())
}

fn write_manifest_identity(out: &mut BoundedJson, record: &ProcedureRecord) -> Result<(), Refusal> {
    out.push_str("{\"procedure_id\":\"");
    out.push_hex(&record.procedure_id);
    out.push_str("\",\"identity_digest\":\"");
    out.push_hex(&record.procedure_id);
    out.push_str("\",\"chunk_id\":\"");
    out.push_hex(&record.chunk.chunk_id);
    out.push_str("\",\"source_revision\":\"");
    out.push_hex(&record.chunk.source_revision);
    out.push_str("\",\"chunk_format\":");
    out.push_u32(record.chunk.format);
    out.push_str(",\"chunk_semantic_revision\":");
    out.push_u32(record.chunk.semantic_revision);
    out.push_str(",\"interface\":{\"stack_in\":[");
    write_type_tags(out, &record.chunk.interface.stack_in)?;
    out.push_str("],\"stack_out\":[");
    write_type_tags(out, &record.chunk.interface.stack_out)?;
    out.push_str("],\"effects\":[]},\"captures\":");
    out.push_usize(record.chunk.captures.len());
    out.push_str(",\"instantiations\":");
    out.push_usize(record.chunk.instantiations.len());
    Ok(())
}

fn write_manifest_provenance(out: &mut BoundedJson, record: &ProcedureRecord) {
    out.push_str(",\"provenance\":{\"producer\":");
    out.push_json_string(producer_code(record.provenance.producer));
    out.push_str(",\"producer_tool_revision\":");
    out.push_u32(record.provenance.producer_tool_revision);
    if let Some(task_id) = record.provenance.task_id {
        out.push_str(",\"task_id\":\"");
        out.push_hex(&task_id);
        out.push('"');
    } else {
        out.push_str(",\"task_id\":null");
    }
    if let Some(revision) = record.provenance.grammar_revision {
        out.push_str(",\"grammar_revision\":");
        out.push_u32(revision);
    } else {
        out.push_str(",\"grammar_revision\":null");
    }
}

fn write_manifest_dependencies(out: &mut BoundedJson, record: &ProcedureRecord) {
    out.push_str("},\"dependencies\":[");
    for (index, dependency) in record.library_dependencies.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push_str("{\"procedure_id\":\"");
        out.push_hex(&dependency.procedure_id);
        out.push_str("\",\"chunk_id\":\"");
        out.push_hex(&dependency.chunk_id);
        out.push_str("\",\"source_revision\":\"");
        out.push_hex(&dependency.source_revision);
        out.push_str("\"}");
    }
    out.push_str("],\"status\":");
    out.push_json_string(record.status.code());
}

fn write_manifest_promotion(out: &mut BoundedJson, record: &ProcedureRecord) {
    out.push_str(",\"promotion\":");
    if let Some(promotion) = &record.promotion {
        out.push_str("{\"policy_revision\":");
        out.push_u32(promotion.policy_revision);
        out.push_str(",\"obligations\":[");
        for (index, obligation) in promotion.obligations.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push_u32(*obligation);
        }
        out.push_str("]}");
    } else {
        out.push_str("null");
    }
}

fn write_manifest_evidence(out: &mut BoundedJson, record: &ProcedureRecord) {
    out.push_str(",\"evidence\":[");
    for (index, evidence) in record.evidence.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push_str("{\"obligation\":");
        out.push_u32(evidence.obligation);
        out.push_str(",\"class\":");
        out.push_json_string(evidence.class.code());
        out.push_str(",\"outcome\":");
        out.push_json_string(evidence.outcome.code());
        out.push_str(",\"toolchain\":[");
        for (tool_index, toolchain) in evidence.toolchain.iter().enumerate() {
            if tool_index != 0 {
                out.push(',');
            }
            out.push_json_string(toolchain);
        }
        out.push_str("],\"claim\":");
        out.push_json_string(&evidence.claim);
        out.push_str(",\"subject_binding\":\"");
        out.push_hex(&evidence.subject_binding);
        out.push_str("\",\"policy_revision\":");
        out.push_u32(evidence.policy_revision);
        out.push('}');
    }
    out.push(']');
}

fn write_manifest_cost_and_links(out: &mut BoundedJson, record: &ProcedureRecord) {
    out.push_str(",\"cost\":{\"expanded_nodes\":");
    out.push_u32(record.cost.expanded_nodes);
    out.push_str(",\"emitted_source_bytes\":");
    out.push_u32(record.cost.emitted_source_bytes);
    out.push_str(",\"evaluation_steps\":");
    out.push_u32(record.cost.evaluation_steps);
    out.push('}');
    out.push_str(",\"supersedes\":");
    if let Some(id) = record.supersedes {
        out.push('"');
        out.push_hex(&id);
        out.push('"');
    } else {
        out.push_str("null");
    }
    out.push_str(",\"alias_of\":");
    if let Some(alias) = record.alias_of {
        out.push_str("{\"procedure_id\":\"");
        out.push_hex(&alias.procedure_id);
        out.push_str("\",\"certificate\":\"");
        out.push_hex(&alias.certificate);
        out.push_str("\"}");
    } else {
        out.push_str("null");
    }
}

fn generation_kind_code(kind: GenerationKind) -> &'static str {
    match kind {
        GenerationKind::Publish => "publish",
        GenerationKind::Rollback => "rollback",
    }
}

fn write_type_tags(out: &mut BoundedJson, types: &[Ty]) -> Result<(), Refusal> {
    for (index, ty) in types.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        let type_tag = if ty == &Ty::I64 {
            "I64"
        } else if ty == &Ty::Bool {
            "Bool"
        } else {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                super::Diagnostic {
                    field: "interface.type",
                    message: "manifest type is outside the procedure profile",
                },
            ));
        };
        out.push_json_string(type_tag);
    }
    Ok(())
}

fn producer_code(producer: Producer) -> &'static str {
    match producer {
        Producer::Chunk => "chunk",
        Producer::Synthesis => "synthesis",
        Producer::Composition => "composition",
    }
}
