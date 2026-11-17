//! `ngraph-llvm-i64/0`: the opt-in lowering edge from an admitted total-`I64` graph to one
//! straight-line, memory-free, call-free LLVM IR function, and an independent validator of the
//! emitted text (issue #107).
//!
//! The emitter and the validator are separate code paths on purpose. [`emit`] renders the module
//! once; [`parse`] tokenizes the actual bytes a caller supplies and [`verify`] maps the parsed
//! instructions back onto the checker's admitted graph. Nothing here re-emits a module and compares
//! it with itself. Semantic soundness of an accepted instruction stream is a Lean theorem over the
//! parsed form (`NobleFoundation.LlvmLowering.validate_sound`); the parser-to-model step and the
//! correspondence between this Rust validator and that Lean model are **not** proved by this
//! module.
//!
//! The profile is deliberately narrow: exactly two `I64` input slots and one output, flat shapes, no
//! facts, no guards, no `i64.eq`, no parameters and no `F32`. The arity is two because the only
//! consumer of these modules, compiler-lab's `llvm-pair-v1` lane, reads the two-`i64`-to-`i64`
//! shape only; a wider interface would widen the ABI claim without a consumer. This module is
//! opt-in: the selected production backend ([`crate::compiled`], [`crate::schedule`]) is unchanged,
//! and adopting the lowering at run time needs its own decision.

/// Lowering profile name, written into the module header and every retained binding.
pub const PROFILE: &str = "ngraph-llvm-i64/0";
/// The only target this profile binds. The module text is target-specific: the data layout and the
/// triple are part of the artifact, not defaults.
pub const TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";
/// The `x86_64-unknown-linux-gnu` data layout as LLVM 18 and later report it.
///
/// Recorded as an assumption: no pinned LLVM exists on this host to query, so compiler-lab owns
/// confirming it against the LLVM it builds the pair with.
pub const DATA_LAYOUT: &str =
    "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128";
/// The single defined function's name.
pub const FUNCTION: &str = "noble_i64";
/// The calling convention. `ccc` is the default and is therefore absent from the text; a module
/// that names any other convention is refused.
pub const CALLING_CONVENTION: &str = "ccc";
/// Interface width of the profile: exactly this many `I64` input slots.
pub const PARAMETERS: u32 = 2;
/// Ordered output width of the profile: exactly one output.
pub const OUTPUTS: u32 = 1;

/// The module's first line. The validator requires it verbatim, so a module from another profile is
/// refused rather than reinterpreted.
pub const HEADER_LINE: &str =
    "; ngraph-llvm-i64/0 module: one straight-line function, no memory, no calls";

