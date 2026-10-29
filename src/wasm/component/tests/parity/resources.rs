use std::fmt::Write;

use super::*;

fn memory_fixture(consumer: Consumer, memory_pages: u32, memory_count: u32) -> super::super::support::Fixture {
    let mut fixture = super::super::support::Fixture::new(consumer);
    let mut other_components = String::new();
    for index in 1..memory_count {
        write!(
            other_components,
            "(core module $memory-{index} (memory {memory_pages} {memory_pages}))\n\
             (core instance $instance-{index} (instantiate $memory-{index}))\n"
        )
        .expect("memory fixture component text");
    }
    fixture.replace_component_bytes(fixtures::encode(memory_pages, "", fixtures::ECHO, &other_components, ""));
    fixture.facts.memory.initial = u64::from(memory_pages) * 65_536;
    fixture.facts.memory.maximum = Some(fixture.facts.memory.initial);
    fixture.facts.memories = u64::from(memory_count);
    fixture.facts.instances = u64::from(memory_count);
    fixture
}

fn assert_resource_admission(fixture: &super::super::support::Fixture, is_admitted: bool) {
    // Prove these are actual resource limits, not stale-facts denials.
    super::super::super::admission::inspection::verify_artifact_facts(&fixture.component_bytes, &fixture.facts)
        .expect("resource facts match actual component bytes");
    let outcome = run(fixture);
    assert_eq!(outcome.is_pass(), is_admitted, "{:?}", outcome.diagnostics);
    if is_admitted {
        assert_eq!(outcome.output, Some(super::super::support::input_value()));
    } else {
        assert_eq!(outcome.output, None);
        assert_eq!(stages(&outcome), vec![Stage::Denial]);
        assert_eq!(outcome.receipts[0].input.trap_class.as_deref(), Some("component-admission-denial"));
    }
}

#[test]
fn actual_per_memory_ceiling_preserves_admission() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component.resources]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        for (pages, is_admitted) in [(256, true), (257, false)] {
            assert_resource_admission(&memory_fixture(consumer, pages, 1), is_admitted);
        }
    }
}

#[test]
fn actual_memory_count_ceiling_preserves_admission() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component.resources]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        for (count, is_admitted) in [(4, true), (5, false)] {
            assert_resource_admission(&memory_fixture(consumer, 1, count), is_admitted);
        }
    }
}
