#![expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the module path names the domain and the item name keeps its canonical operation spelling"
)]

//! D11 immutable parameter snapshots and their identities.
//!
//! `ParamSnapshotId = H("noble.ngraph.params.v0" ‖ GraphLogicalId ‖ ordered
//! param layout ‖ exact LE bytes)`. A hash identifies bytes; it does not prove
//! correctness. Snapshots are retained owners (D12): immutable and shareable.

use crate::identity::IdentityScheme;
/// Ordered parameter layout key of a graph: the ordered param slot indices.
#[must_use]
pub fn layout_key(ids: &dyn IdentityScheme, graph: &crate::ir::Graph) -> crate::ir::Digest {
    layout_key_of(ids, &graph.param_slots())
}

/// Param layout key of an ordered param slot list.
#[must_use]
pub fn layout_key_of(ids: &dyn IdentityScheme, params: &[u32]) -> crate::ir::Digest {
    let mut c = crate::ir::Canon::new(ids, "noble.ngraph.param-layout.v0");
    c.u64(params.len() as u64);
    for &k in params {
        c.u32(k);
        c.str(crate::ir::ScalarType::F32.name());
    }
    c.finish()
}

/// An immutable parameter snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamSnapshot {
    graph: crate::ir::Digest,
    layout: crate::ir::Digest,
    slots: alloc::rc::Rc<[u32]>,
    bytes: alloc::rc::Rc<[u8]>,
    id: crate::ir::Digest,
}

fn snapshot_id(
    ids: &dyn IdentityScheme,
    graph: &crate::ir::Digest,
    slots: &[u32],
    bytes: &[u8],
) -> crate::ir::Digest {
    let mut c = crate::ir::Canon::new(ids, "noble.ngraph.params.v0");
    c.bytes(graph);
    c.u64(slots.len() as u64);
    for k in slots {
        c.u32(*k);
    }
    c.bytes(bytes);
    c.finish()
}

impl ParamSnapshot {
    /// Builds a snapshot for `checked` from param values in param-slot order.
    ///
    /// # Errors
    /// `E-SNAPSHOT-LAYOUT` on a count mismatch, `E-NONFINITE-UPDATE` on a
    /// non-finite value, `E-BUDGET-SNAPSHOT-BYTES` over the `/0` budget.
    pub fn new(
        ids: &dyn IdentityScheme,
        checked: &crate::check::CheckedGraph,
        values: &[f32],
    ) -> Result<Self, crate::diag::Diagnostic> {
        Self::with_budget(
            ids,
            checked,
            values,
            crate::check::Budgets::default().snapshot_bytes,
        )
    }

    /// As [`Self::new`] with an explicit snapshot byte budget.
    ///
    /// # Errors
    /// See [`Self::new`].
    pub fn with_budget(
        ids: &dyn IdentityScheme,
        checked: &crate::check::CheckedGraph,
        values: &[f32],
        snapshot_bytes: u64,
    ) -> Result<Self, crate::diag::Diagnostic> {
        let g = checked.graph();
        let slots: alloc::vec::Vec<u32> = g.param_slots();
        if slots.len() != values.len() {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::SnapshotLayout,
                crate::diag::Site::Graph,
                alloc::format!("{} values for {} param slots", values.len(), slots.len()),
            ));
        }
        let observed = (values.len() * 4) as u64;
        if observed > snapshot_bytes {
            return Err(crate::diag::Diagnostic::budget(
                crate::diag::BudgetKind::SnapshotBytes,
                snapshot_bytes,
                observed,
            ));
        }
        if let Some(k) = values.iter().position(|v| !v.is_finite()) {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::NonfiniteUpdate,
                crate::diag::Site::Slot(slots[k]),
                "non-finite parameter value",
            ));
        }
        let mut bytes = alloc::vec::Vec::with_capacity(values.len() * 4);
        for v in values {
            bytes.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        Ok(Self::from_parts(
            ids,
            checked.id(),
            checked.param_layout(),
            slots,
            bytes,
        ))
    }

    /// Raw constructor (identity recomputed from the parts). Used to model
    /// foreign/stale snapshots in tests and by the trainer.
    #[must_use]
    pub fn from_parts(
        ids: &dyn IdentityScheme,
        graph: crate::ir::Digest,
        layout: crate::ir::Digest,
        slots: alloc::vec::Vec<u32>,
        bytes: alloc::vec::Vec<u8>,
    ) -> Self {
        let id = snapshot_id(ids, &graph, &slots, &bytes);
        Self {
            graph,
            layout,
            slots: slots.into(),
            bytes: bytes.into(),
            id,
        }
    }

    /// `ParamSnapshotId`.
    #[must_use]
    pub const fn id(&self) -> crate::ir::Digest {
        self.id
    }

    /// `GraphLogicalId` this snapshot belongs to.
    #[must_use]
    pub const fn graph(&self) -> crate::ir::Digest {
        self.graph
    }

    /// Param layout key.
    #[must_use]
    pub const fn layout(&self) -> crate::ir::Digest {
        self.layout
    }

    /// Ordered param slots.
    #[must_use]
    pub fn slots(&self) -> &[u32] {
        &self.slots
    }

    /// Exact little-endian binary32 bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Shared owner of the bytes (for retained-owner accounting).
    #[must_use]
    pub fn bytes_rc(&self) -> alloc::rc::Rc<[u8]> {
        alloc::rc::Rc::clone(&self.bytes)
    }

    /// Param values in slot order.
    #[must_use]
    pub fn values(&self) -> alloc::vec::Vec<f32> {
        self.bytes
            .chunks_exact(4)
            .map(|c| f32::from_bits(u32::from_le_bytes([c[0], c[1], c[2], c[3]])))
            .collect()
    }

    /// Value bits at snapshot position `p` (param-slot order).
    #[must_use]
    pub fn bits_at(&self, p: usize) -> Option<u32> {
        let c = self.bytes.get(p * 4..p * 4 + 4)?;
        Some(u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
    }

    /// Value bits of param slot `k`.
    #[must_use]
    pub fn bits_of_slot(&self, k: u32) -> Option<u32> {
        let p = self.slots.iter().position(|s| *s == k)?;
        let c = &self.bytes[p * 4..p * 4 + 4];
        Some(u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
    }

    /// D11 "Use": refuses a snapshot of another graph or layout.
    ///
    /// # Errors
    /// `E-SNAPSHOT-STALE` or `E-SNAPSHOT-LAYOUT`.
    pub fn check_use(
        &self,
        checked: &crate::check::CheckedGraph,
    ) -> Result<(), crate::diag::Diagnostic> {
        if self.graph != checked.id() {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::SnapshotStale,
                crate::diag::Site::Graph,
                "snapshot belongs to another GraphLogicalId",
            ));
        }
        let params = checked.param_slots();
        if self.layout != checked.param_layout()
            || *self.slots != *params
            || self.bytes.len() != params.len() * 4
        {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::SnapshotLayout,
                crate::diag::Site::Graph,
                "snapshot param layout differs from the graph's",
            ));
        }
        Ok(())
    }
}
