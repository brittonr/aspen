use std::rc;

use super::super::admission;
use super::super::allocation;
use super::super::continuation::Binding;
use super::super::continuation::CurrentAdmission;
use super::super::continuation::Failure;
use super::super::continuation::Progress;
use super::super::continuation::ResumeContext;
use super::super::fixtureauthority;
use super::super::profile;
use super::super::receipt::Terminal;
use super::super::replay;
use super::Engine;
use super::Start;

const CREATED_TURN: u64 = 10;
const RETENTION_TURNS: u64 = 2;
const LAST_RETAINED_TURN: u64 = CREATED_TURN + RETENTION_TURNS;

fn at_retention_limit() -> (Engine, ResumeContext) {
    // Run the actual engine under explicitly local admission; no provider evidence or live effects.
    let wasm = wat::parse_str(
        r#"(module
        (memory (export "memory") 1 1)
        (func (export "molten_alloc") (param i32) (result i32) i32.const 0)
        (func (export "molten_run") (param i32 i32) (result i64) local.get 1 i64.extend_i32_u)
        (func (export "molten_dealloc") (param i32 i32)))"#,
    )
    .unwrap();
    let value = crate::codec::record("retention-probe", Vec::new());
    let input = crate::codec::canonical_bytes(&value).unwrap();
    let mut profile = profile::Profile::default();
    profile.limits.retention_turns = RETENTION_TURNS;
    let binding = Binding {
        actor: "fixture".into(),
        generation: 1,
        artifact: crate::codec::content_ref_from_bytes(&wasm),
        profile: profile.content_ref().unwrap(),
        input: crate::codec::canonical_hash(&value).unwrap(),
        authority: String::new(),
        resource: crate::codec::content_ref_from_bytes(b"local retention resources"),
        effects: crate::codec::content_ref_from_bytes(b"no fixture effects"),
    };
    let current = rc::Rc::new(fixtureauthority::Authority::new(binding).unwrap());
    let binding = current.current().unwrap();
    let plan = admission::Plan {
        profile,
        profile_ref: binding.profile.clone(),
        artifact_ref: binding.artifact.clone(),
        actor: binding.actor.clone(),
        generation: binding.generation,
        authority_ref: binding.authority.clone(),
        resource_ref: binding.resource.clone(),
        evidence_ref: crate::codec::content_ref_from_bytes(b"local test only"),
        fixture_name: "local-retention-probe".into(),
        input_ref: binding.input.clone(),
        hostcalls: Vec::new(),
        memory_pages: 1,
        table_elements: 0,
    };
    let mut engine = Engine::start(&wasm, Start {
        plan,
        binding,
        current: current.clone(),
        host: rc::Rc::new(replay::Replay::new(Vec::new())),
        input,
        logical_turn: CREATED_TURN,
    })
    .unwrap();
    let context = ResumeContext {
        current,
        logical_turn: LAST_RETAINED_TURN,
    };
    assert!(matches!(engine.resume(&context), Progress::Yield { .. }), "allocator phase");
    assert!(matches!(engine.resume(&context), Progress::Yield { .. }), "run phase at the same turn");
    (engine, context)
}

// r[verify molten.spacewasm_core.resume]
#[test]
fn retained_turn_rejects_clock_regression_and_expiry() {
    for turn in [LAST_RETAINED_TURN - 1, LAST_RETAINED_TURN + 1] {
        for cancel in [false, true] {
            let (mut engine, mut context) = at_retention_limit();
            let spent = engine.instructions();
            context.logical_turn = turn;
            let progress = if cancel {
                engine.cancel(&context, "operator-cancelled".into())
            } else {
                engine.resume(&context)
            };
            let Progress::Terminal(observation) = progress else {
                panic!("a stale continuation must terminate before executing");
            };
            assert_eq!(observation.terminal, Terminal::Failed(Failure::Denied("stale continuation".into())));
            assert_eq!(engine.instructions(), spent, "stale turn must not consume work");
            assert!(engine.cleanup().iter().any(|event| event.detail() == "guest-code-stack-memory-released"));
            assert!(allocation::Budget::enter(0).is_ok());
        }
    }
    let (mut engine, context) = at_retention_limit();
    let Progress::Terminal(observation) = engine.resume(&context) else {
        panic!("another segment at the retention limit must finish");
    };
    assert_eq!(observation.terminal, Terminal::Finished);
}
