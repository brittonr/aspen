//! Editor-only syntax transport. No editor node is a kernel candidate or a
//! `.noble` token. An executable tree is re-parsed by normal source admission.

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
use alloc::{string::String, vec::Vec};

pub const FORMAT: u32 = 1;

#[derive(Clone, Debug)]
pub enum Node {
    Integer(i64),
    Boolean(bool),
    Word(String),
    Quotation(Vec<Node>),
    Hole,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub format: u32,
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub struct HoleConstraint {
    pub input_stack: String,
    pub output_stack: String,
    /// Additional guaranteed suffix if the known incoming suffix is retained;
    /// the editor hole may instead consume it, so no preservation is inferred.
    pub required_output_suffix: Vec<String>,
    pub known_effects: Vec<u32>,
    pub unresolved_effect: bool,
}

#[derive(Debug)]
pub struct Analysis {
    pub holes: Vec<HoleConstraint>,
    pub known_effects: Vec<u32>,
    pub unresolved_effect: bool,
}

struct Builder {
    tree: super::Tree,
    source: Vec<u8>,
    holes: usize,
    /// The generated span of the first hole. An admission refusal is located there rather than at
    /// the whole tree, so a client learns which node to fix and a nested hole maps to its own path.
    first_hole: Option<crate::Span>,
    /// One entry per generated node: its synthetic span and the bounded editor
    /// path that produced it. Paths are indices into the editor node tree, so
    /// they are stable within one input and never claim to be source offsets.
    paths: Vec<(crate::Span, Vec<u32>)>,
}

/// Owned build state: each step consumes and returns it, so no caller-owned
/// value is mutated through a borrowed capability.
struct Emitter {
    builder: Builder,
    meter: crate::Meter,
}

impl Emitter {
    fn append(mut self, bytes: &[u8]) -> Result<Self, crate::Diagnostic> {
        let builder = &mut self.builder;
        let end = attempt!(crate::index(
            builder.source.len().saturating_add(bytes.len()),
            builder.tree.span
        ));
        if end >= self.meter.limits.bytes {
            return Err(super::exhausted(
                builder.tree.span,
                "editor source byte limit exceeded",
            ));
        }
        attempt!(self.meter.charge(
            attempt!(crate::index(bytes.len(), builder.tree.span)),
            builder.tree.span
        ));
        builder.source.extend_from_slice(bytes);
        builder.source.push(b' ');
        Ok(self)
    }

