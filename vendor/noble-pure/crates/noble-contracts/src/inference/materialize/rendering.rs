enum Part {
    Term(u32),
    Space,
    Close,
    Comma,
    Arrow,
}

struct Traversal {
    pending: alloc::vec::Vec<Part>,
    text: alloc::string::String,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; each rendering step charges work, validates term IDs, and enforces fixed output/pending bounds; a rendering refusal is returned without replacing the original type error."
)]
pub(super) fn describe(
    arena: &crate::inference::Arena,
    root: u32,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<alloc::string::String, crate::Diagnostic> {
    // At most 255 pending parts precede scheduling at most four children.
    let mut rendering = Traversal {
        pending: alloc::vec::Vec::with_capacity(259),
        text: alloc::string::String::new(),
    };
    rendering.pending.push(Part::Term(root));
    let mut failure = None;
    while let Some(part) = rendering.pending.pop() {
        match rendering.step(part, arena, span, meter) {
            Ok(true) => break,
            Ok(false) => {}
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(rendering.text),
    }
}

impl Traversal {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; rendering mutates an owned growable string and invokes non-const arena traversal under the live diagnostic work meter."
    )]
    fn step(
        &mut self,
        part: Part,
        arena: &crate::inference::Arena,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<bool, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        if self.text.len() >= 384 || self.pending.len() >= 256 {
            self.text.push_str("...");
            return Ok(true);
        }
        #[expect(
            tigerstyle::fragile_exhaustive_enum_match,
            reason = "Owner: noble-maintainers; every diagnostic continuation must emit text or visit its term; a new Part variant must require an explicit rendering rule."
        )]
        match part {
            Part::Space => self.text.push(' '),
            Part::Close => self.text.push('>'),
            Part::Comma => self.text.push(','),
            Part::Arrow => self.text.push_str(" -- "),
            Part::Term(id) => attempt!(self.term(id, arena, span, meter)),
        }
        Ok(false)
    }

    fn term(
        &mut self,
        id: u32,
        arena: &crate::inference::Arena,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        let id = attempt!(arena.root(id, span, meter));
        let term = attempt!(arena.get(id, span));
        let text = match term {
            crate::inference::Term::Hole(crate::inference::Sort::Stack) => "?stack",
            crate::inference::Term::Hole(crate::inference::Sort::Value) => "?value",
            crate::inference::Term::RigidValue(_) => "forall-value",
            crate::inference::Term::RigidStack(_) => "forall-stack",
            crate::inference::Term::Unit => "Unit",
            crate::inference::Term::Bool => "Bool",
            crate::inference::Term::I64 => "I64",
            crate::inference::Term::Text => "Text",
            crate::inference::Term::Syntax => "Syntax",
            crate::inference::Term::Contract => "Contract",
            crate::inference::Term::Evidence => "Evidence",
            crate::inference::Term::Certified => "Certified",
            crate::inference::Term::Resource(_) => "Resource",
            crate::inference::Term::Nominal(_) => "Nominal",
            crate::inference::Term::GenericNominal(_, a, b) => {
                self.pending.push(Part::Close);
                self.pending.push(Part::Term(b));
                self.pending.push(Part::Comma);
                self.pending.push(Part::Term(a));
                "Nominal<"
            }
            crate::inference::Term::Empty => "[]",
            crate::inference::Term::Link(_) => return Err(crate::internal(span)),
            crate::inference::Term::Push(stack, value) => {
                self.pending.push(Part::Term(value));
                self.pending.push(Part::Space);
                self.pending.push(Part::Term(stack));
                return Ok(());
            }
            crate::inference::Term::List(item) => {
                self.pending.push(Part::Close);
                self.pending.push(Part::Term(item));
                "List<"
            }
            crate::inference::Term::Pair(a, b)
            | crate::inference::Term::Sum(a, b)
            | crate::inference::Term::Program(a, b) => {
                self.pending.push(Part::Close);
                self.pending.push(Part::Term(b));
                self.pending
                    .push(if matches!(term, crate::inference::Term::Program(_, _)) {
                        Part::Arrow
                    } else {
                        Part::Comma
                    });
                self.pending.push(Part::Term(a));
                match term {
                    crate::inference::Term::Pair(_, _) => "Pair<",
                    crate::inference::Term::Sum(_, _) => "Sum<",
                    _ => "Program<",
                }
            }
        };
        self.text.push_str(text);
        Ok(())
    }
}
