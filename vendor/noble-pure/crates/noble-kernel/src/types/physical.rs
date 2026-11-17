//! Borrowed walk over the physical (owned) size of one type tree.
//!
//! `Ty::size` counts the declared, logical constructors a type names: one per
//! variant layer, whatever the representation. Copying a value is a different
//! quantity. A generic descriptor stores its arguments *and* a descriptor
//! built from copies of those arguments, so one nesting layer doubles the
//! owned nodes while the logical size grows by one. The kernel therefore keeps
//! the declared size for its limits and measures a copy against this physical
//! count before it makes the copy.

impl super::Ty {
    /// Owned nodes one structural copy of this type would allocate.
    ///
    /// The walk borrows its pending nodes and visits each at most once, so it
    /// describes the copy without making one. `None` means the tree is larger
    /// than [`super::PHYSICAL_NODE_CAP`], which fails the copy closed.
    pub(crate) fn physical_nodes(&self) -> Option<u32> {
        let mut work: alloc::vec::Vec<&super::Ty> = alloc::vec::Vec::with_capacity(8);
        work.push(self);
        let mut visited = 0u32;
        while let Some(node) = work.pop() {
            visited = visited.saturating_add(1);
            if visited > super::PHYSICAL_NODE_CAP || work.len() >= super::WORK_CAP {
                return None;
            }
            match node {
                // A generic descriptor holds its argument array and a copy of
                // those arguments inside the descriptor.
                super::Ty::GenericNominal(_, args, shape) => {
                    work.push(&args[0]);
                    work.push(&args[1]);
                    match &**shape {
                        super::NominalShape::Variant(left, right) => {
                            work.push(left);
                            work.push(right);
                        }
                        super::NominalShape::Opaque(representation) => {
                            work.push(representation);
                        }
                    }
                }
                super::Ty::Nominal(_, shape) => match &**shape {
                    super::NominalShape::Opaque(representation) => work.push(representation),
                    super::NominalShape::Variant(left, right) => {
                        work.push(left);
                        work.push(right);
                    }
                },
                super::Ty::Pair(left, right) | super::Ty::Sum(left, right) => {
                    work.push(left);
                    work.push(right);
                }
                super::Ty::List(item) => work.push(item),
                super::Ty::Program(input, output, _) | super::Ty::LiveRef(input, output, _) => {
                    work.extend(input.iter());
                    work.extend(output.iter());
                }
                super::Ty::Unit
                | super::Ty::Bool
                | super::Ty::I64
                | super::Ty::Text
                | super::Ty::Syntax
                | super::Ty::Contract
                | super::Ty::Evidence
                | super::Ty::Certified
                | super::Ty::Resource(_) => {}
            }
        }
        Some(visited)
    }
}