    fn body(mut self, nodes: &[Node]) -> Result<(Self, Vec<u32>), crate::Diagnostic> {
        struct Frame<'a> {
            nodes: &'a [Node],
            next: usize,
            index: u32,
            start: u32,
            depth: u32,
            value_start: usize,
        }
        // The first frame is the node list itself rather than an editor node, so
        // an editor path is the per-level node indexes below it.
        fn ancestors(frames: &[Frame<'_>]) -> Vec<u32> {
            let mut path = Vec::with_capacity(frames.len());
            for frame in frames.iter().skip(1) {
                path.push(frame.index);
            }
            path
        }
        let span = self.builder.tree.span;
        attempt!(self.meter.depth(0, span));
        let mut frames = alloc::vec![Frame {
            nodes,
            next: 0,
            index: 0,
            start: 0,
            depth: 0,
            value_start: 0,
        }];
        let mut values = Vec::new();
        loop {
            let Some(frame) = frames.last_mut() else {
                return Err(crate::internal(span));
            };
            if frame.next == frame.nodes.len() {
                let Some(frame) = frames.pop() else {
                    return Err(crate::internal(span));
                };
                let mut quotation_path = ancestors(frames.as_slice());
                quotation_path.push(frame.index);
                let Some(parent) = frames.last_mut() else {
                    return Ok((self, values));
                };
                self = attempt!(self.append(b"]"));
                let id = attempt!(crate::index(self.builder.tree.nodes.len(), span));
                let end = attempt!(crate::index(self.builder.source.len(), span));
                let quotation_span = crate::Span {
                    start: frame.start,
                    end,
                };
                self.builder.paths.push((quotation_span, quotation_path));
                self.builder.tree.nodes.push(super::Node {
                    kind: super::Kind::Quotation(values.split_off(frame.value_start)),
                    span: quotation_span,
                });
                let Some(parent_items) = values.len().checked_sub(parent.value_start) else {
                    return Err(crate::internal(span));
                };
                if parent_items >= parent.nodes.len() {
                    return Err(super::exhausted(span, "editor node limit exceeded"));
                }
                if values.len() >= self.builder.tree.nodes.len() {
                    return Err(crate::internal(span));
                }
                values.push(id);
                continue;
            }
            let node = &frame.nodes[frame.next];
            frame.next += 1;
            attempt!(self.meter.node(self.builder.tree.span));
            let start = attempt!(crate::index(
                self.builder.source.len(),
                self.builder.tree.span
            ));
            let kind = match node {
                Node::Integer(value) => {
                    let text = alloc::format!("{value}");
                    self = attempt!(self.append(text.as_bytes()));
                    super::Kind::Literal(noble_kernel::untrusted::Lit::I64(*value))
                }
                Node::Boolean(value) => {
                    self = attempt!(self.append(if *value { b"true" } else { b"false" }));
                    super::Kind::Literal(noble_kernel::untrusted::Lit::Bool(*value))
                }
                Node::Word(word) => {
                    let span = crate::Span { start, end: start };
                    if !matches!(
                        attempt!(super::lexer::tokens::classify(
                            word.as_bytes(),
                            span,
                            &mut self.meter,
                            false
                        )),
                        super::lexer::TokenKind::Word(_)
                    ) {
                        return Err(crate::invalid(
                            span,
                            "editor word is not source word syntax",
                        ));
                    }
                    self = attempt!(self.append(word.as_bytes()));
                    super::Kind::Word(word.as_bytes().to_vec())
                }
                Node::Hole => {
                    self.builder.holes += 1;
                    self = attempt!(self.append(b"@editor-hole"));
                    super::Kind::EditorHole
                }
                Node::Quotation(children) => {
                    let depth = frame.depth.saturating_add(1);
                    let index = frame.next.saturating_sub(1) as u32;
                    self = attempt!(self.append(b"["));
                    attempt!(self.meter.depth(depth, span));
                    frames.push(Frame {
                        nodes: children,
                        next: 0,
                        index,
                        start,
                        depth,
                        value_start: values.len(),
                    });
                    continue;
                }
            };
            let id = attempt!(crate::index(
                self.builder.tree.nodes.len(),
                self.builder.tree.span
            ));
            let end = attempt!(crate::index(
                self.builder.source.len(),
                self.builder.tree.span
            ));
            // The node budget is charged above, and each input contributes at
            // most one body entry; do not reserve for rejected candidates.
            let Some(body_items) = values.len().checked_sub(frame.value_start) else {
                return Err(crate::internal(span));
            };
            if body_items >= frame.nodes.len() {
                return Err(super::exhausted(
                    self.builder.tree.span,
                    "editor node limit exceeded",
                ));
            }
            if matches!(kind, super::Kind::EditorHole) && self.builder.first_hole.is_none() {
                self.builder.first_hole = Some(crate::Span { start, end });
            }
            let own_index = frame.next.saturating_sub(1) as u32;
            let mut leaf_path = ancestors(frames.as_slice());
            leaf_path.push(own_index);
            self.builder
                .paths
                .push((crate::Span { start, end }, leaf_path));
            self.builder.tree.nodes.push(super::Node {
                kind,
                span: crate::Span { start, end },
            });
            if values.len() >= self.builder.tree.nodes.len() {
                return Err(crate::internal(span));
            }
            values.push(id);
        }
    }
}

impl Candidate {
    fn build(&self, limits: crate::Limits) -> Result<(Builder, crate::Meter), super::Error> {
        let span = crate::Span { start: 0, end: 0 };
        if self.format != FORMAT {
            return Err(super::Error::at(
                super::Stage::Parse,
                crate::Diagnostic::new(
                    crate::DiagnosticKind::Unsupported,
                    span,
                    "unsupported editor format",
                ),
            ));
        }
        let emitter = Emitter {
            builder: Builder {
                tree: super::Tree {
                    nodes: Vec::new(),
                    body: Vec::new(),
                    span,
                },
                source: Vec::new(),
                holes: 0,
                first_hole: None,
                paths: Vec::new(),
            },
            meter: crate::Meter::new(limits),
        };
        let (Emitter { mut builder, meter }, body) = attempt!(emitter
            .body(&self.nodes)
            .map_err(|error| super::Error::at(super::Stage::Parse, error)));
        builder.tree.body = body;
        builder.tree.span.end = attempt!(crate::index(builder.source.len(), span)
            .map_err(|error| super::Error::at(super::Stage::Parse, error)));
        Ok((builder, meter))
    }

