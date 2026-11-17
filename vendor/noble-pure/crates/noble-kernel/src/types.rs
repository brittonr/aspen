//! Fragment v0 value, stack, and program types.
//!
//! Every structural walk (`size`, `is_data`, equality) runs iteratively on an
//! explicit work stack and fails closed past a local bound; none of them
//! recurses. `Clone` and `Debug` are the exceptions: they are structural and
//! recurse, so callers must bound a value before copying or formatting it
//! (`acceptance::nodes::node_of` does). Stacks are ordered bottom-first,
//! matching the specification's "top on the right".

mod data;
mod impls;
mod physical;
mod size;

/// Local bound for one type walk; beyond it the predicate fails closed.
const WORK_CAP: usize = 512;

/// Most owned nodes one structural copy of a type may allocate.
///
/// This is the copy guard, not a typing limit: `Ty::size` stays the declared
/// logical size the kernel charges and its consumers read. A generic
/// descriptor stores its arguments and a descriptor built from copies of them,
/// so owned nodes double per nesting layer while the logical size grows by one
/// — the quantity that makes substitution exponential.
///
/// The cap matches the node bound the contract layer already enforces:
/// noble-contracts refuses an inferred representation above 4096 nodes
/// (`TYPE_NODE_CAP` in its inference) before admission, so nothing that layer
/// can hand the kernel is refused here, and a representation larger than it
/// admits is refused before its copy exists.
pub(crate) const PHYSICAL_NODE_CAP: u32 = 4096;

/// Stable identity of a resource kind supplied by the environment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceKind(pub u32);

/// Immutable resolved module identity and declaration ordinal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NominalTypeId {
    pub module: u64,
    pub ordinal: u32,
}

/// Exact, independently checked representation of a nominal type.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(Eq))]
pub enum NominalShape {
    Opaque(alloc::boxed::Box<Ty>),
    Variant(alloc::boxed::Box<Ty>, alloc::boxed::Box<Ty>),
}

/// Stable identity of one host-operation effect.
///
/// The identity is ordered by its payload, compared explicitly at each use.
/// A derived `PartialOrd`/`Ord` would have Aeneas emit a `PartialOrd` instance
/// whose `lt` and `gt` fields are the trait's own default methods, and the
/// Lean backend renders those defaults by handing them the whole instance
/// where the model expects the comparison function. Comparing the payloads
/// keeps ordering inside the scalar operations the backend already models.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffId(pub u32);

/// A finite set of effect identities, kept sorted and deduplicated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffSet(alloc::vec::Vec<EffId>);

impl EffSet {
    /// The empty bound.
    pub fn empty() -> Self {
        EffSet(alloc::vec::Vec::new())
    }

    /// Build from a list, sorting and deduplicating.
    ///
    /// The insertion walk is explicit: slice sort helpers draw core iterator
    /// code into the extraction subject, which Aeneas cannot translate. Order
    /// is compared on the payloads, so no `PartialOrd` instance is involved.
    pub fn from_ids(ids: &[EffId]) -> Self {
        let mut sorted: alloc::vec::Vec<EffId> =
            alloc::vec::Vec::with_capacity(crate::capacity::at_least(ids.len(), 4));
        let mut index = 0;
        while index < ids.len() {
            let id = ids[index];
            let mut position = 0;
            let mut is_placed = false;
            while position < sorted.len() && !is_placed {
                if sorted[position] == id {
                    is_placed = true;
                } else if sorted[position].0 > id.0 {
                    sorted.insert(position, id);
                    is_placed = true;
                } else {
                    position += 1;
                }
            }
            if !is_placed {
                sorted.push(id);
            }
            index += 1;
        }
        EffSet(sorted)
    }

    /// Membership.
    pub fn contains(&self, id: EffId) -> bool {
        let mut index = 0;
        let mut is_found = false;
        while index < self.0.len() {
            if self.0[index] == id {
                is_found = true;
                break;
            }
            index += 1;
        }
        is_found
    }

    /// Least upper bound.
    pub fn union(&self, other: &EffSet) -> EffSet {
        #[expect(
            tigerstyle::raw_arithmetic_overflow,
            reason = "Owner: noble-maintainers; each Vec<EffId> is bounded by isize::MAX bytes and EffId is nonzero-sized, so the sum of two lengths is at most 2 * isize::MAX < usize::MAX; reassess if storage becomes zero-sized."
        )]
        let mut out = alloc::vec::Vec::with_capacity(self.0.len() + other.0.len());
        let mut left = 0usize;
        let mut right = 0usize;
        while left < self.0.len() || right < other.0.len() {
            let is_take_left = right == other.0.len()
                || (left < self.0.len() && self.0[left].0 < other.0[right].0);
            let is_take_right = left == self.0.len()
                || (right < other.0.len() && other.0[right].0 < self.0[left].0);
            if is_take_left {
                out.push(self.0[left]);
                left += 1;
            } else if is_take_right {
                out.push(other.0[right]);
                right += 1;
            } else {
                out.push(self.0[left]);
                left += 1;
                right += 1;
            }
        }
        EffSet(out)
    }

    /// Include one effect, preserving sorted uniqueness with one result
    /// allocation rather than allocating a temporary singleton set.
    pub fn with_id(&self, id: EffId) -> EffSet {
        let mut out = alloc::vec::Vec::with_capacity(self.0.len().saturating_add(1));
        let mut is_inserted = false;
        let mut index = 0;
        while index < self.0.len() {
            let existing = self.0[index];
            if !is_inserted && id.0 < existing.0 {
                out.push(id);
                is_inserted = true;
            }
            if existing == id {
                is_inserted = true;
            }
            out.push(existing);
            index += 1;
        }
        if !is_inserted {
            out.push(id);
        }
        EffSet(out)
    }

    /// Inclusion: every identity here is present in `other`.
    pub fn is_subset_of(&self, other: &EffSet) -> bool {
        let mut index = 0;
        let mut is_subset = true;
        while index < self.0.len() {
            if !other.contains(self.0[index]) {
                is_subset = false;
                break;
            }
            index += 1;
        }
        is_subset
    }

    /// The sorted identities.
    pub fn as_slice(&self) -> &[EffId] {
        &self.0
    }

    /// Number of identities, or `None` when the count is not representable.
    pub fn len(&self) -> Option<u64> {
        u64::try_from(self.0.len()).ok()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }
}