/// Why a graph or module is outside this profile. Every variant names the check that refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The graph's sub-profile is not `ngraph-i64/0`; the refusal carries the observed name.
    Profile(&'static str),
    /// The target lines or their pinned module-preamble positions differ.
    Target,
    /// The graph's interface is not exactly two `I64` input slots, or the module's `define` line is
    /// not the pinned signature (return type, name, parameter names and types, attributes).
    Interface,
    /// The graph declares a `Param` slot, which only `ngraph-f32/0` has.
    Param,
    /// An interface or node shape contains a non-scalar (compound) shape.
    Shape,
    /// The graph carries a fact record.
    Fact,
    /// A node is a `guard` root.
    Guard,
    /// A node is `i64.eq`, which has no `I64` instruction in this profile.
    Equality,
    /// A node is outside the operation inventory, including every `F32` operation.
    Operation(&'static str),
    /// The graph does not select exactly one output, or the `ret` operand does not denote it.
    Output,
    /// The first line is not this profile's header comment.
    Header,
    /// An instruction line is not `  %v<n> = <op> i64 <a>, <b>`, or its destination does not
    /// strictly increase.
    Instruction,
    /// The opcode is not one of `add`, `sub` and `mul`.
    Opcode,
    /// An overflow or poison flag (`nsw`, `nuw`, `exact`, `samesign`, `disjoint`, `nneg`) is
    /// present. This profile has no flagged arithmetic.
    Flags,
    /// An operand is not a parameter, a `%v<n>` value or a canonical signed `i64` literal.
    Operand,
    /// The body does not end in exactly one `ret` before the closing brace, or holds a second one.
    Return,
    /// An instruction's destination is not the node at its position: an arithmetic node has no
    /// instruction, or an instruction defines a parameter or a literal.
    NodeMap,
    /// A parsed operand does not denote the node the admitted graph supplies there.
    Operands,
}

impl Refusal {
    /// Stable diagnostic name, as the lane's documents list it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Profile(_) => "L-PROFILE",
            Self::Target => "L-TARGET",
            Self::Interface => "L-INTERFACE",
            Self::Param => "L-PARAM",
            Self::Shape => "L-SHAPE",
            Self::Fact => "L-FACT",
            Self::Guard => "L-GUARD",
            Self::Equality => "L-EQUALITY",
            Self::Operation(_) => "L-OPERATION",
            Self::Output => "L-OUTPUT",
            Self::Header => "L-HEADER",
            Self::Instruction => "L-INSTRUCTION",
            Self::Opcode => "L-OPCODE",
            Self::Flags => "L-FLAGS",
            Self::Operand => "L-OPERAND",
            Self::Return => "L-RETURN",
            Self::NodeMap => "L-NODEMAP",
            Self::Operands => "L-OPERANDS",
        }
    }
}

/// One operand exactly as the module text spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    /// `%a<slot>`: a function argument.
    Parameter(u32),
    /// A canonical signed decimal `i64` literal, as its exact bits.
    Constant(i64),
    /// `%v<node>`: the value of the node with that index.
    Value(u32),
}

/// The three arithmetic opcodes the profile emits, never with a flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    /// Wrapping addition.
    Add,
    /// Wrapping subtraction.
    Sub,
    /// Wrapping multiplication.
    Mul,
}

impl Opcode {
    /// The opcode word the text spells.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
        }
    }
}

/// One parsed instruction: the node whose value the destination SSA name carries, its opcode and its
/// ordered operands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    /// The node the destination SSA name denotes.
    pub node: u32,
    /// The instruction's opcode.
    pub opcode: Opcode,
    /// The ordered operands, left first.
    pub operands: [Operand; 2],
}

/// A module that [`parse`] accepted, with the instructions and return operand it read.
///
/// These are the parsed artifact, not the graph: [`verify`] is what binds them to a graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    /// The instructions in text order.
    pub instructions: alloc::vec::Vec<Instruction>,
    /// The single `ret` operand.
    pub output: Operand,
}

/// Flag words no instruction in this profile may carry. LLVM attaches these to the opcode or to the
/// type, so their presence is refused before any operand is read.
const FLAGS: [&str; 6] = ["nsw", "nuw", "exact", "samesign", "disjoint", "nneg"];

const BLANK: &str = "";
const ENTRY: &str = "entry:";
const CLOSE: &str = "}";
const INDENT: &str = "  ";
const RET: &str = "ret";
const TYPE: &str = "i64";

fn is_arith(op: &crate::ir::Op) -> bool {
    matches!(
        op,
        crate::ir::Op::I64Add(..) | crate::ir::Op::I64Sub(..) | crate::ir::Op::I64Mul(..)
    )
}

fn opcode(op: &crate::ir::Op) -> Option<Opcode> {
    match op {
        crate::ir::Op::I64Add(..) => Some(Opcode::Add),
        crate::ir::Op::I64Sub(..) => Some(Opcode::Sub),
        crate::ir::Op::I64Mul(..) => Some(Opcode::Mul),
        _ => None,
    }
}

