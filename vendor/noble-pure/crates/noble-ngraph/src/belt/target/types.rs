/// Operation class assigned to a physical machine unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitClass {
    /// Addition, subtraction and equality.
    Alu,
    /// Wrapping multiplication.
    Multiply,
    /// Input/constant materialization and preservation traffic.
    Move,
}

/// Immutable descriptor selected by the machine-family revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachineDescriptor {
    /// Maximum operations accepted in one issue cycle.
    pub issue_width: u8,
    /// Number of newest-first value positions in the belt.
    pub belt_slots: u8,
    /// Number of explicit preservation slots.
    pub save_slots: u8,
    /// Number of pipelined ALU units.
    pub alu_units: u8,
    /// Number of pipelined multiply units.
    pub multiply_units: u8,
    /// Number of pipelined move units.
    pub move_units: u8,
    /// ALU/equality latency in cycles.
    pub alu_cycles: u8,
    /// Multiply latency in cycles.
    pub multiply_cycles: u8,
    /// Move latency in cycles.
    pub move_cycles: u8,
    /// Minimum issue gap for every unit, in cycles.
    pub issue_gap_cycles: u8,
}

impl MachineDescriptor {
    /// Returns a unit's operation class using globally unique physical slot ids.
    #[must_use]
    pub const fn unit_class(self, unit: u8) -> Option<UnitClass> {
        if unit < self.alu_units {
            Some(UnitClass::Alu)
        } else if unit < self.alu_units + self.multiply_units {
            Some(UnitClass::Multiply)
        } else if unit < self.alu_units + self.multiply_units + self.move_units {
            Some(UnitClass::Move)
        } else {
            None
        }
    }

    /// Returns the fixed latency of a unit class.
    #[must_use]
    pub const fn completion_cycles(self, class: UnitClass) -> u8 {
        match class {
            UnitClass::Alu => self.alu_cycles,
            UnitClass::Multiply => self.multiply_cycles,
            UnitClass::Move => self.move_cycles,
        }
    }

    pub(super) const fn bytes(self) -> [u8; 10] {
        [
            self.issue_width,
            self.belt_slots,
            self.save_slots,
            self.alu_units,
            self.multiply_units,
            self.move_units,
            self.alu_cycles,
            self.multiply_cycles,
            self.move_cycles,
            self.issue_gap_cycles,
        ]
    }
}

/// Fixed target model. There are no caller-supplied descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Machine {
    /// Width 1, belt 4, one ALU/multiply/move; multiply latency 3.
    T1 = 1,
    /// Width 2, belt 8, two ALU/move and one multiply; multiply latency 2.
    T2 = 2,
}

impl Machine {
    /// Resolve the immutable descriptor selected by this id.
    #[must_use]
    pub const fn descriptor(self) -> MachineDescriptor {
        match self {
            Self::T1 => MachineDescriptor {
                issue_width: 1,
                belt_slots: 4,
                save_slots: 8,
                alu_units: 1,
                multiply_units: 1,
                move_units: 1,
                alu_cycles: 1,
                multiply_cycles: 3,
                move_cycles: 1,
                issue_gap_cycles: 1,
            },
            Self::T2 => MachineDescriptor {
                issue_width: 2,
                belt_slots: 8,
                save_slots: 8,
                alu_units: 2,
                multiply_units: 1,
                move_units: 2,
                alu_cycles: 1,
                multiply_cycles: 2,
                move_cycles: 1,
                issue_gap_cycles: 1,
            },
        }
    }

    pub(super) const fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Self::T1),
            2 => Some(Self::T2),
            _ => None,
        }
    }
}

/// One graph operation issued on a selected unit at a selected cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Materialize an input graph node from its interface slot.
    Input { node: u32, slot: u32 },
    /// Materialize an I64 literal graph node.
    Constant { node: u32, bits: u64 },
    /// Wrapping addition from two pre-cycle belt ages.
    Add {
        node: u32,
        left_age: u8,
        right_age: u8,
    },
    /// Wrapping subtraction from two pre-cycle belt ages.
    Sub {
        node: u32,
        left_age: u8,
        right_age: u8,
    },
    /// Wrapping multiplication from two pre-cycle belt ages.
    Multiply {
        node: u32,
        left_age: u8,
        right_age: u8,
    },
    /// Equality from two pre-cycle belt ages.
    Equal {
        node: u32,
        left_age: u8,
        right_age: u8,
    },
    /// Save a belt value to an explicit slot; completion updates the slot.
    Save { node: u32, age: u8, slot: u8 },
    /// Load a saved graph value; completion inserts it at belt age zero.
    Load { node: u32, slot: u8 },
}

/// One declared issued operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Issue {
    /// Issue cycle, starting at zero.
    pub cycle: u32,
    /// Issue lane in `0..issue_width`.
    pub lane: u8,
    /// Globally numbered physical unit slot.
    pub unit: u8,
    /// Operation issued at this point.
    pub operation: Operation,
}

/// A final graph output in the required graph order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputRead {
    /// Graph node whose value is read.
    pub node: u32,
}

