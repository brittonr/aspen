//! Conversions between `preserves::IOValue` and the canonical `PackedValue` tree.
//!
//! Both directions keep an explicit work stack, so conversion depth is not bounded by the thread
//! stack. Annotations are dropped: canonical bytes are computed without them, exactly as
//! `preserves::PackedWriter` does when it is told not to write annotations.

type Packed = molten_preserves_core::model::PackedValue;

/// A compound whose children are being converted.
#[derive(Clone, Copy)]
enum Compound {
    Record,
    Sequence,
    Set,
    Dictionary,
    Embedded,
}

enum Step<T> {
    Enter(T),
    Exit { compound: Compound, children: usize },
}

/// Converted values in completion order plus the pending work.
struct Assembly<S, T> {
    steps: Vec<Step<S>>,
    done: Vec<T>,
}

impl<S, T> Assembly<S, T> {
    fn new(root: S) -> Self {
        Self {
            steps: vec![Step::Enter(root)],
            done: Vec::new(),
        }
    }

    /// Queues a compound: its children convert in order, then `Exit` assembles them.
    fn open(&mut self, compound: Compound, children: Vec<S>) {
        self.steps.push(Step::Exit {
            compound,
            children: children.len(),
        });
        self.steps.extend(children.into_iter().rev().map(Step::Enter));
    }

    fn finish(&mut self, value: T) {
        self.done.push(value);
    }

    /// The converted children of the compound now exiting.
    fn take_children(&mut self, count: usize) -> super::Result<Vec<T>> {
        let start = self
            .done
            .len()
            .checked_sub(count)
            .ok_or_else(|| super::MoltenError::invalid_harness("Preserves value conversion lost a child"))?;
        Ok(self.done.split_off(start))
    }

    fn into_root(mut self) -> super::Result<T> {
        match (self.done.pop(), self.done.is_empty()) {
            (Some(root), true) => Ok(root),
            (Some(_) | None, _) => Err(super::MoltenError::invalid_harness("Preserves value conversion lost its root")),
        }
    }
}

/// The canonical model of `value`, without annotations.
pub(super) fn packed_from_iovalue(value: &super::IoValue) -> super::Result<Packed> {
    let mut assembly = Assembly::new(value.clone());
    while let Some(step) = assembly.steps.pop() {
        match step {
            Step::Enter(value) => enter_iovalue(&mut assembly, &value)?,
            Step::Exit { compound, children } => {
                let children = assembly.take_children(children)?;
                assembly.finish(packed_compound(compound, children)?);
            }
        }
    }
    assembly.into_root()
}

fn enter_iovalue(assembly: &mut Assembly<super::IoValue, Packed>, value: &super::IoValue) -> super::Result<()> {
    let owned = |child: super::Value<super::IoValue>| super::IoValue::from(child);
    match value.value_class() {
        preserves::ValueClass::Atomic(atom) => assembly.finish(packed_atom(value, atom)?),
        preserves::ValueClass::Compound(preserves::CompoundClass::Record) => {
            let children = std::iter::once(owned(value.label())).chain(value.iter().map(owned)).collect();
            assembly.open(Compound::Record, children);
        }
        preserves::ValueClass::Compound(preserves::CompoundClass::Sequence) => {
            assembly.open(Compound::Sequence, value.iter().map(owned).collect());
        }
        preserves::ValueClass::Compound(preserves::CompoundClass::Set) => {
            assembly.open(Compound::Set, value.iter().map(owned).collect());
        }
        preserves::ValueClass::Compound(preserves::CompoundClass::Dictionary) => {
            let children = value.entries().flat_map(|(key, item)| [owned(key), owned(item)]).collect();
            assembly.open(Compound::Dictionary, children);
        }
        preserves::ValueClass::Embedded => {
            let inner = value.as_embedded().map(std::borrow::Cow::into_owned).ok_or_else(class_mismatch)?;
            assembly.open(Compound::Embedded, vec![inner]);
        }
    }
    Ok(())
}

fn class_mismatch() -> super::MoltenError {
    super::MoltenError::invalid_harness("Preserves value class and accessor disagree")
}

fn packed_atom(value: &super::IoValue, atom: preserves::AtomClass) -> super::Result<Packed> {
    let packed = match atom {
        preserves::AtomClass::Boolean => value.as_boolean().map(Packed::Boolean),
        preserves::AtomClass::Double => value.as_double().map(Packed::from),
        preserves::AtomClass::SignedInteger => {
            value.as_signed_integer().map(|integer| packed_integer(&integer)).transpose()?
        }
        preserves::AtomClass::String => value.as_string().map(|text| Packed::String(text.into_owned())),
        preserves::AtomClass::ByteString => value.as_bytestring().map(|bytes| Packed::ByteString(bytes.into_owned())),
        preserves::AtomClass::Symbol => value.as_symbol().map(|name| Packed::Symbol(name.into_owned())),
    };
    packed.ok_or_else(class_mismatch)
}