fn binary(op: &crate::ir::Op) -> Option<(crate::ir::NodeId, crate::ir::NodeId)> {
    match op {
        crate::ir::Op::I64Add(a, b) | crate::ir::Op::I64Sub(a, b) | crate::ir::Op::I64Mul(a, b) => {
            Some((*a, *b))
        }
        _ => None,
    }
}

fn flat(shape: Option<&alloc::vec::Vec<crate::ir::ShapeTree>>) -> bool {
    shape.is_none_or(|items| items.iter().all(|item| item.is_scalar()))
}

/// Screens a raw graph for this profile's exclusions, before full graph validation.
///
/// A successful screen does not establish graph well-formedness: callers must still use
/// [`crate::check::check`] and pass its [`crate::check::CheckedGraph`] to [`emit`].
///
/// This filter lets producers diagnose out-of-profile content before full validation; every
/// profile refusal is reachable from raw producer data.
///
/// # Errors
/// The first refusal the profile imposes, in the order this module documents: profile, interface,
/// parameter, shape, fact, output count, then the node inventory in canonical node order.
pub fn admit_graph(graph: &crate::ir::Graph) -> Result<(), Refusal> {
    if graph.profile != crate::ir::SubProfile::I64 {
        return Err(Refusal::Profile(graph.profile.name()));
    }
    if u32::try_from(graph.slots.len()).ok() != Some(PARAMETERS) {
        return Err(Refusal::Interface);
    }
    for slot in &graph.slots {
        if matches!(slot.kind, crate::ir::SlotKind::Param) {
            return Err(Refusal::Param);
        }
        if slot.ty != crate::ir::ScalarType::I64 {
            return Err(Refusal::Interface);
        }
    }
    if !flat(graph.input_shape.as_ref()) || !flat(graph.output_shape.as_ref()) {
        return Err(Refusal::Shape);
    }
    if !graph.facts.is_empty() {
        return Err(Refusal::Fact);
    }
    if u32::try_from(graph.outputs.len()).ok() != Some(OUTPUTS) {
        return Err(Refusal::Output);
    }
    for node in &graph.nodes {
        match &node.op {
            crate::ir::Op::Input(slot) if *slot < PARAMETERS => {}
            crate::ir::Op::Input(_) => return Err(Refusal::Interface),
            crate::ir::Op::Const(crate::ir::Literal::I64(_)) => {}
            crate::ir::Op::Const(_) => return Err(Refusal::Operation(node.op.name())),
            crate::ir::Op::Param(_) => return Err(Refusal::Param),
            crate::ir::Op::I64Add(..) | crate::ir::Op::I64Sub(..) | crate::ir::Op::I64Mul(..) => {}
            crate::ir::Op::I64Eq(..) => return Err(Refusal::Equality),
            crate::ir::Op::Guard { .. } => return Err(Refusal::Guard),
            other => return Err(Refusal::Operation(other.name())),
        }
    }
    Ok(())
}

/// Admits the checker's accepted graph for this profile.
///
/// # Errors
/// [`admit_graph`]'s refusals.
pub fn admit(checked: &crate::check::CheckedGraph) -> Result<(), Refusal> {
    admit_graph(checked.graph())
}

/// Appends an unsigned integer without allocating a temporary decimal string.
#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-ngraph-maintainers; appending digits requires the caller-owned String builder, not an immutable string slice."
)]
fn append_u64_decimal(text: &mut alloc::string::String, mut value: u64) {
    let mut digits = [0_u8; 20];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (value % 10).to_le_bytes()[0];
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for digit in &digits[start..] {
        text.push(char::from(*digit));
    }
}

