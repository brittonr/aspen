
pub fn step_value(step: &super::core::CoreStep) -> IoValue {
    match step {
        super::core::CoreStep::Send { from, to, body } => {
            record("send", vec![string(from), string(to), body.as_iovalue().clone()])
        }
        super::core::CoreStep::Observe { actor, pattern } => {
            record("observe", vec![string(actor), pattern.as_iovalue().clone()])
        }
        super::core::CoreStep::Assert { actor, value } => {
            record("assert", vec![string(actor), value.as_iovalue().clone()])
        }
        super::core::CoreStep::Retract { actor, value } => {
            record("retract", vec![string(actor), value.as_iovalue().clone()])
        }
        super::core::CoreStep::Clock { actor } => record("clock", vec![string(actor)]),
        super::core::CoreStep::Random { actor, upper } => record("random", vec![string(actor), u64_value(*upper)]),
    }
}