fn packed_integer(integer: &preserves::SignedInteger) -> super::Result<Packed> {
    match integer.repr() {
        preserves::signed_integer::SignedIntegerRepr::I128(value) => Ok(Packed::from(*value)),
        preserves::signed_integer::SignedIntegerRepr::U128(value) => Ok(Packed::from(*value)),
        preserves::signed_integer::SignedIntegerRepr::Big(value) => {
            molten_preserves_core::model::signed::PackedInteger::from_be_bytes(&value.to_signed_bytes_be())
                .map(Packed::SignedInteger)
                .map_err(|_| super::MoltenError::Preserves("big integer bytes are not minimal".to_string()))
        }
    }
}

fn packed_compound(compound: Compound, children: Vec<Packed>) -> super::Result<Packed> {
    let mut children = children.into_iter();
    Ok(match compound {
        Compound::Record => Packed::record(children.next().ok_or_else(class_mismatch)?, children.collect()),
        Compound::Sequence => Packed::Sequence(children.collect()),
        Compound::Set => Packed::Set(children.collect()),
        Compound::Dictionary => Packed::Dictionary(pairs(children).collect()),
        Compound::Embedded => Packed::embedded(children.next().ok_or_else(class_mismatch)?),
    })
}

fn pairs<T>(mut items: impl Iterator<Item = T>) -> impl Iterator<Item = (T, T)> {
    std::iter::from_fn(move || Some((items.next()?, items.next()?)))
}

/// The `IOValue` of a canonical model value.
pub(super) fn iovalue_from_packed(value: &Packed) -> super::Result<super::IoValue> {
    let mut assembly = Assembly::new(value);
    while let Some(step) = assembly.steps.pop() {
        match step {
            Step::Enter(value) => enter_packed(&mut assembly, value)?,
            Step::Exit { compound, children } => {
                let children = assembly.take_children(children)?;
                assembly.finish(iovalue_compound(compound, children)?);
            }
        }
    }
    assembly.into_root()
}

fn enter_packed<'a>(assembly: &mut Assembly<&'a Packed, super::IoValue>, value: &'a Packed) -> super::Result<()> {
    match value {
        Packed::Boolean(flag) => assembly.finish(super::IoValue::new(*flag)),
        Packed::Double(double) => assembly.finish(super::IoValue::new(double.to_f64())),
        Packed::SignedInteger(integer) => assembly.finish(iovalue_integer(integer)?),
        Packed::String(text) => assembly.finish(super::IoValue::new(text.clone())),
        Packed::ByteString(bytes) => assembly.finish(super::IoValue::bytes(bytes.clone())),
        Packed::Symbol(name) => assembly.finish(super::IoValue::symbol(name.clone())),
        Packed::Record(record) => {
            let children = std::iter::once(&*record.label).chain(record.fields.iter()).collect();
            assembly.open(Compound::Record, children);
        }
        Packed::Sequence(items) => assembly.open(Compound::Sequence, items.iter().collect()),
        Packed::Set(set) => assembly.open(Compound::Set, set.iter().collect()),
        Packed::Dictionary(dictionary) => {
            assembly.open(Compound::Dictionary, dictionary.iter().flat_map(|(key, item)| [key, item]).collect());
        }
        Packed::Embedded(inner) => assembly.open(Compound::Embedded, vec![&**inner]),
    }
    Ok(())
}

/// Integers within `i128`/`u128` convert directly; wider ones go through the packed reader, the
/// only `preserves` constructor for big integers this crate can reach without `num-bigint`.
fn iovalue_integer(integer: &molten_preserves_core::model::signed::PackedInteger) -> super::Result<super::IoValue> {
    if let Some(value) = integer.to_i128() {
        return Ok(super::IoValue::new(preserves::SignedInteger::from(value)));
    }
    if let Some(value) = integer.to_u128() {
        return Ok(super::IoValue::new(preserves::SignedInteger::from(value)));
    }
    let bytes = molten_preserves_core::writer::encode_canonical(&Packed::SignedInteger(integer.clone()));
    preserves::read_iovalue_packed(&bytes, false).map_err(|error| super::MoltenError::Preserves(error.to_string()))
}

fn iovalue_compound(compound: Compound, children: Vec<super::IoValue>) -> super::Result<super::IoValue> {
    let mut children = children.into_iter();
    Ok(match compound {
        Compound::Record => super::IoValue::record(children.next().ok_or_else(class_mismatch)?, children.collect()),
        Compound::Sequence => super::IoValue::new(children.collect::<Vec<_>>()),
        Compound::Set => super::IoValue::new(children.collect::<preserves::Set<super::IoValue>>()),
        Compound::Dictionary => super::IoValue::new(pairs(children).collect::<preserves::Map<_, _>>()),
        Compound::Embedded => super::IoValue::embedded(children.next().ok_or_else(class_mismatch)?),
    })
}
