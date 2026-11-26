//! The structural scan over `IOValue`, driven by the kernel's bounded walker.

/// One `IOValue` node for `molten_preserves_core::inspect`.
struct IoNode(super::IoValue);

fn io_node(child: super::Value<super::IoValue>) -> IoNode {
    IoNode(super::IoValue::from(child))
}

fn indexed(
    children: impl Iterator<Item = super::Value<super::IoValue>>,
    segment: fn(u64) -> molten_preserves_core::inspect::PathSegment,
) -> impl Iterator<Item = (molten_preserves_core::inspect::PathSegment, IoNode)> {
    children.zip(0_u64..).map(move |(child, index)| (segment(index), io_node(child)))
}

impl molten_preserves_core::inspect::TraversalNode for IoNode {
    fn shape(&self) -> molten_preserves_core::inspect::NodeShape<'_> {
        let value = &self.0;
        if value.is_record() {
            let label = value.label().as_symbol().map(|name| std::borrow::Cow::Owned(name.into_owned()));
            return molten_preserves_core::inspect::NodeShape::Record { symbol_label: label };
        }
        if let Some(name) = value.as_symbol() {
            return molten_preserves_core::inspect::NodeShape::Symbol(name);
        }
        if let Some(text) = value.as_string() {
            return molten_preserves_core::inspect::NodeShape::String(text);
        }
        value.as_bytestring().map_or(
            molten_preserves_core::inspect::NodeShape::Other,
            molten_preserves_core::inspect::NodeShape::ByteString,
        )
    }

    /// Record labels and fields, sequence and set items, dictionary keys and values, and the
    /// payload of an `Embedded` value, within the budget. Embedded payloads are children like any
    /// other, so a wrapped marker or content ref cannot evade the scan.
    // r[impl molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
    fn children(&self, budget: u64) -> Vec<(molten_preserves_core::inspect::PathSegment, Self)> {
        let value = &self.0;
        if value.is_record() {
            let label = std::iter::once((molten_preserves_core::inspect::PathSegment::Label, io_node(value.label())));
            let fields = indexed(value.iter(), molten_preserves_core::inspect::PathSegment::Field);
            return molten_preserves_core::inspect::within_budget(label.chain(fields), budget);
        }
        if value.is_sequence() || value.is_set() {
            let items = indexed(value.iter(), molten_preserves_core::inspect::PathSegment::Item);
            return molten_preserves_core::inspect::within_budget(items, budget);
        }
        if value.is_dictionary() {
            let entries = value.entries().zip(0_u64..).flat_map(|((key, item), index)| {
                [
                    (molten_preserves_core::inspect::PathSegment::EntryKey(index), io_node(key)),
                    (molten_preserves_core::inspect::PathSegment::EntryValue(index), io_node(item)),
                ]
            });
            return molten_preserves_core::inspect::within_budget(entries, budget);
        }
        let embedded = value
            .as_embedded()
            .map(|inner| (molten_preserves_core::inspect::PathSegment::Embedded, IoNode(inner.into_owned())));
        molten_preserves_core::inspect::within_budget(embedded.into_iter(), budget)
    }
}

fn shell_match(found: molten_preserves_core::inspect::TokenMatch) -> super::StructuralMatch {
    super::StructuralMatch {
        kind: found.kind,
        token: found.token,
        path: found.path.iter().map(ToString::to_string).collect(),
    }
}

fn shell_result(
    found: Result<Option<molten_preserves_core::inspect::TokenMatch>, molten_preserves_core::inspect::TraversalError>,
) -> super::Result<Option<super::StructuralMatch>> {
    found
        .map(|found| found.map(shell_match))
        .map_err(|error| super::MoltenError::invalid_harness(error.to_string()))
}

/// The first token in preorder that `predicate` selects, within the shell's scope and limits.
pub(super) fn find_match<F>(
    value: &super::IoValue,
    scope: super::StructuralInspectionScope,
    limits: super::StructuralInspectionLimits,
    predicate: F,
) -> super::Result<Option<super::StructuralMatch>>
where
    F: FnMut(super::StructuralTokenKind, &str) -> bool,
{
    let scope = molten_preserves_core::inspect::TokenScope {
        record_labels: scope.record_labels,
        symbols: scope.symbols,
        strings: scope.strings,
        byte_strings: scope.byte_strings,
        content_refs: scope.content_refs,
    };
    let limits = molten_preserves_core::inspect::TraversalLimits {
        max_nodes: crate::bounded::u64_from_usize(limits.max_nodes, "structural scan node limit")?,
        max_depth: crate::bounded::u64_from_usize(limits.max_depth, "structural scan depth limit")?,
    };
    shell_result(molten_preserves_core::inspect::find_match(IoNode(value.clone()), scope, limits, predicate))
}

/// The first record label or symbol in `names`, under the kernel's default limits.
pub(super) fn find_named(value: &super::IoValue, names: &[&str]) -> super::Result<Option<super::StructuralMatch>> {
    shell_result(molten_preserves_core::inspect::catalog::find_named(IoNode(value.clone()), names))
}

/// The first string equal to `target`, under the kernel's default limits.
pub(super) fn find_content_ref(
    value: &super::IoValue,
    target: &super::ContentRef,
) -> super::Result<Option<super::StructuralMatch>> {
    let target = molten_preserves_core::identity::ContentRef::parse(target.as_str())
        .map_err(|error| super::identity_bridge::ref_failure(target.as_str(), error))?;
    shell_result(molten_preserves_core::inspect::catalog::find_content_ref(IoNode(value.clone()), &target))
}