/// A type in the fragment's finite universe.
///
/// The program case carries its stacks and bound directly, so the type family
/// is self-recursive: mutually recursive types would leave Aeneas' dependency
/// analysis with mixed declaration groups it refuses to translate.
///
/// The extracted constructors carry a `Type` suffix: `Unit` and `Bool` would
/// otherwise shadow Lean's own `Unit` and `Bool` inside every declaration
/// named `types.Ty.*`, which is where this type's own methods and instances
/// live. The Rust names are unchanged.
///
/// `Clone`, `Debug`, and equality are hand-written in `impls`: a derived body
/// would reference this type.s own instance, which the Lean backend renders
/// after the method that feeds it.
#[charon::variants_suffix("Type")]
pub enum Ty {
    /// The unit type.
    Unit,
    /// The boolean type.
    Bool,
    /// The wrapping 64-bit integer type.
    I64,
    /// The text type.
    Text,
    /// Inert syntax produced by `reflect`.
    Syntax,
    /// An opaque inert contract descriptor. Supplied by the contract profile;
    /// never constructed by a source word and never data-bearing.
    Contract,
    /// An opaque inert evidence reference. Possession names an accepted
    /// evidence entry; it does not establish acceptance by itself.
    Evidence,
    /// An opaque inert certified-program companion. It carries a subject
    /// program plus inert contract and evidence references.
    Certified,
    /// A product of two payloads.
    Pair(alloc::boxed::Box<Ty>, alloc::boxed::Box<Ty>),
    /// A sum of two payloads.
    Sum(alloc::boxed::Box<Ty>, alloc::boxed::Box<Ty>),
    /// A homogeneous list.
    List(alloc::boxed::Box<Ty>),
    /// A first-class program type: required stack, result stack, latent bound.
    Program(
        alloc::boxed::Box<alloc::vec::Vec<Ty>>,
        alloc::boxed::Box<alloc::vec::Vec<Ty>>,
        EffSet,
    ),
    /// A borrowed live program reference. Its ordered logical input may name
    /// further borrowed references; neither it nor those references are stack
    /// values. Its output must contain only values.
    LiveRef(
        alloc::boxed::Box<alloc::vec::Vec<Ty>>,
        alloc::boxed::Box<alloc::vec::Vec<Ty>>,
        EffSet,
    ),
    /// An opaque resource kind; never data-eligible.
    Resource(ResourceKind),
    /// Resolved nominal identity with its exact opaque representation or two
    /// ordered variant payloads. The descriptor is checked against `Env`.
    Nominal(NominalTypeId, alloc::boxed::Box<NominalShape>),
    /// A two-parameter variant family, its ordered arguments and its
    /// independently checked canonical descriptor.
    GenericNominal(
        NominalTypeId,
        alloc::boxed::Box<[Ty; 2]>,
        alloc::boxed::Box<NominalShape>,
    ),
}

impl Ty {
    /// Build a program type.
    pub fn program(
        stack_in: alloc::vec::Vec<Ty>,
        stack_out: alloc::vec::Vec<Ty>,
        effects: EffSet,
    ) -> Ty {
        Ty::Program(
            alloc::boxed::Box::new(stack_in),
            alloc::boxed::Box::new(stack_out),
            effects,
        )
    }

    /// Build a borrowed live program reference type.
    pub fn live_ref(
        stack_in: alloc::vec::Vec<Ty>,
        stack_out: alloc::vec::Vec<Ty>,
        effects: EffSet,
    ) -> Ty {
        Ty::LiveRef(
            alloc::boxed::Box::new(stack_in),
            alloc::boxed::Box::new(stack_out),
            effects,
        )
    }

    /// A size measure used by the declared type-size limit.
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; every size-walk Step must map explicitly to failure or a computed size; a new state must require review of the fail-closed size contract."
    )]
    pub fn size(&self) -> Option<u32> {
        let mut walk = size::Walk {
            todo: alloc::vec::Vec::with_capacity(8),
            sizes: alloc::vec::Vec::with_capacity(8),
        };
        walk.todo.push((self, false));
        let mut outcome = size::Step::Continue;
        while matches!(outcome, size::Step::Continue) {
            let (next, step) = size::walk_step(walk);
            walk = next;
            outcome = step;
        }
        match outcome {
            size::Step::Failed => None,
            size::Step::Continue | size::Step::Done => walk.sizes.pop(),
        }
    }
}

pub fn stack_is_data(stack: &[Ty]) -> bool {
    let mut index = 0;
    let mut is_data = true;
    while index < stack.len() {
        if !stack[index].is_data() {
            is_data = false;
            break;
        }
        index += 1;
    }
    is_data
}
