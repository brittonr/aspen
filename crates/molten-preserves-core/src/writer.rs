//! The canonical packed encoder.
//!
//! Output is byte-identical to `preserves::PackedWriter` with annotations disabled: minimal
//! varint lengths, minimal two's-complement integers, binary64 floats by exact bit pattern, and
//! set elements and dictionary entries sorted by their canonical bytes. The walk keeps an explicit
//! stack instead of recursing, so encoding depth is bounded only by the value itself.

/// Initial output reservation; most Aspen records encode to a few hundred bytes.
const INITIAL_CAPACITY: usize = 256;

enum Pending<'a> {
    Value(&'a crate::model::PackedValue),
    /// Bytes already in canonical form: a set element or a dictionary key.
    Encoded(&'a [u8]),
    End,
}

struct Encoder<'a> {
    output: alloc::vec::Vec<u8>,
    pending: alloc::vec::Vec<Pending<'a>>,
}

/// Encodes `value` in canonical packed form.
pub fn encode_canonical(value: &crate::model::PackedValue) -> alloc::vec::Vec<u8> {
    let mut encoder = Encoder {
        output: alloc::vec::Vec::with_capacity(INITIAL_CAPACITY),
        pending: alloc::vec![Pending::Value(value)],
    };
    while let Some(step) = encoder.pending.pop() {
        encoder.write(step);
    }
    encoder.output
}

/// The low seven bits of `length`.
fn low_seven_bits(length: usize) -> u8 {
    length.to_le_bytes()[0] & 0x7f
}

/// Maximum varint width of a 64-bit length.
const MAX_VARINT_BYTES: usize = 10;

/// The minimal little-endian base-128 varint of `length`, in a stack buffer, and its width.
fn length_varint(mut length: usize) -> ([u8; MAX_VARINT_BYTES], usize) {
    let mut bytes = [0; MAX_VARINT_BYTES];
    let mut width = 0;
    while length >= 0x80 && width + 1 < MAX_VARINT_BYTES {
        bytes[width] = low_seven_bits(length) | 0x80;
        length >>= 7;
        width += 1;
    }
    bytes[width] = low_seven_bits(length);
    (bytes, width + 1)
}

/// Appends one length-prefixed atom with a single reservation.
pub(crate) fn write_atom(output: &mut impl AtomSink, tag: u8, payload: &[u8]) {
    let (varint, width) = length_varint(payload.len());
    output.append_atom(tag, &varint[..width], payload);
}

/// Byte outputs that accept one complete atom.
pub(crate) trait AtomSink {
    fn append_atom(&mut self, tag: u8, length: &[u8], payload: &[u8]);
}

impl AtomSink for alloc::vec::Vec<u8> {
    fn append_atom(&mut self, tag: u8, length: &[u8], payload: &[u8]) {
        self.reserve(1 + length.len() + payload.len());
        self.push(tag);
        self.extend_from_slice(length);
        self.extend_from_slice(payload);
    }
}

impl<'a> Encoder<'a> {
    fn write(&mut self, step: Pending<'a>) {
        match step {
            Pending::Value(value) => self.write_value(value),
            Pending::Encoded(bytes) => self.output.extend_from_slice(bytes),
            Pending::End => self.output.push(crate::tags::END),
        }
    }

    fn write_atom(&mut self, tag: u8, payload: &[u8]) {
        write_atom(&mut self.output, tag, payload);
    }

    /// Opens a compound: its tag now, its end marker after the children queued by the caller.
    fn open(&mut self, tag: u8) {
        self.output.push(tag);
        self.pending.push(Pending::End);
    }

    fn write_value(&mut self, value: &'a crate::model::PackedValue) {
        match value {
            crate::model::PackedValue::Boolean(false) => self.output.push(crate::tags::FALSE),
            crate::model::PackedValue::Boolean(true) => self.output.push(crate::tags::TRUE),
            crate::model::PackedValue::Double(double) => {
                self.write_atom(crate::tags::IEEE754, &double.to_bits().to_be_bytes());
            }
            crate::model::PackedValue::SignedInteger(integer) => {
                self.write_atom(crate::tags::SIGNED_INTEGER, integer.as_be_bytes());
            }
            crate::model::PackedValue::String(text) => self.write_atom(crate::tags::STRING, text.as_bytes()),
            crate::model::PackedValue::ByteString(bytes) => self.write_atom(crate::tags::BYTE_STRING, bytes),
            crate::model::PackedValue::Symbol(name) => self.write_atom(crate::tags::SYMBOL, name.as_bytes()),
            crate::model::PackedValue::Record(record) => {
                self.open(crate::tags::RECORD);
                self.pending.extend(record.fields.iter().rev().map(Pending::Value));
                self.pending.push(Pending::Value(&record.label));
            }
            crate::model::PackedValue::Sequence(items) => {
                self.open(crate::tags::SEQUENCE);
                self.pending.extend(items.iter().rev().map(Pending::Value));
            }
            crate::model::PackedValue::Set(set) => {
                self.open(crate::tags::SET);
                self.pending.extend(set.encoded().rev().map(Pending::Encoded));
            }
            crate::model::PackedValue::Dictionary(dictionary) => {
                self.open(crate::tags::DICTIONARY);
                self.pending.extend(
                    dictionary
                        .encoded_entries()
                        .rev()
                        .flat_map(|(key, value)| [Pending::Value(value), Pending::Encoded(key)]),
                );
            }
            crate::model::PackedValue::Embedded(inner) => {
                self.output.push(crate::tags::EMBEDDED);
                self.pending.push(Pending::Value(inner));
            }
        }
    }
}