/// Appends the canonical signed decimal spelling, including `i64::MIN` without overflow.
#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-ngraph-maintainers; appending digits requires the caller-owned String builder, not an immutable string slice."
)]
fn append_i64_decimal(text: &mut alloc::string::String, value: i64) {
    if value < 0 {
        text.push('-');
        append_u64_decimal(text, value.unsigned_abs());
    } else {
        append_u64_decimal(text, value as u64);
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-ngraph-maintainers; this helper appends formatted values to the caller-owned module output buffer, so its mutable String sink is required."
)]
fn append_value(
    text: &mut alloc::string::String,
    graph: &crate::ir::Graph,
    node: crate::ir::NodeId,
) -> Result<(), Refusal> {
    let held = graph.nodes.get(node.index()).ok_or(Refusal::NodeMap)?;
    match &held.op {
        crate::ir::Op::Input(slot) => {
            if *slot >= PARAMETERS {
                return Err(Refusal::Interface);
            }
            text.push_str("%a");
            append_u64_decimal(text, u64::from(*slot));
            Ok(())
        }
        crate::ir::Op::Const(crate::ir::Literal::I64(value)) => {
            append_i64_decimal(text, *value);
            Ok(())
        }
        // An arithmetic node is named by its own SSA value. The checker admits only earlier nodes
        // as operands, so the fallback is the arithmetic arm for every admitted graph.
        crate::ir::Op::I64Add(..) | crate::ir::Op::I64Sub(..) | crate::ir::Op::I64Mul(..) => {
            text.push_str("%v");
            append_u64_decimal(text, u64::from(node.0));
            Ok(())
        }
        crate::ir::Op::Param(_) => Err(Refusal::Param),
        crate::ir::Op::Const(crate::ir::Literal::Bool(_))
        | crate::ir::Op::Const(crate::ir::Literal::F32(_)) => {
            Err(Refusal::Operation(held.op.name()))
        }
        crate::ir::Op::I64Eq(..) => Err(Refusal::Equality),
        crate::ir::Op::F32Add(..)
        | crate::ir::Op::F32Sub(..)
        | crate::ir::Op::F32Mul(..)
        | crate::ir::Op::F32Affine { .. }
        | crate::ir::Op::F32Relu(..)
        | crate::ir::Op::F32Reglu { .. }
        | crate::ir::Op::F32Step(..)
        | crate::ir::Op::F32Select { .. }
        | crate::ir::Op::F32Index { .. } => Err(Refusal::Operation(held.op.name())),
        crate::ir::Op::Guard { .. } => Err(Refusal::Guard),
    }
}

/// Emits the module for an admitted graph: one SSA value per arithmetic node, exact immediates,
/// plain `add`/`sub`/`mul`, and one `ret` of the selected output.
///
/// Node order is the graph's canonical order, so the text is deterministic: the same admitted graph
/// emits the same bytes.
///
/// # Errors
/// [`admit`]'s refusals.
pub fn emit(checked: &crate::check::CheckedGraph) -> Result<alloc::string::String, Refusal> {
    admit(checked)?;
    let graph = checked.graph();
    let mut text = alloc::string::String::new();
    crate::format::write_line!(text, "{HEADER_LINE}");
    crate::format::write_line!(text, "target datalayout = \"{DATA_LAYOUT}\"");
    crate::format::write_line!(text, "target triple = \"{TARGET_TRIPLE}\"");
    crate::format::write_line!(text, "{BLANK}");
    crate::format::write_line!(
        text,
        "define i64 @{FUNCTION}(i64 noundef %a0, i64 noundef %a1) {{"
    );
    crate::format::write_line!(text, "{ENTRY}");
    for (index, node) in graph.nodes.iter().enumerate() {
        let Some(opcode) = opcode(&node.op) else {
            continue;
        };
        let (left, right) = binary(&node.op).ok_or(Refusal::Operation(node.op.name()))?;
        crate::format::write_text!(text, "{INDENT}%v{index} = {} {TYPE} ", opcode.name());
        append_value(&mut text, graph, left)?;
        text.push_str(", ");
        append_value(&mut text, graph, right)?;
        text.push('\n');
    }
    let output = graph.outputs.first().ok_or(Refusal::Output)?;
    crate::format::write_text!(text, "{INDENT}{RET} {TYPE} ");
    append_value(&mut text, graph, *output)?;
    text.push('\n');
    crate::format::write_line!(text, "{CLOSE}");
    Ok(text)
}

