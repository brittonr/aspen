/// A refused belt artifact or execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The graph is not in the admitted total `I64`/`Bool` fragment.
    UnsupportedGraph,
    /// The declared capacity is outside `2..=MAX_CAPACITY`.
    Capacity,
    /// The declared preservation-slot count exceeds `MAX_SAVE_SLOTS`.
    SaveSlots,
    /// Graph nodes or outputs exceed this backend's bound.
    GraphBound,
    /// The emitted instruction count exceeds this backend's bound.
    InstructionBound,
    /// The serialized artifact exceeds the byte bound or has inconsistent lengths.
    ArtifactBound,
    /// The artifact header or instruction encoding is malformed.
    Malformed,
    /// The artifact format, lowerer or checker revision is not selected.
    Revision,
    /// The artifact names a different graph identity.
    GraphIdentity,
    /// The instruction bytes differ from their declared digest.
    InstructionDigest,
    /// The artifact binding digest does not match its header fields.
    ArtifactBinding,
    /// A belt age is outside the current belt.
    StaleAge,
    /// An instruction does not implement the graph node at the next canonical index.
    BodyMismatch,
    /// A graph operand is not present at the declared belt age.
    OperandMismatch,
    /// A preservation slot index is outside the artifact's declared slots.
    SlotBound,
    /// A reload or overwrite reads a slot that has never been initialized.
    UninitializedSlot,
    /// A live value would be lost by eviction or overwrite.
    LiveValueLost,
    /// A preservation slot would overwrite a live value with no other location.
    LiveSlotOverwrite,
    /// An output read is missing, extra, or in a different order.
    OutputMismatch,
    /// The instruction stream does not define every graph node exactly once.
    IncompleteProgram,
    /// Runtime inputs do not match the admitted graph interface.
    Inputs,
    /// The runtime interpreter observed an impossible checked state.
    RuntimeFault,
}

/// A freshly lowered artifact and bounded work counters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lowered {
    /// Canonically encoded artifact bytes.
    pub bytes: alloc::vec::Vec<u8>,
    /// Number of decoded instructions, including storage and output operations.
    pub instruction_count: usize,
    /// Explicit save operations emitted to preserve values before eviction.
    pub preservation_saves: usize,
    /// Explicit reload operations emitted to recover saved values.
    pub preservation_loads: usize,
}

/// A checker-issued token borrowing the exact artifact bytes it validated.
#[derive(Clone, Copy, Debug)]
pub struct CheckedArtifact<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) graph_id: crate::ir::Digest,
    pub(crate) belt_slots: u8,
    pub(crate) save_slots: u8,
    pub(crate) instruction_digest: crate::ir::Digest,
    pub(crate) artifact_binding: crate::ir::Digest,
    pub(crate) instruction_count: usize,
}

/// A structurally decoded artifact whose byte digests and revisions match.
#[derive(Clone, Copy, Debug)]
pub struct ArtifactView<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) graph_id: crate::ir::Digest,
    pub(crate) belt_slots: u8,
    pub(crate) save_slots: u8,
    pub(crate) instruction_bytes: &'a [u8],
    pub(crate) instruction_digest: crate::ir::Digest,
    pub(crate) artifact_binding: crate::ir::Digest,
    pub(crate) instruction_count: usize,
}

impl ArtifactView<'_> {
    /// Graph identity bound by the artifact.
    #[must_use]
    pub const fn graph_id(&self) -> crate::ir::Digest {
        self.graph_id
    }

    /// Selected belt capacity.
    #[must_use]
    pub const fn capacity_values(&self) -> u8 {
        self.belt_slots
    }

    /// Explicit preservation-slot count.
    #[must_use]
    pub const fn save_slots(&self) -> u8 {
        self.save_slots
    }

    /// Canonically encoded instruction bytes (not the enclosing header).
    #[must_use]
    pub const fn instruction_bytes(&self) -> &'_ [u8] {
        self.instruction_bytes
    }

    /// Number of structurally valid encoded instructions.
    #[must_use]
    pub const fn instruction_count(&self) -> usize {
        self.instruction_count
    }
}

impl CheckedArtifact<'_> {
    /// Number of instructions the independent checker accepted.
    #[must_use]
    pub const fn instruction_count(&self) -> usize {
        self.instruction_count
    }

    /// Selected belt capacity.
    #[must_use]
    pub const fn capacity_values(&self) -> u8 {
        self.belt_slots
    }

    /// Declared explicit preservation-slot count.
    #[must_use]
    pub const fn save_slots(&self) -> u8 {
        self.save_slots
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Instruction {
    Input(u32),
    Constant(u64),
    Add(u8, u8),
    Sub(u8, u8),
    Mul(u8, u8),
    Eq(u8, u8),
    Save(u8, u8),
    Load(u8),
    Output(u8),
}
