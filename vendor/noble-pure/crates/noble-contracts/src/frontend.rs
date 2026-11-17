#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; frontend orchestration mutates only its preparation-owned work meter and fresh predicate arena; disjoint frontend/kernel work reservations and checked node/type limits bound resolution, while bindings and definitions follow parsed child lists; borrowed source, syntax, and retained ordinary acceptance remain unchanged."
)]

mod bindings;
mod fields;
mod logic;

#[derive(Clone, Copy)]
struct Container<'a> {
    source: &'a [u8],
    tree: &'a crate::syntax::Tree,
    children: &'a [u32],
    span: crate::Span,
}

impl<'a> Container<'a> {
    fn new(
        source: &'a [u8],
        tree: &'a crate::syntax::Tree,
        root: u32,
    ) -> Result<Self, crate::Diagnostic> {
        let span = attempt!(tree.node(root)).span;
        let children = attempt!(tree.round(root));
        Ok(Self {
            source,
            tree,
            children,
            span,
        })
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; hostile source and ordinary typing fail through diagnostics, not assertions. floor(work / 2) is at most work, so the disjoint frontend remainder cannot underflow."
)]
pub(crate) fn prepare(
    source: &[u8],
    limits: crate::Limits,
) -> Result<crate::Prepared, crate::Diagnostic> {
    let mut meter = crate::Meter::new(limits);
    // Disjoint budgets bound the total frontend plus kernel work; acceptance
    // does not expose its consumed meter, so its reservation cannot be reused.
    let kernel_work = limits.work / 2;
    meter.work = limits.work - kernel_work;
    let tree = attempt!(crate::syntax::parse(source, &mut meter));
    let container = attempt!(Container::new(source, &tree, tree.root));
    let name = attempt!(header(container, &mut meter));
    let parts = attempt!(fields::collect(container, &mut meter));
    let inputs = attempt!(bindings::named(
        attempt!(Container::new(
            source,
            &tree,
            attempt!(required(parts.input, container.span, "missing input field"))
        )),
        &mut meter
    ));
    let outputs = attempt!(bindings::named(
        attempt!(Container::new(
            source,
            &tree,
            attempt!(required(
                parts.output,
                container.span,
                "missing output field"
            ))
        )),
        &mut meter
    ));
    let input_types = attempt!(bindings::types(&inputs, container.span, &mut meter));
    let output_types = attempt!(bindings::types(&outputs, container.span, &mut meter));
    let program_field = attempt!(required(
        parts.program,
        container.span,
        "missing program field"
    ));
    let program = attempt!(argument(&tree, program_field));
    let context = crate::predicate::Context {
        source,
        tree: &tree,
        inputs: &inputs,
        outputs: &outputs,
        params: &[],
        definitions: &[],
    };
    let (candidate, request, checked) = attempt!(ordinary(
        container,
        program,
        (input_types, output_types),
        kernel_work,
        &mut meter
    ));
    let logic = match logic::resolve(context, parts, &mut meter) {
        Ok(logic) => logic,
        Err(diagnostic) => return Err(diagnostic.with_typing(checked)),
    };
    Ok(crate::Prepared {
        name,
        inputs,
        outputs,
        params: logic.params,
        definitions: logic.definitions,
        expressions: logic.expressions,
        requires: logic.requires,
        ensures: logic.ensures,
        candidate,
        request,
        checked,
    })
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; header checks return source diagnostics and identifier decoding allocates an owned String, which is not a const operation."
)]
fn header(
    container: Container<'_>,
    meter: &mut crate::Meter,
) -> Result<alloc::string::String, crate::Diagnostic> {
    let head = attempt!(container.tree.atom(
        container.source,
        attempt!(crate::syntax::child(container.children, 0, container.span))
    ));
    if head != b"contract" {
        return Err(crate::invalid(
            container.span,
            "expected a versioned contract container",
        ));
    }
    let version_id = attempt!(crate::syntax::child(container.children, 1, container.span));
    let version_span = attempt!(container.tree.node(version_id)).span;
    match attempt!(crate::syntax::integer(
        attempt!(container.tree.atom(container.source, version_id)),
        version_span,
        meter
    )) {
        Some(1) => {}
        Some(_) => {
            return Err(crate::Diagnostic::new(
                crate::DiagnosticKind::Unsupported,
                version_span,
                "unsupported contract format revision",
            ))
        }
        None => {
            return Err(crate::invalid(
                version_span,
                "contract revision must be a decimal integer",
            ))
        }
    }
    crate::syntax::identifier(
        container.source,
        container.tree,
        attempt!(crate::syntax::child(container.children, 2, container.span)),
        meter,
    )
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; this sole preparation boundary allocates the candidate and returns checker failures. The syntax-node ID and disjoint kernel-work budget are named, nonadjacent arguments, not interchangeable quantities."
)]
fn ordinary(
    container: Container<'_>,
    body: u32,
    stacks: (
        alloc::vec::Vec<noble_kernel::types::Ty>,
        alloc::vec::Vec<noble_kernel::types::Ty>,
    ),
    kernel_work: u32,
    meter: &mut crate::Meter,
) -> Result<
    (
        noble_kernel::untrusted::Candidate,
        noble_kernel::untrusted::Request,
        noble_kernel::untrusted::Checked,
    ),
    crate::Diagnostic,
> {
    let span = container.span;
    let env = match noble_kernel::contracts::environment() {
        Ok(env) => env,
        Err(_) => return Err(crate::internal(span)),
    };
    let (input_types, output_types) = stacks;
    let (candidate, node_spans) = attempt!(crate::program::resolve(
        crate::program::Context {
            source: container.source,
            tree: container.tree,
            env: &env
        },
        body,
        &input_types,
        &output_types,
        meter
    ));
    let request = noble_kernel::untrusted::Request {
        input_bytes: attempt!(crate::index(container.source.len(), span)),
        expected: noble_kernel::untrusted::Expected {
            stack_in: input_types,
            stack_out: output_types,
            allowed_effects: noble_kernel::types::EffSet::empty(),
        },
        limits: noble_kernel::untrusted::Limits {
            bytes: meter.limits.bytes,
            nodes: meter.limits.nodes,
            depth: meter.limits.depth,
            type_size: crate::syntax::TYPE_CAP,
            stack_height: crate::inference::STACK_CAP,
            work: kernel_work,
            diagnostics: 32,
        },
    };
    // Witness inference is not an acceptance decision. Only this existing
    // kernel check can produce the retained ordinary typing result.
    let checked = attempt!(crate::program::check(
        &env,
        &candidate,
        &request,
        &node_spans,
        attempt!(container.tree.node(body)).span
    ));
    Ok((candidate, request, checked))
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; a missing field constructs an owned diagnostic String and cannot be const."
)]
fn required(
    value: Option<u32>,
    span: crate::Span,
    message: &str,
) -> Result<u32, crate::Diagnostic> {
    match value {
        Some(value) => Ok(value),
        None => Err(crate::invalid(span, message)),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; syntax lookup and arity failures construct owned diagnostics through non-const parser methods."
)]
fn argument(tree: &crate::syntax::Tree, id: u32) -> Result<u32, crate::Diagnostic> {
    let span = attempt!(tree.node(id)).span;
    let items = attempt!(tree.round(id));
    if items.len() != 2 {
        return Err(crate::invalid(
            span,
            "contract field requires exactly one argument",
        ));
    }
    crate::syntax::child(items, 1, span)
}
