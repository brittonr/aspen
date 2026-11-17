//! Parse one logical S-expression from a scanner-selected bracketed body.
//! This syntax is untrusted; independent proof/type checking happens later.

#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; this stage imports the domain types it operates on directly, matching the module vocabulary"
)]
pub(super) fn parse(
    tokens: &[crate::source::lexer::LogicalToken],
    span: crate::Span,
) -> Result<crate::intrinsic::Form, crate::Diagnostic> {
    use crate::source::lexer::LogicalKind;
    let Some(_) = tokens.first() else {
        return Err(crate::invalid(
            span,
            "logical body must contain exactly one form",
        ));
    };
    let mut frames: alloc::vec::Vec<(crate::Span, usize)> = alloc::vec::Vec::new();
    let mut values: alloc::vec::Vec<crate::intrinsic::Form> = alloc::vec::Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        let form = match &token.kind {
            LogicalKind::OpenParen => {
                if frames.len() >= 64 {
                    return Err(crate::invalid(
                        token.span,
                        "logical nesting exceeds supported depth",
                    ));
                }
                frames.push((token.span, values.len()));
                continue;
            }
            LogicalKind::CloseParen => {
                let Some((opening, start)) = frames.pop() else {
                    return Err(crate::invalid(token.span, "unmatched logical parenthesis"));
                };
                crate::intrinsic::Form {
                    kind: crate::intrinsic::FormKind::List(values.split_off(start)),
                    span: crate::Span {
                        start: opening.start,
                        end: token.span.end,
                    },
                }
            }
            LogicalKind::Atom(bytes) => {
                let text = attempt!(core::str::from_utf8(bytes)
                    .map_err(|_| crate::invalid(token.span, "invalid logical UTF-8 atom")));
                crate::intrinsic::Form {
                    kind: crate::intrinsic::FormKind::Atom(alloc::string::String::from(text)),
                    span: token.span,
                }
            }
            LogicalKind::Colon | LogicalKind::Comma => {
                let spelling = if matches!(&token.kind, LogicalKind::Colon) {
                    ":"
                } else {
                    ","
                };
                crate::intrinsic::Form {
                    kind: crate::intrinsic::FormKind::Atom(alloc::string::String::from(spelling)),
                    span: token.span,
                }
            }
        };
        let next = attempt!(at.checked_add(1).ok_or_else(|| {
            crate::invalid(token.span, "logical token position exceeds address space")
        }));
        if frames.is_empty() && next == tokens.len() {
            return Ok(form);
        }
        if frames.is_empty() {
            return Err(crate::invalid(
                tokens[next].span,
                "logical body contains a trailing form",
            ));
        }
        if values.len() >= tokens.len() {
            return Err(crate::invalid(
                token.span,
                "logical list exceeds its token body",
            ));
        }
        values.push(form);
    }
    Err(crate::invalid(
        frames.last().map_or(span, |frame| frame.0),
        "unclosed logical parenthesis",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    use crate::source::lexer::{LogicalKind, LogicalToken};

    #[test]
    fn nested_form_preserves_order_and_rejects_excess_depth() {
        let span = crate::Span { start: 0, end: 1 };
        let tokens = [
            LogicalToken {
                kind: LogicalKind::OpenParen,
                span,
            },
            LogicalToken {
                kind: LogicalKind::Atom(alloc::vec![b'a']),
                span,
            },
            LogicalToken {
                kind: LogicalKind::OpenParen,
                span,
            },
            LogicalToken {
                kind: LogicalKind::Atom(alloc::vec![b'b']),
                span,
            },
            LogicalToken {
                kind: LogicalKind::CloseParen,
                span,
            },
            LogicalToken {
                kind: LogicalKind::CloseParen,
                span,
            },
        ];
        let form = parse(&tokens, span).expect("nested form");
        let crate::intrinsic::FormKind::List(items) = form.kind else {
            panic!("expected outer list");
        };
        let crate::intrinsic::FormKind::Atom(first) = &items[0].kind else {
            panic!("expected first atom");
        };
        assert_eq!(first, "a");
        let crate::intrinsic::FormKind::List(inner) = &items[1].kind else {
            panic!("expected inner list");
        };
        let crate::intrinsic::FormKind::Atom(second) = &inner[0].kind else {
            panic!("expected nested atom");
        };
        assert_eq!(second, "b");

        let deep: alloc::vec::Vec<_> = (0..65)
            .map(|_| LogicalToken {
                kind: LogicalKind::OpenParen,
                span,
            })
            .collect();
        let error = parse(&deep, span).expect_err("nesting bound");
        assert_eq!(error.message, "logical nesting exceeds supported depth");
    }
}
