//! Iterative eligibility walk for the fragment's `Data` constraint.

impl super::Ty {
    /// A resource anywhere inside a payload makes the whole value ineligible,
    /// including through `Pair`, `Sum`, and `List` alternatives.
    ///
    /// The walk borrows its pending nodes: eligibility never copies the
    /// payload it inspects, so an ineligible value is rejected without first
    /// duplicating it.
    pub fn is_data(&self) -> bool {
        let mut work: alloc::vec::Vec<&super::Ty> = alloc::vec::Vec::with_capacity(8);
        work.push(self);
        let mut is_data = true;
        while let Some(node) = work.pop() {
            if work.len() >= super::WORK_CAP {
                is_data = false;
                break;
            }
            match node {
                super::Ty::Resource(_) | super::Ty::LiveRef(_, _, _) => {
                    is_data = false;
                    break;
                }
                super::Ty::Pair(left, right) | super::Ty::Sum(left, right) => {
                    work.push(left);
                    work.push(right);
                }
                super::Ty::List(item) => work.push(item),
                super::Ty::Nominal(_, shape) => match &**shape {
                    super::NominalShape::Opaque(representation) => work.push(representation),
                    super::NominalShape::Variant(left, right) => {
                        work.push(left);
                        work.push(right);
                    }
                },
                super::Ty::GenericNominal(_, args, shape) => {
                    work.push(&args[0]);
                    work.push(&args[1]);
                    match &**shape {
                        super::NominalShape::Variant(left, right) => {
                            work.push(left);
                            work.push(right);
                        }
                        super::NominalShape::Opaque(_) => {
                            is_data = false;
                            break;
                        }
                    }
                }
                super::Ty::Unit
                | super::Ty::Bool
                | super::Ty::I64
                | super::Ty::Text
                | super::Ty::Syntax
                | super::Ty::Contract
                | super::Ty::Evidence
                | super::Ty::Certified
                | super::Ty::Program(_, _, _) => {}
            }
        }
        is_data
    }

    /// Generic family arguments are Data: a valid immutable program is a
    /// value, not an owned copy of resources merely named by its interface.
    /// Live resources in stored payloads remain ineligible.
    pub(crate) fn valid_generic_argument(&self) -> bool {
        match self {
            super::Ty::Unit
            | super::Ty::Bool
            | super::Ty::I64
            | super::Ty::Text
            | super::Ty::Syntax
            | super::Ty::Program(_, _, _) => return true,
            super::Ty::Resource(_)
            | super::Ty::LiveRef(_, _, _)
            | super::Ty::Contract
            | super::Ty::Evidence
            | super::Ty::Certified => return false,
            _ => {}
        }
        let mut work = alloc::vec::Vec::with_capacity(8);
        work.push(self);
        let mut visited = 0usize;
        while let Some(node) = work.pop() {
            if visited >= super::WORK_CAP || work.len() >= super::WORK_CAP {
                return false;
            }
            visited += 1;
            match node {
                super::Ty::Pair(left, right) | super::Ty::Sum(left, right) => {
                    work.push(left);
                    work.push(right);
                }
                super::Ty::List(item) => work.push(item),
                super::Ty::Nominal(_, shape) => match &**shape {
                    super::NominalShape::Opaque(inner) => work.push(inner),
                    super::NominalShape::Variant(left, right) => {
                        work.push(left);
                        work.push(right);
                    }
                },
                super::Ty::GenericNominal(_, args, shape) => {
                    work.push(&args[0]);
                    work.push(&args[1]);
                    match &**shape {
                        super::NominalShape::Variant(left, right) => {
                            work.push(left);
                            work.push(right);
                        }
                        super::NominalShape::Opaque(_) => return false,
                    }
                }
                super::Ty::Unit
                | super::Ty::Bool
                | super::Ty::I64
                | super::Ty::Text
                | super::Ty::Syntax
                | super::Ty::Program(_, _, _) => {}
                super::Ty::Resource(_)
                | super::Ty::LiveRef(_, _, _)
                | super::Ty::Contract
                | super::Ty::Evidence
                | super::Ty::Certified => return false,
            }
        }
        true
    }
}