    /// The generated-span to editor-node-path index for this candidate.
    ///
    /// These spans are offsets into the synthetic editor source, never into a
    /// `.noble` file or the caller's JSON. The mapping is rebuilt from the same
    /// emitter that produced the source, so a span the emitter never produced
    /// has no path and is reported as unmapped rather than guessed.
    pub fn editor_paths(
        &self,
        limits: crate::Limits,
    ) -> Result<Vec<(crate::Span, Vec<u32>)>, super::Error> {
        let (builder, _) = attempt!(self.build(limits));
        Ok(builder.paths)
    }

    /// Only a hole-free tree can become source bytes. The ordinary source
    /// parser, inference, kernel and compiler remain the sole admission route.
    pub fn admitted_source(&self, limits: crate::Limits) -> Result<Vec<u8>, super::Error> {
        let (builder, _) = attempt!(self.build(limits));
        if builder.holes != 0 {
            // Located at the first hole's own generated span, not at the whole tree.
            let span = builder.first_hole.unwrap_or(builder.tree.span);
            return Err(super::Error::at(
                super::Stage::Acceptance,
                crate::invalid(span, "editor hole cannot enter source admission"),
            ));
        }
        Ok(builder.source)
    }
}

impl super::Session {
    /// Analyze incomplete editor syntax against the *same* namespace and word
    /// schemes as source preparation, without emitting or admitting a candidate.
    pub fn analyze_editor(
        &self,
        candidate: &Candidate,
        inputs: &[noble_kernel::types::Ty],
        limits: crate::Limits,
    ) -> Result<Analysis, super::Error> {
        self.analyze_editor_metered(candidate, inputs, limits)
            .map(|(analysis, _)| analysis)
    }

