# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the Aspen project.

ADRs record significant architectural decisions along with their context, rationale,
and consequences. They provide a trail of reasoning for anyone who joins the project
and needs to understand why things are the way they are.

## Format

Each ADR follows this structure:

```
# NNN. Title

**Status:** accepted | superseded | deprecated

## Context

Background, constraints, and forces driving the decision.

## Decision

What was decided and why.

## Consequences

What changed as a result — both positive and negative.
```

When listing alternatives considered, annotate each aspect:

- `(+)` positive
- `(~)` neutral
- `(-)` negative

## Numbering

Files are named `NNN-title.md` with a zero-padded three-digit prefix.
Numbers are sequential and never reused. A superseded ADR keeps its
number; the superseding ADR references it.

## When to Write an ADR

Write an ADR when:

- Introducing a new architectural pattern or technology
- Choosing between alternatives that affect multiple crates
- Changing a fundamental project constraint

Don't write an ADR for routine implementation choices that affect a single module.

## References

- [ADR GitHub](https://adr.github.io/)
- [Michael Nygard's original article](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
