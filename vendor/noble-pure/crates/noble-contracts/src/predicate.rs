#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; predicate resolution mutates only preparation-owned metered worklists, node-metered expression arenas with crate::syntax::TYPE_CAP-bounded types, and work accounting; borrowed source, syntax, bindings, and definitions remain unchanged."
)]

mod application;
mod binary;
mod mapping;
mod references;
pub(crate) mod resolving;
mod structural;
mod unary;

#[derive(Clone, Copy)]
pub(crate) struct Context<'a> {
    pub source: &'a [u8],
    pub tree: &'a crate::syntax::Tree,
    pub inputs: &'a [crate::NamedType],
    pub outputs: &'a [crate::NamedType],
    pub params: &'a [crate::NamedType],
    pub definitions: &'a [crate::LogicDef],
}

pub(crate) struct Arena {
    pub expressions: alloc::vec::Vec<crate::Expr>,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            expressions: alloc::vec::Vec::new(),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; expression insertion measures structural types, meters work, and grows the owned arena Vec."
    )]
    fn push(
        &mut self,
        expr: crate::Expr,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        attempt!(meter.node(expr.span));
        let constructors = match expr.ty.size() {
            Some(size) => size,
            None => {
                return Err(crate::Diagnostic::new(
                    crate::DiagnosticKind::Exhausted,
                    expr.span,
                    "logical expression type exceeds its local bound",
                ))
            }
        };
        if constructors > crate::syntax::TYPE_CAP {
            return Err(crate::Diagnostic::new(
                crate::DiagnosticKind::Exhausted,
                expr.span,
                "logical expression type exceeds 256 constructors",
            ));
        }
        attempt!(meter.charge(crate::syntax::TYPE_CAP, expr.span));
        let id = attempt!(crate::index(self.expressions.len(), expr.span));
        self.expressions.push(expr);
        Ok(id)
    }
}

#[derive(Clone, Copy)]
#[octet::sealed_enum]
enum Op {
    Not,
    And,
    Or,
    Implies,
    Eq,
    Lt,
    Le,
    Add,
    Sub,
    Mul,
    Pair,
    First,
    Second,
    Inl,
    Inr,
    IsLeft,
    Left,
    Right,
    Nil,
    Cons,
    IsNil,
    Head,
    Tail,
    Length,
    Maps,
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; byte-slice comparison is non-const and unsupported operators produce owned diagnostics."
)]
fn operation(name: &[u8], span: crate::Span) -> Result<Op, crate::Diagnostic> {
    match name {
        _ if name == b"not" => Ok(Op::Not),
        _ if name == b"and" => Ok(Op::And),
        _ if name == b"or" => Ok(Op::Or),
        _ if name == b"implies" => Ok(Op::Implies),
        _ if name == b"eq" => Ok(Op::Eq),
        _ if name == b"lt" => Ok(Op::Lt),
        _ if name == b"le" => Ok(Op::Le),
        _ if name == b"add" => Ok(Op::Add),
        _ if name == b"sub" => Ok(Op::Sub),
        _ if name == b"mul" => Ok(Op::Mul),
        _ if name == b"pair" => Ok(Op::Pair),
        _ if name == b"first" => Ok(Op::First),
        _ if name == b"second" => Ok(Op::Second),
        _ if name == b"inl" => Ok(Op::Inl),
        _ if name == b"inr" => Ok(Op::Inr),
        _ if name == b"is-left" => Ok(Op::IsLeft),
        _ if name == b"left" => Ok(Op::Left),
        _ if name == b"right" => Ok(Op::Right),
        _ if name == b"nil" => Ok(Op::Nil),
        _ if name == b"cons" => Ok(Op::Cons),
        _ if name == b"is-nil" => Ok(Op::IsNil),
        _ if name == b"head" => Ok(Op::Head),
        _ if name == b"tail" => Ok(Op::Tail),
        _ if name == b"length" => Ok(Op::Length),
        _ if name == b"maps" => Ok(Op::Maps),
        _ if name == b"host" || name == b"call" || name == b"run" || name == b"effect" => {
            Err(crate::Diagnostic::new(
                crate::DiagnosticKind::Unsupported,
                span,
                "effectful or executable expression is outside the pure logical fragment",
            ))
        }
        _ => Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            span,
            "unknown logical expression operator",
        )),
    }
}

const fn arity(op: Op) -> usize {
    match op {
        Op::Nil => 0,
        Op::Not
        | Op::First
        | Op::Second
        | Op::Inl
        | Op::Inr
        | Op::IsLeft
        | Op::Left
        | Op::Right
        | Op::IsNil
        | Op::Head
        | Op::Tail
        | Op::Length => 1,
        Op::Maps => 3,
        Op::And
        | Op::Or
        | Op::Implies
        | Op::Eq
        | Op::Lt
        | Op::Le
        | Op::Add
        | Op::Sub
        | Op::Mul
        | Op::Pair
        | Op::Cons => 2,
    }
}

pub(crate) fn same(
    left: &noble_kernel::types::Ty,
    right: &noble_kernel::types::Ty,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(crate::syntax::TYPE_CAP.saturating_mul(2), span));
    if left != right {
        Err(crate::invalid(span, "logical expression type mismatch"))
    } else {
        Ok(())
    }
}

pub(crate) fn get(
    expressions: &[crate::Expr],
    id: u32,
    span: crate::Span,
) -> Result<&crate::Expr, crate::Diagnostic> {
    match expressions.get(attempt!(crate::offset(id, span))) {
        Some(expr) => Ok(expr),
        None => Err(crate::internal(span)),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; a missing operand constructs an owned internal diagnostic."
)]
fn required(value: Option<u32>, span: crate::Span) -> Result<u32, crate::Diagnostic> {
    match value {
        Some(value) => Ok(value),
        None => Err(crate::internal(span)),
    }
}