/// The identity of one module's exact bytes under the injected scheme.
///
/// A digest identifies bytes and proves nothing: a retained binding records this identity together
/// with the graph's, and the semantic claim is the Lean theorem.
#[must_use]
pub fn module_identity(ids: &dyn crate::identity::IdentityScheme, text: &str) -> crate::ir::Digest {
    let mut canon = crate::ir::Canon::new(ids, "noble.ngraph.llvm-module.v0");
    canon.str(PROFILE);
    canon.str(TARGET_TRIPLE);
    canon.str(DATA_LAYOUT);
    canon.str(text);
    canon.finish()
}

/// Whether `token` is a canonical decimal: digits only, with no sign and no leading zero.
fn canonical(token: &str) -> bool {
    let bytes = token.as_bytes();
    let Some(first) = bytes.first() else {
        return false;
    };
    bytes.iter().all(u8::is_ascii_digit) && (*first != b'0' || bytes.len() == 1)
}

/// A canonical unsigned `u32`.
fn digits(token: &str) -> Option<u32> {
    canonical(token).then_some(token)?.parse().ok()
}

/// A canonical signed decimal `i64`, including `i64::MIN`.
fn literal(token: &str) -> Option<i64> {
    if token == "-0" {
        return None;
    }
    let body = token.strip_prefix('-').unwrap_or(token);
    canonical(body).then_some(token)?.parse().ok()
}

/// `%v<index>` with a canonical decimal index.
fn value_name(token: &str) -> Option<u32> {
    digits(token.strip_prefix("%v")?)
}

/// `%a<slot>` with a canonical decimal slot.
fn parameter_name(token: &str) -> Option<u32> {
    digits(token.strip_prefix("%a")?)
}

fn operand(token: &str) -> Option<Operand> {
    if let Some(slot) = parameter_name(token) {
        return Some(Operand::Parameter(slot));
    }
    if let Some(node) = value_name(token) {
        return Some(Operand::Value(node));
    }
    literal(token).map(Operand::Constant)
}

/// Reads one `  %v<n> = <op> i64 <a>, <b>` line.
fn read_instruction(line: &str) -> Result<Instruction, Refusal> {
    let body = line.strip_prefix(INDENT).ok_or(Refusal::Instruction)?;
    if body.split(' ').next() == Some(RET) {
        // A `ret` before the last body line is the second return this profile refuses.
        return Err(Refusal::Return);
    }
    let (name, rest) = body.split_once(" = ").ok_or(Refusal::Instruction)?;
    let node = value_name(name).ok_or(Refusal::Instruction)?;
    let mut words = rest.splitn(3, ' ');
    let spelling = words.next().ok_or(Refusal::Instruction)?;
    if FLAGS.contains(&spelling) {
        return Err(Refusal::Flags);
    }
    let opcode = match spelling {
        "add" => Opcode::Add,
        "sub" => Opcode::Sub,
        "mul" => Opcode::Mul,
        _ => return Err(Refusal::Opcode),
    };
    let ty = words.next().ok_or(Refusal::Instruction)?;
    if FLAGS.contains(&ty) {
        return Err(Refusal::Flags);
    }
    if ty != TYPE {
        return Err(Refusal::Instruction);
    }
    let spelled = words.next().ok_or(Refusal::Instruction)?;
    let (left, right) = spelled.split_once(", ").ok_or(Refusal::Instruction)?;
    Ok(Instruction {
        node,
        opcode,
        operands: [
            operand(left).ok_or(Refusal::Operand)?,
            operand(right).ok_or(Refusal::Operand)?,
        ],
    })
}

