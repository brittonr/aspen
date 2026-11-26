//! `TraversalNode` for the canonical value tree.

impl super::TraversalNode for &crate::model::PackedValue {
    fn shape(&self) -> super::NodeShape<'_> {
        match self {
            crate::model::PackedValue::Record(record) => super::NodeShape::Record {
                symbol_label: record.label.as_symbol().map(alloc::borrow::Cow::Borrowed),
            },
            crate::model::PackedValue::Symbol(name) => super::NodeShape::Symbol(alloc::borrow::Cow::Borrowed(name)),
            crate::model::PackedValue::String(text) => super::NodeShape::String(alloc::borrow::Cow::Borrowed(text)),
            crate::model::PackedValue::ByteString(bytes) => {
                super::NodeShape::ByteString(alloc::borrow::Cow::Borrowed(bytes))
            }
            crate::model::PackedValue::Boolean(_)
            | crate::model::PackedValue::Double(_)
            | crate::model::PackedValue::SignedInteger(_)
            | crate::model::PackedValue::Sequence(_)
            | crate::model::PackedValue::Set(_)
            | crate::model::PackedValue::Dictionary(_)
            | crate::model::PackedValue::Embedded(_) => super::NodeShape::Other,
        }
    }

    fn children(&self, budget: u64) -> alloc::vec::Vec<(super::PathSegment, Self)> {
        match *self {
            crate::model::PackedValue::Record(record) => super::within_budget(
                core::iter::once((super::PathSegment::Label, &*record.label))
                    .chain(indexed(record.fields.iter(), super::PathSegment::Field)),
                budget,
            ),
            crate::model::PackedValue::Sequence(items) => {
                super::within_budget(indexed(items.iter(), super::PathSegment::Item), budget)
            }
            crate::model::PackedValue::Set(set) => {
                super::within_budget(indexed(set.iter(), super::PathSegment::Item), budget)
            }
            crate::model::PackedValue::Dictionary(dictionary) => super::within_budget(
                dictionary.iter().zip(0_u64..).flat_map(|((key, value), index)| {
                    [
                        (super::PathSegment::EntryKey(index), key),
                        (super::PathSegment::EntryValue(index), value),
                    ]
                }),
                budget,
            ),
            crate::model::PackedValue::Embedded(inner) => {
                super::within_budget(core::iter::once((super::PathSegment::Embedded, &**inner)), budget)
            }
            crate::model::PackedValue::Boolean(_)
            | crate::model::PackedValue::Double(_)
            | crate::model::PackedValue::SignedInteger(_)
            | crate::model::PackedValue::String(_)
            | crate::model::PackedValue::ByteString(_)
            | crate::model::PackedValue::Symbol(_) => alloc::vec::Vec::new(),
        }
    }
}

fn indexed<'a>(
    values: impl Iterator<Item = &'a crate::model::PackedValue>,
    segment: fn(u64) -> super::PathSegment,
) -> impl Iterator<Item = (super::PathSegment, &'a crate::model::PackedValue)> {
    values.zip(0_u64..).map(move |(value, index)| (segment(index), value))
}
