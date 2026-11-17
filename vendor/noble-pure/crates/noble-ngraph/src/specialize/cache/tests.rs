use super::*;
use crate::identity::IdentityScheme;

struct TestIds;

struct TestHasher([u64; 4]);

impl crate::identity::IdentityHasher for TestHasher {
    fn update(&mut self, bytes: &[u8]) {
        for (index, byte) in bytes.iter().enumerate() {
            let lane = index % 4;
            self.0[lane] = self.0[lane]
                .rotate_left(7)
                .wrapping_mul(0x9e37_79b1_85eb_ca87)
                ^ u64::from(*byte);
        }
    }

    fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest {
        let mut digest = [0; 32];
        for (index, lane) in self.0.iter().enumerate() {
            digest[index * 8..index * 8 + 8].copy_from_slice(&lane.to_le_bytes());
        }
        digest
    }
}

impl IdentityScheme for TestIds {
    fn name(&self) -> &'static str {
        "cache-test"
    }

    fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
        alloc::boxed::Box::new(TestHasher([0x6a09_e667_f3bc_c908; 4]))
    }
}

fn graph(
    ids: &dyn IdentityScheme,
    root_uses_y: bool,
    capture: i64,
    closure: &[u64],
    third_input: bool,
    bool_input: bool,
    assumption: bool,
) -> crate::check::CheckedGraph {
    let subject = crate::ir::Subject::derive(
        ids,
        crate::ir::SubjectKind::Extracted,
        &[b"cache-fixture-root"],
        closure.to_vec(),
    );
    let mut builder = crate::ir::GraphBuilder::new(crate::ir::SubProfile::I64, subject);
    let x = builder.input(crate::ir::ScalarType::I64);
    let y = builder.input(crate::ir::ScalarType::I64);
    let product = builder.op(crate::ir::Op::I64Mul(x, y));
    let capture_node = builder.i64_const(capture);
    let captured = builder.op(crate::ir::Op::I64Add(product, capture_node));
    let root_operand = if root_uses_y { y } else { x };
    let result = builder.op(crate::ir::Op::I64Add(captured, root_operand));
    builder.output(result);
    if third_input {
        let z = builder.input(crate::ir::ScalarType::I64);
        let sum = builder.op(crate::ir::Op::I64Add(result, z));
        builder.output(sum);
    }
    if bool_input {
        let bit = builder.input(crate::ir::ScalarType::Bool);
        builder.output(bit);
    }
    if assumption {
        builder.fact(
            x,
            crate::ir::Fact::Range {
                lo: crate::ir::Literal::I64(0),
                hi: crate::ir::Literal::I64(10),
            },
            crate::ir::Provenance::InputValidated,
        );
    }
    crate::check::check(ids, builder.finish()).expect("fixture graph checks")
}

fn run(
    session: &mut Session,
    ids: &dyn IdentityScheme,
    checked: &crate::check::CheckedGraph,
    static_inputs: &[Option<i64>],
) -> core::result::Result<Result, crate::diag::Diagnostic> {
    let recipe = crate::format::graph::print(checked.graph());
    let budgets = crate::check::Budgets::default();
    session.specialize(super::Invocation::new(
        ids,
        checked,
        recipe.as_bytes(),
        super::super::Request {
            static_inputs,
            budgets: &budgets,
        },
        noble_contracts::Limits::default(),
    ))
}

fn evaluate(
    checked: &crate::check::CheckedGraph,
    values: &[i64],
) -> alloc::vec::Vec<crate::sem::Value> {
    let inputs = values
        .iter()
        .copied()
        .map(crate::sem::Value::I64)
        .collect::<alloc::vec::Vec<_>>();
    match crate::eval::reference(checked, &inputs, None, &mut crate::eval::Control::default()).obs {
        crate::eval::Obs::Normal(outputs) => outputs,
        other => panic!("fixture evaluation refused: {other:?}"),
    }
}

#[path = "tests/adversarial.rs"]
mod adversarial;
#[path = "tests/identity.rs"]
mod identity;
#[path = "tests/matrix.rs"]
mod matrix;
#[path = "tests/resources.rs"]
mod resources;
#[path = "tests/reuse.rs"]
mod reuse;