/// Untrusted scheduler result and bounded work counters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lowered {
    /// Fully identity-bound encoded schedule bytes.
    pub bytes: alloc::vec::Vec<u8>,
    /// Number of issued operations, including save/load traffic.
    pub event_count: usize,
    /// Number of cycles from cycle zero through the last completion.
    pub modeled_cycles: u32,
    /// Number of explicit preservation saves.
    pub preservation_saves: usize,
    /// Number of explicit preservation loads.
    pub preservation_loads: usize,
    /// Deterministic list-scheduler candidate decisions.
    pub scheduling_work: usize,
}

/// Refusal of schedule production, admission, or execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Graph is outside the checked I64/Bool backend fragment.
    UnsupportedGraph,
    /// Schedule names an unknown versioned machine.
    UnknownMachine,
    /// Encoded descriptor differs from the fixed named profile.
    DescriptorMismatch,
    /// Schedule is not bound to the checked graph identity.
    GraphIdentity,
    /// Event or output counts exceed finite bounds.
    EventBound,
    /// Schedule horizon or total checker work exceeds finite bounds.
    HorizonBound,
    /// Artifact length or field arithmetic is malformed or over bound.
    ArtifactBound,
    /// Header, revision or instruction bytes are malformed.
    Malformed,
    /// Content digest or artifact binding does not match the encoded bytes.
    DigestMismatch,
    /// Caller selected a machine other than the encoded machine.
    MachineMismatch,
    /// Event uses a lane outside the selected descriptor.
    IssueLane,
    /// Event uses a unit absent from the selected descriptor.
    UnitBound,
    /// Event operation is assigned to an incompatible unit class.
    UnitClass,
    /// One issue lane accepts more than one operation in a cycle.
    IssueConflict,
    /// One physical unit accepts more than one operation in a cycle.
    UnitConflict,
    /// Physical unit initiation interval is violated.
    InitiationInterval,
    /// Operation dependency is not complete at cycle start.
    DependencyNotReady,
    /// Operand belt age does not name the graph's required operand.
    OperandMismatch,
    /// Operation does not correspond to its declared graph node.
    NodeMismatch,
    /// Graph node is issued more than once.
    DuplicateNode,
    /// Graph node definition is omitted.
    MissingNode,
    /// Operation completion cycle overflows or exceeds the schedule horizon.
    CompletionHorizon,
    /// A completion would evict a still-live node with no saved copy.
    LiveValueLost,
    /// Save slot index is outside the selected descriptor.
    SaveSlotBound,
    /// Save would overwrite a live value in its slot.
    LiveSaveOverwrite,
    /// Load reads an uninitialized or mismatched save slot.
    UninitializedSave,
    /// Final outputs are absent, unavailable, or not in exact graph order.
    OutputMismatch,
    /// Producer exhausted its bounded search/work budget.
    SchedulingExhausted,
    /// Independent checker exhausted its bounded work budget.
    CheckingExhausted,
    /// Runtime input arity or types do not match the graph interface.
    Inputs,
    /// A checker-issued token was rebound to different schedule bytes.
    PostCheckMutation,
    /// A checked execution encountered an impossible runtime state.
    RuntimeFault,
}

/// Checker-issued token tied to the exact schedule bytes validated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckedSchedule<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) machine: Machine,
    pub(super) graph_id: crate::ir::Digest,
    pub(super) descriptor_digest: crate::ir::Digest,
    pub(super) schedule_digest: crate::ir::Digest,
    pub(super) artifact_binding: crate::ir::Digest,
    pub(super) event_count: usize,
    pub(super) output_count: usize,
}

impl CheckedSchedule<'_> {
    /// Target selected by the independent checker.
    #[must_use]
    pub const fn machine(&self) -> Machine {
        self.machine
    }

    /// Number of checked issue events.
    #[must_use]
    pub const fn event_count(&self) -> usize {
        self.event_count
    }

    /// Number of ordered final reads.
    #[must_use]
    pub const fn output_count(&self) -> usize {
        self.output_count
    }
}

/// Structurally decoded artifact with fixed descriptor and digest validation.
#[derive(Clone, Debug)]
pub struct DecodedSchedule {
    pub(super) machine: Machine,
    pub(super) descriptor_digest: crate::ir::Digest,
    pub(super) graph_id: crate::ir::Digest,
    pub(super) events: alloc::vec::Vec<Issue>,
    pub(super) outputs: alloc::vec::Vec<OutputRead>,
    pub(super) schedule_digest: crate::ir::Digest,
    pub(super) artifact_binding: crate::ir::Digest,
}

impl DecodedSchedule {
    /// Machine id declared by this artifact.
    #[must_use]
    pub const fn machine(&self) -> Machine {
        self.machine
    }

    /// Issue events in encoded order.
    #[must_use]
    pub fn events(&self) -> &[Issue] {
        &self.events
    }

    /// Final output reads in encoded order.
    #[must_use]
    pub fn outputs(&self) -> &[OutputRead] {
        &self.outputs
    }
}
