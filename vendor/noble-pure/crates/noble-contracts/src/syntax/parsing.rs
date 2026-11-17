struct Frame {
    start: u32,
    is_square: bool,
    children: alloc::vec::Vec<u32>,
}

pub(super) struct State {
    tree: super::Tree,
    frames: alloc::vec::Vec<Frame>,
    root: Option<u32>,
    pub(super) position: usize,
}

impl State {
    pub(super) fn new() -> Self {
        Self {
            tree: super::Tree {
                nodes: alloc::vec::Vec::new(),
                root: 0,
            },
            frames: alloc::vec::Vec::new(),
            root: None,
            position: 0,
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; malformed syntax and depth/work bounds return diagnostics. Successful source.get(position) proves position < source.len() before the comment successor."
)]
pub(super) fn step(
    source: &[u8],
    full: crate::Span,
    state: &mut State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let start = attempt!(crate::index(state.position, full));
    let at = crate::Span {
        start,
        end: start.saturating_add(1),
    };
    attempt!(meter.charge(1, at));
    let byte = match source.get(state.position) {
        Some(byte) => *byte,
        None => return Err(crate::internal(at)),
    };
    if matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
        state.position += 1;
        return Ok(());
    }
    if byte == b';' {
        state.position = attempt!(comment_end(source, state.position + 1, at, meter));
        return Ok(());
    }
    if byte == b'(' || byte == b'[' {
        let depth = attempt!(crate::index(state.frames.len(), at)).saturating_add(1);
        attempt!(meter.depth(depth, at));
        attempt!(meter.node(at));
        state.frames.push(Frame {
            start,
            is_square: byte == b'[',
            children: alloc::vec::Vec::new(),
        });
        state.position += 1;
        return Ok(());
    }
    let node = if byte == b')' || byte == b']' {
        attempt!(close(byte, at, state))
    } else {
        if matches!(byte, b'"' | b'\'' | b'{' | b'}') || byte >= 128 {
            return Err(crate::Diagnostic::new(
                crate::DiagnosticKind::Unsupported,
                at,
                "string, host, and non-ASCII source syntax is outside contract version 1",
            ));
        }
        state.position = attempt!(atom_end(source, state.position, at, meter));
        attempt!(meter.node(at));
        super::Node {
            span: crate::Span {
                start,
                end: attempt!(crate::index(state.position, at)),
            },
            form: super::Form::Atom,
        }
    };
    let id = attempt!(crate::index(state.tree.nodes.len(), node.span));
    state.tree.nodes.push(node);
    match state.frames.pop() {
        Some(mut frame) => {
            frame.children.push(id);
            state.frames.push(frame);
        }
        None => {
            if state.root.is_some() {
                return Err(crate::invalid(
                    at,
                    "expected exactly one contract container",
                ));
            }
            state.root = Some(id);
        }
    }
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; closing pops an owned Vec frame and validates delimiter pairing and source-index conversion through diagnostics."
)]
fn close(byte: u8, at: crate::Span, state: &mut State) -> Result<super::Node, crate::Diagnostic> {
    let frame = match state.frames.pop() {
        Some(frame) => frame,
        None => return Err(crate::invalid(at, "unmatched closing delimiter")),
    };
    if frame.is_square != (byte == b']') {
        return Err(crate::invalid(at, "mismatched closing delimiter"));
    }
    state.position += 1;
    Ok(super::Node {
        span: crate::Span {
            start: frame.start,
            end: attempt!(crate::index(state.position, at)),
        },
        form: if frame.is_square {
            super::Form::Square(frame.children)
        } else {
            super::Form::Round(frame.children)
        },
    })
}

fn comment_end(
    source: &[u8],
    mut position: usize,
    at: crate::Span,
    meter: &mut crate::Meter,
) -> Result<usize, crate::Diagnostic> {
    let mut failure = None;
    while position < source.len() {
        match comment_byte(source, position, at, meter) {
            Ok(true) => break,
            Ok(false) => position += 1,
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(position),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; comment scanning charges runtime work and checked lookup can construct an owned diagnostic."
)]
fn comment_byte(
    source: &[u8],
    position: usize,
    at: crate::Span,
    meter: &mut crate::Meter,
) -> Result<bool, crate::Diagnostic> {
    attempt!(meter.charge(1, at));
    let byte = match source.get(position) {
        Some(byte) => *byte,
        None => return Err(crate::internal(at)),
    };
    Ok(byte == b'\n')
}

fn atom_end(
    source: &[u8],
    mut position: usize,
    at: crate::Span,
    meter: &mut crate::Meter,
) -> Result<usize, crate::Diagnostic> {
    let mut failure = None;
    while position < source.len() {
        match atom_byte(source, position, at, meter) {
            Ok(true) => position += 1,
            Ok(false) => break,
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(position),
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; atom bytes are metered and unsupported syntax returns a bounded source diagnostic rather than a panic."
)]
fn atom_byte(
    source: &[u8],
    position: usize,
    at: crate::Span,
    meter: &mut crate::Meter,
) -> Result<bool, crate::Diagnostic> {
    let next = match source.get(position) {
        Some(next) => *next,
        None => return Err(crate::internal(at)),
    };
    if matches!(
        next,
        b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')' | b'[' | b']' | b';'
    ) {
        return Ok(false);
    }
    if !next.is_ascii_graphic() || matches!(next, b'"' | b'\'' | b'{' | b'}') {
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            crate::Span {
                start: attempt!(crate::index(position, at)),
                end: attempt!(crate::index(position.saturating_add(1), at)),
            },
            "unsupported source byte",
        ));
    }
    attempt!(meter.charge(1, at));
    Ok(true)
}

pub(super) fn finish(
    mut state: State,
    full: crate::Span,
) -> Result<super::Tree, crate::Diagnostic> {
    if let Some(frame) = state.frames.pop() {
        return Err(crate::invalid(
            crate::Span {
                start: frame.start,
                end: full.end,
            },
            "unclosed delimiter",
        ));
    }
    state.tree.root = match state.root {
        Some(root) => root,
        None => return Err(crate::invalid(full, "empty contract source")),
    };
    Ok(state.tree)
}