/// Reads one `  ret i64 <operand>` line.
fn read_return(line: &str) -> Result<Operand, Refusal> {
    let body = line.strip_prefix(INDENT).ok_or(Refusal::Return)?;
    let rest = body
        .strip_prefix(RET)
        .and_then(|trailing| trailing.strip_prefix(' '))
        .and_then(|typed| typed.strip_prefix(TYPE))
        .and_then(|typed| typed.strip_prefix(' '))
        .ok_or(Refusal::Return)?;
    operand(rest).ok_or(Refusal::Operand)
}

/// Matches the pinned function signature without constructing a temporary line.
fn signature_matches(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("define i64 @") else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(FUNCTION) else {
        return false;
    };
    rest == "(i64 noundef %a0, i64 noundef %a1) {"
}

/// Tokenizes the module text and reads the signature, the instruction list and the return operand.
///
/// Nothing here consults a graph or a second emission: the header, the targets, the signature, the
/// instruction grammar, the strictly increasing destinations, the single `ret` and the closing brace
/// are checked against the profile's pinned text, and the result is the artifact as spelled.
///
/// # Errors
/// [`Refusal::Header`], [`Refusal::Target`], [`Refusal::Interface`], [`Refusal::Instruction`],
/// [`Refusal::Opcode`], [`Refusal::Flags`], [`Refusal::Operand`] or [`Refusal::Return`].
pub fn parse(text: &str) -> Result<Module, Refusal> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER_LINE) {
        return Err(Refusal::Header);
    }
    let layout = lines
        .next()
        .and_then(|line| line.strip_prefix("target datalayout = \""))
        .and_then(|value| value.strip_suffix('\"'));
    if layout != Some(DATA_LAYOUT) {
        return Err(Refusal::Target);
    }
    let triple = lines
        .next()
        .and_then(|line| line.strip_prefix("target triple = \""))
        .and_then(|value| value.strip_suffix('\"'));
    if triple != Some(TARGET_TRIPLE) {
        return Err(Refusal::Target);
    }
    if lines.next() != Some(BLANK) {
        return Err(Refusal::Target);
    }
    if !lines.next().is_some_and(signature_matches) {
        return Err(Refusal::Interface);
    }
    if lines.next() != Some(ENTRY) {
        return Err(Refusal::Instruction);
    }
    if lines.next_back() != Some(CLOSE) {
        return Err(Refusal::Return);
    }
    let returned = lines.next_back().ok_or(Refusal::Return)?;
    let output = read_return(returned)?;
    let mut instructions = alloc::vec::Vec::with_capacity(lines.clone().count());
    let mut previous: Option<u32> = None;
    for line in lines {
        let instruction = read_instruction(line)?;
        if previous.is_some_and(|earlier| instruction.node <= earlier) {
            return Err(Refusal::Instruction);
        }
        previous = Some(instruction.node);
        instructions.push(instruction);
    }
    Ok(Module {
        instructions,
        output,
    })
}

/// Whether the parsed operand, read where it stands, is the value of graph node `node`.
fn denotes(graph: &crate::ir::Graph, node: usize, spelled: Operand) -> bool {
    let Some(held) = graph.nodes.get(node) else {
        return false;
    };
    match (spelled, &held.op) {
        (Operand::Parameter(slot), crate::ir::Op::Input(declared)) => slot == *declared,
        (Operand::Constant(bits), crate::ir::Op::Const(crate::ir::Literal::I64(value))) => {
            bits == *value
        }
        (Operand::Value(index), op) => is_arith(op) && usize::try_from(index) == Ok(node),
        _ => false,
    }
}

