//! Ownership-preserving transitions of a bounded type traversal.

/// Pending nodes and charged nodes travel together through every step.
pub(super) struct State<'a> {
    /// An interface type mentioned by a program is not its owned payload.
    pub(super) pending: alloc::vec::Vec<(&'a crate::types::Ty, bool)>,
    pub(super) visited: u32,
}

impl<'a> State<'a> {
    pub(super) fn for_descriptor(shape: &'a crate::types::NominalShape) -> Option<Self> {
        Self {
            pending: alloc::vec::Vec::with_capacity(4),
            visited: 0,
        }
        .with_descriptor(shape, false)
    }

    pub(super) fn with_type(mut self, ty: &'a crate::types::Ty, payload: bool) -> Self {
        self.pending.push((ty, payload));
        self
    }

    pub(super) fn with_pair(
        mut self,
        left: &'a crate::types::Ty,
        right: &'a crate::types::Ty,
        payload: bool,
    ) -> Self {
        self.pending.push((left, payload));
        self.pending.push((right, payload));
        self
    }

    pub(super) fn with_program(
        mut self,
        input: &'a [crate::types::Ty],
        output: &'a [crate::types::Ty],
    ) -> Self {
        self.pending
            .reserve(input.len().saturating_add(output.len()));
        self.pending.extend(input.iter().map(|ty| (ty, false)));
        self.pending.extend(output.iter().map(|ty| (ty, false)));
        self
    }

    pub(super) fn with_descriptor(
        self,
        shape: &'a crate::types::NominalShape,
        payload: bool,
    ) -> Option<Self> {
        if let crate::types::NominalShape::Opaque(ty) = shape {
            return Some(self.with_type(ty, payload));
        }
        if let crate::types::NominalShape::Variant(left, right) = shape {
            return Some(self.with_pair(left, right, payload));
        }
        None
    }

    pub(super) fn with_generic(
        self,
        args: &'a [crate::types::Ty; 2],
        shape: &'a crate::types::NominalShape,
    ) -> Option<Self> {
        self.with_pair(&args[0], &args[1], false)
            .with_descriptor(shape, false)
    }
}