    /// Analyze an editor tree and report its consumed frontend work so a
    /// caller-owned bounded service can debit the same cumulative budget.
    pub(crate) fn analyze_editor_metered(
        &self,
        candidate: &Candidate,
        inputs: &[noble_kernel::types::Ty],
        limits: crate::Limits,
    ) -> Result<(Analysis, u32), super::Error> {
        let (mut builder, mut meter) = attempt!(candidate.build(limits));
        let result = (|| {
            if let Err(error) =
                super::resolution::resolve(&mut builder.tree, None, self, false, &mut meter)
            {
                return Err(super::Error::at(super::Stage::Resolve, error));
            }
            if let Err(error) = super::preflight::check(inputs, self, builder.tree.span, &mut meter)
            {
                return Err(super::Error::at(super::Stage::Check, error));
            }
            let environment = attempt!(self
                .environment()
                .map_err(|error| super::Error::at(super::Stage::Check, error)));
            let scope = super::inference::Scope {
                root: &builder.tree,
                session: self,
                environment: &environment,
                root_signature: None,
                prospective_owner: None,
            };
            let state = attempt!(super::inference::infer(
                scope,
                super::inference::Mode::EditorAnalysis,
                inputs,
                &mut meter
            )
            .map_err(|error| super::Error::at(super::Stage::Check, error)));
            let mut holes = Vec::with_capacity(state.holes.len());
            for hole in &state.holes {
                let input_stack = attempt!(state
                    .arena
                    .describe_stack(hole.input, hole.span, &mut meter)
                    .map_err(|error| super::Error::at(super::Stage::Check, error)));
                let output_stack = attempt!(state
                    .arena
                    .describe_stack(hole.output, hole.span, &mut meter)
                    .map_err(|error| super::Error::at(super::Stage::Check, error)));
                let incoming = attempt!(state
                    .arena
                    .editor_stack_suffix(hole.input, hole.span, &mut meter)
                    .map_err(|error| super::Error::at(super::Stage::Check, error)));
                let outgoing = attempt!(state
                    .arena
                    .editor_stack_suffix(hole.output, hole.span, &mut meter)
                    .map_err(|error| super::Error::at(super::Stage::Check, error)));
                let common = incoming
                    .iter()
                    .zip(&outgoing)
                    .take_while(|(left, right)| left == right)
                    .count();
                let required_output_suffix = outgoing[common..].to_vec();
                let (known_effects, unresolved_effect) = attempt!(state
                    .arena
                    .editor_effect(hole.effect, hole.span, &mut meter)
                    .map_err(|error| super::Error::at(super::Stage::Check, error)));
                holes.push(HoleConstraint {
                    input_stack,
                    output_stack,
                    required_output_suffix,
                    known_effects,
                    unresolved_effect,
                });
            }
            let root = match state.bodies.first() {
                Some(root) => root,
                None => {
                    return Err(super::Error::at(
                        super::Stage::Check,
                        crate::internal(builder.tree.span),
                    ))
                }
            };
            let (known_effects, unresolved_effect) = attempt!(state
                .arena
                .editor_effect(root.effect, root.span, &mut meter)
                .map_err(|error| super::Error::at(super::Stage::Check, error)));
            Ok(Analysis {
                holes,
                known_effects,
                unresolved_effect,
            })
        })();
        let work_used = meter.spent();
        match result {
            Ok(analysis) => Ok((analysis, work_used)),
            Err(mut error) => {
                error.work_used = work_used;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_spans_map_to_bounded_node_paths() {
        let direct = Candidate {
            format: FORMAT,
            nodes: alloc::vec![Node::Integer(1), Node::Hole, Node::Word(String::from("+"))],
        };
        let paths = direct
            .editor_paths(crate::Limits::default())
            .expect("direct mapping");
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].1, alloc::vec![0]);
        assert_eq!(paths[1].1, alloc::vec![1]);
        assert_eq!(paths[2].1, alloc::vec![2]);

        let nested = Candidate {
            format: FORMAT,
            nodes: alloc::vec![Node::Quotation(alloc::vec![
                Node::Hole,
                Node::Quotation(alloc::vec![Node::Hole]),
            ])],
        };
        let paths = nested
            .editor_paths(crate::Limits::default())
            .expect("nested mapping");
        assert_eq!(paths.len(), 4);
        // Entries are recorded when each node is emitted, so a quotation appears
        // after its children. Repeated indices name the outer and inner
        // quotation levels rather than collapsing them.
        assert_eq!(paths[0].1, alloc::vec![0, 0]);
        assert_eq!(paths[1].1, alloc::vec![0, 1, 0]);
        assert_eq!(paths[2].1, alloc::vec![0, 1]);
        assert_eq!(paths[3].1, alloc::vec![0]);
        for (span, path) in &paths {
            assert!(span.start <= span.end);
            assert!(!path.is_empty());
        }
    }

    #[test]
    fn repeated_and_unicode_words_keep_their_own_generated_spans() {
        // Issue #63 span mapping: every emitted node keeps its own generated span and its own bounded
        // path, so two identical words never share an address and a repeated word is addressed by
        // position rather than by text.
        let repeated = Candidate {
            format: FORMAT,
            nodes: alloc::vec![
                Node::Word(String::from("+")),
                Node::Word(String::from("+")),
                Node::Quotation(alloc::vec![Node::Word(String::from("+"))]),
                Node::Word(String::from("+")),
            ],
        };
        let paths = repeated
            .editor_paths(crate::Limits::default())
            .expect("repeated mapping");
        assert_eq!(paths.len(), 5, "four words and one quotation");
        let direct_paths: alloc::vec::Vec<_> =
            paths.iter().take(3).map(|(_, path)| path.clone()).collect();
        assert_eq!(
            direct_paths,
            alloc::vec![alloc::vec![0], alloc::vec![1], alloc::vec![2, 0]],
            "repeated words keep distinct paths rather than collapsing into one"
        );
        assert_eq!(paths[3].1, alloc::vec![2], "the quotation's own path");
        assert_eq!(paths[4].1, alloc::vec![3], "the trailing word's own path");
        let mut spans: alloc::vec::Vec<_> = paths.iter().map(|(span, _)| *span).collect();
        spans.sort_by_key(|span| span.start);
        spans.dedup();
        assert_eq!(spans.len(), paths.len(), "no two nodes share a span");

        // Spans are byte ranges of the generated source rather than character counts, and each
        // word's span is exactly its own bytes: a hole-free tree lets the same emitter's bytes be
        // read back at a word's span.
        let source = Candidate {
            format: FORMAT,
            nodes: alloc::vec![Node::Integer(10), Node::Word(String::from("+"))],
        };
        let bytes = source
            .admitted_source(crate::Limits::default())
            .expect("a hole-free tree becomes source");
        let paths = source
            .editor_paths(crate::Limits::default())
            .expect("word mapping");
        let word_span = paths[1].0;
        let start = usize::try_from(word_span.start).expect("bounded span");
        let end = usize::try_from(word_span.end).expect("bounded span");
        // The emitted span covers the node and the separator the emitter writes after it, so the
        // bytes it names start with the word itself rather than being an assumed character count.
        assert!(
            bytes[start..end].starts_with("+".as_bytes()),
            "the span names the word's own bytes first"
        );
        assert_eq!(
            start, 3,
            "the two-byte integer and its separator precede the word"
        );

        // Unicode coverage is a refusal rather than a silent transliteration: the editor word
        // grammar admits ASCII words only, so a non-ASCII word is refused at the parse stage at the
        // refused node's own bounded span instead of being mapped or rewritten.
        let unicode = Candidate {
            format: FORMAT,
            nodes: alloc::vec![Node::Integer(10), Node::Word(String::from("λ"))],
        };
        let refusal = unicode
            .editor_paths(crate::Limits::default())
            .expect_err("a non-ASCII word is refused, never transliterated");
        assert_eq!(refusal.stage(), super::super::Stage::Parse);
        assert_eq!(refusal.diagnostic().kind.category(), "invalid");
        assert_eq!(
            refusal.diagnostic().span.start,
            refusal.diagnostic().span.end
        );
    }

    #[test]
    fn a_hole_admission_refusal_is_located_at_the_first_hole() {
        // Issue #63: the refusal used to carry the whole tree's span, so a nested hole reported the
        // outer quotation as its address. It now carries the first hole's own generated span, and the
        // later hole is not silently chosen instead.
        let candidate = Candidate {
            format: FORMAT,
            nodes: alloc::vec![Node::Quotation(alloc::vec![
                Node::Integer(1),
                Node::Quotation(alloc::vec![Node::Hole]),
            ])],
        };
        let paths = candidate
            .editor_paths(crate::Limits::default())
            .expect("nested paths");
        let error = candidate
            .admitted_source(crate::Limits::default())
            .expect_err("a hole cannot enter admission");
        assert_eq!(error.stage(), super::super::Stage::Acceptance);
        assert_eq!(error.diagnostic().kind.category(), "invalid");
        let (span, path) = paths
            .iter()
            .find(|(span, _)| *span == error.diagnostic().span)
            .expect("the refusal names a node the emitter produced");
        assert_eq!(*span, error.diagnostic().span);
        assert_eq!(*path, alloc::vec![0, 1, 0], "the inner hole's own path");

        let trailing = Candidate {
            format: FORMAT,
            nodes: alloc::vec![
                Node::Integer(1),
                Node::Quotation(alloc::vec![Node::Hole]),
                Node::Hole,
            ],
        };
        let paths = trailing
            .editor_paths(crate::Limits::default())
            .expect("first-hole paths");
        let error = trailing
            .admitted_source(crate::Limits::default())
            .expect_err("a hole cannot enter admission");
        let (_, path) = paths
            .iter()
            .find(|(span, _)| *span == error.diagnostic().span)
            .expect("the first hole is the one named");
        assert_eq!(*path, alloc::vec![1, 0], "the first hole, not the last");
    }

    #[test]
    fn deep_quotation_emission_respects_configured_depth_without_call_stack_recursion() {
        let mut node = Node::Integer(7);
        for _ in 0..512 {
            node = Node::Quotation(alloc::vec![node]);
        }
        let candidate = Candidate {
            format: FORMAT,
            nodes: alloc::vec![node],
        };
        let mut limits = crate::Limits {
            depth: 512,
            ..crate::Limits::default()
        };
        let source = candidate
            .admitted_source(limits)
            .expect("iterative quotation emission");
        assert_eq!(source.iter().filter(|&&byte| byte == b'[').count(), 512);
        assert_eq!(source.iter().filter(|&&byte| byte == b']').count(), 512);
        assert!(source.starts_with(b"[ [ "));
        assert!(source.ends_with(b"] ] "));

        limits.depth = 511;
        let error = candidate.admitted_source(limits).expect_err("depth limit");
        assert_eq!(error.stage(), super::super::Stage::Parse);
        assert_eq!(error.diagnostic().message, "frontend depth limit exceeded");
        // A budget refusal stays an exhaustion rather than collapsing into a
        // generic invalid input.
        assert_eq!(error.diagnostic().kind.category(), "exhausted");
    }
}