/// Binds a parsed module to the admitted graph, independently of how the module was produced.
///
/// The instruction at each position must define exactly the arithmetic node at that position, with
/// that node's opcode and operands, and the `ret` operand must denote the selected output.
///
/// # Errors
/// [`admit`]'s refusals, then [`Refusal::NodeMap`], [`Refusal::Operands`] or [`Refusal::Output`].
pub fn verify(checked: &crate::check::CheckedGraph, module: &Module) -> Result<(), Refusal> {
    admit(checked)?;
    let graph = checked.graph();
    let arithmetic_nodes = graph.nodes.iter().filter(|node| is_arith(&node.op)).count();
    if arithmetic_nodes != module.instructions.len() {
        return Err(Refusal::NodeMap);
    }
    let mut instructions = module.instructions.iter();
    for (node, held) in graph.nodes.iter().enumerate() {
        if !is_arith(&held.op) {
            continue;
        }
        let instruction = instructions.next().ok_or(Refusal::NodeMap)?;
        if usize::try_from(instruction.node) != Ok(node) {
            return Err(Refusal::NodeMap);
        }
        let (left, right) = binary(&held.op).ok_or(Refusal::NodeMap)?;
        if opcode(&held.op) != Some(instruction.opcode) {
            return Err(Refusal::NodeMap);
        }
        if !denotes(graph, left.index(), instruction.operands[0])
            || !denotes(graph, right.index(), instruction.operands[1])
        {
            return Err(Refusal::Operands);
        }
    }
    let output = graph.outputs.first().ok_or(Refusal::Output)?;
    if !denotes(graph, output.index(), module.output) {
        return Err(Refusal::Output);
    }
    Ok(())
}

/// Parses the text and binds it to the admitted graph: the validator the tests and the lane call.
///
/// # Errors
/// [`parse`]'s refusals, then [`verify`]'s.
pub fn validate(checked: &crate::check::CheckedGraph, text: &str) -> Result<Module, Refusal> {
    let module = parse(text)?;
    verify(checked, &module)?;
    Ok(module)
}

fn operand_value(
    spelled: Operand,
    inputs: [i64; 2],
    registers: &[(u32, Option<i64>)],
) -> Option<i64> {
    match spelled {
        Operand::Parameter(slot) => inputs.get(usize::try_from(slot).ok()?).copied(),
        Operand::Constant(bits) => Some(bits),
        Operand::Value(node) => {
            let index = registers
                .binary_search_by_key(&node, |(register, _)| *register)
                .ok()?;
            registers.get(index).and_then(|(_, value)| *value)
        }
    }
}

/// The value one concrete input pair reads out of the parsed module.
///
/// This interprets the artifact's own instructions in text order, mirroring the Lean model
/// (`NobleFoundation.LlvmLowering.result`): a parameter and a literal are read where they stand, a
/// `%v<n>` operand reads the value node `n` wrote, and an instruction with a missing operand writes
/// no value. It is evidence of **concrete agreement** at the inputs a caller supplies; the
/// universally quantified statement is the Lean theorem, and neither replaces the other. Its
/// sparse register table is sized by instruction count, not by the numeric SSA identifiers.
#[must_use]
pub fn execute(module: &Module, inputs: [i64; 2]) -> Option<i64> {
    let mut registers: alloc::vec::Vec<(u32, Option<i64>)> =
        alloc::vec::Vec::with_capacity(module.instructions.len());
    for instruction in &module.instructions {
        let left = operand_value(instruction.operands[0], inputs, &registers);
        let right = operand_value(instruction.operands[1], inputs, &registers);
        let value = match (left, right) {
            (Some(left), Some(right)) => Some(match instruction.opcode {
                Opcode::Add => left.wrapping_add(right),
                Opcode::Sub => left.wrapping_sub(right),
                Opcode::Mul => left.wrapping_mul(right),
            }),
            _ => None,
        };
        match registers.binary_search_by_key(&instruction.node, |(register, _)| *register) {
            Ok(index) => {
                let (_, register) = registers.get_mut(index)?;
                *register = value;
            }
            Err(index) => registers.insert(index, (instruction.node, value)),
        }
    }
    operand_value(module.output, inputs, &registers)
}
