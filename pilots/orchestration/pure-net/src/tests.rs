// SPDX-License-Identifier: AGPL-3.0-or-later
use molten_batch_core::Id;

use super::*;
fn host() -> HostBindings {
    HostBindings {
        executable: Id([1; 32]),
        dependency_closure: Id([2; 32]),
    }
}
fn grants() -> ExecutionGrants {
    ExecutionGrants {
        fuel: 256_000_000,
        memory_bytes: 64 * 1024 * 1024,
        max_output_bytes: 262_144,
        max_attempts: 1,
    }
}
fn run(source: &str, inputs: &[i64], backend: Backend, limits: Limits, grants: ExecutionGrants) -> Receipt {
    run_result(source, inputs, backend, limits, grants).unwrap()
}
fn run_result(
    source: &str,
    inputs: &[i64],
    backend: Backend,
    limits: Limits,
    grants: ExecutionGrants,
) -> Result<Receipt, Error> {
    let manifest = encode_manifest(&manifest(host(), backend, limits).unwrap()).unwrap();
    execute(
        ExecutionInput {
            manifest_bytes: &manifest,
            program_bytes: &compile(source, inputs.len() as u32).unwrap(),
            input_bytes: &encode_inputs(inputs).unwrap(),
            host: &host(),
            grants: &grants,
        },
        &mut |_| Control::Continue,
    )
    .map(|(receipt, _)| receipt)
}
// r[verify molten.pure_net.producer]
// r[verify molten.pure_net.outcomes]
#[test]
fn complete_frozen_corpus_matches_independent_source_ir_and_exact_expectations() {
    for case in fixtures::POSITIVE_FIXTURES {
        let expected = Outcome::Completed(case.expected.iter().copied().map(Value::I64).collect());
        for backend in [Backend::SequentialNet, Backend::IndependentReference] {
            let actual = run(case.source, case.inputs, backend, Limits::default(), grants());
            assert_eq!(actual.outcome, expected, "{} {backend:?}", case.name);
            assert_eq!(decode_receipt(&encode_receipt(&actual).unwrap()).unwrap(), actual);
            assert!(actual.counters.total_work > 0);
        }
    }
}
// r[verify molten.pure_net.identity]
// r[verify molten.pure_net.replay]
#[test]
fn repeated_full_binding_has_identical_canonical_receipt() {
    let a = run("dup dup * swap", &[7], Backend::SequentialNet, Limits::default(), grants());
    let b = run("dup dup * swap", &[7], Backend::SequentialNet, Limits::default(), grants());
    let changed_input = run("dup dup * swap", &[8], Backend::SequentialNet, Limits::default(), grants());
    let changed_manifest = run(
        "dup dup * swap",
        &[7],
        Backend::SequentialNet,
        Limits {
            quantum_work: 1,
            ..Limits::default()
        },
        grants(),
    );
    assert_eq!(encode_receipt(&a).unwrap(), encode_receipt(&b).unwrap());
    assert_eq!(a.outcome, Outcome::Completed(vec![Value::I64(49), Value::I64(7)]));
    assert_eq!(changed_input.outcome, Outcome::Completed(vec![Value::I64(64), Value::I64(8)]));
    assert_eq!(changed_manifest.outcome, a.outcome);
    assert_ne!(a.ordered_inputs, changed_input.ordered_inputs);
    assert_ne!(a.ordered_input_semantics, changed_input.ordered_input_semantics);
    assert_ne!(a.manifest, changed_manifest.manifest);
    assert_ne!(a.manifest_semantics, changed_manifest.manifest_semantics);
    assert_ne!(encode_receipt(&a).unwrap(), encode_receipt(&changed_input).unwrap());
    assert_ne!(encode_receipt(&a).unwrap(), encode_receipt(&changed_manifest).unwrap());
    assert_ne!(a.manifest, a.manifest_semantics);
    assert_ne!(a.program, a.program_semantics);
    assert_eq!(a.bindings.actual_producer_inets_reference, PRODUCER_INETS);
    assert_ne!(a.bindings.actual_producer_inets_reference, a.bindings.requested_inets_reference);
}
#[test]
fn fuel_boundary_preserves_cumulative_work_and_never_returns_partial_values() {
    let baseline = run("-", &[19, 7], Backend::SequentialNet, Limits::default(), grants());
    let k = baseline.counters.interactions;
    assert!(k > 0);
    for cap in [k - 1, k, k + 1] {
        let limits = Limits {
            interaction_fuel: cap,
            quantum_work: 1,
            ..Limits::default()
        };
        let actual = run("-", &[19, 7], Backend::SequentialNet, limits, grants());
        if cap < k {
            assert_eq!(actual.outcome, Outcome::Rejected(Error::Exhausted(Resource::Reduction)));
            assert_eq!(actual.counters.interactions, cap);
        } else {
            assert_eq!(actual.outcome, Outcome::Completed(vec![Value::I64(12)]));
        }
    }
}
#[test]
fn reference_has_its_own_node_fuel_boundary() {
    let baseline = run("dup *", &[5], Backend::IndependentReference, Limits::default(), grants());
    let nodes = baseline.counters.reference_nodes;
    assert!(nodes > 0);
    let low = run(
        "dup *",
        &[5],
        Backend::IndependentReference,
        Limits {
            reference_fuel: nodes - 1,
            ..Limits::default()
        },
        grants(),
    );
    assert_eq!(low.outcome, Outcome::Rejected(Error::Exhausted(Resource::Reference)));
    let exact = run(
        "dup *",
        &[5],
        Backend::IndependentReference,
        Limits {
            reference_fuel: nodes,
            ..Limits::default()
        },
        grants(),
    );
    assert_eq!(exact.outcome, Outcome::Completed(vec![Value::I64(25)]));
}
#[test]
fn each_nonreduction_work_phase_has_an_effective_negative_cap() {
    for (limits, resource) in [
        (
            Limits {
                max_setup_work: 1,
                ..Limits::default()
            },
            Resource::Setup,
        ),
        (
            Limits {
                max_validation_work: 1,
                ..Limits::default()
            },
            Resource::Validation,
        ),
        (
            Limits {
                max_readback_work: 1,
                ..Limits::default()
            },
            Resource::Readback,
        ),
        (
            Limits {
                max_observations: 1,
                ..Limits::default()
            },
            Resource::Observations,
        ),
        (
            Limits {
                max_total_work: 1,
                ..Limits::default()
            },
            Resource::TotalWork,
        ),
        (
            Limits {
                max_queue_entries: 1,
                ..Limits::default()
            },
            Resource::Queue,
        ),
        (
            Limits {
                max_agent_slots: 1,
                ..Limits::default()
            },
            Resource::Nodes,
        ),
        (
            Limits {
                max_wires: 1,
                ..Limits::default()
            },
            Resource::Wires,
        ),
    ] {
        let actual = run("dup *", &[5], Backend::SequentialNet, limits, grants());
        assert_eq!(actual.outcome, Outcome::Rejected(Error::Exhausted(resource)), "{resource:?}");
    }
}
#[test]
fn request_grants_are_not_replaced_by_the_larger_manifest() {
    for (grant, resource) in [
        (
            ExecutionGrants {
                memory_bytes: 1,
                ..grants()
            },
            Resource::Memory,
        ),
        (ExecutionGrants { fuel: 1, ..grants() }, Resource::TotalWork),
    ] {
        assert_eq!(
            run_result("-", &[19, 7], Backend::SequentialNet, Limits::default(), grant),
            Err(Error::Exhausted(resource))
        );
    }
    assert_eq!(
        run("-", &[19, 7], Backend::SequentialNet, Limits::default(), ExecutionGrants {
            max_output_bytes: 1,
            ..grants()
        })
        .outcome,
        Outcome::Rejected(Error::Exhausted(Resource::Readback))
    );
    assert_eq!(
        run_result(
            "-",
            &[19, 7],
            Backend::SequentialNet,
            Limits {
                max_memory_bytes: 1,
                ..Limits::default()
            },
            grants()
        ),
        Err(Error::Exhausted(Resource::Memory))
    );
}
// r[verify molten.pure_net.bounds]
#[test]
fn malformed_input_bounds_and_wrong_host_are_admission_failures() {
    for (_, source, arity) in fixtures::MALFORMED_SOURCES {
        assert!(compile(source, *arity).is_err());
    }
    let bytes = encode_manifest(&manifest(host(), Backend::SequentialNet, Limits::default()).unwrap()).unwrap();
    let program = compile("-", 2).unwrap();
    let inputs = encode_inputs(&[19, 7]).unwrap();
    let wrong = HostBindings {
        executable: Id([3; 32]),
        ..host()
    };
    assert_eq!(
        execute(
            ExecutionInput {
                manifest_bytes: &bytes,
                program_bytes: &program,
                input_bytes: &inputs,
                host: &wrong,
                grants: &grants()
            },
            &mut |_| Control::Continue
        ),
        Err(Error::Bindings)
    );
    assert_eq!(
        execute(
            ExecutionInput {
                manifest_bytes: &bytes,
                program_bytes: &program,
                input_bytes: b"not a frame",
                host: &host(),
                grants: &grants()
            },
            &mut |_| Control::Continue
        ),
        Err(Error::Malformed)
    );
    assert_eq!(
        execute(
            ExecutionInput {
                manifest_bytes: &bytes,
                program_bytes: &program,
                input_bytes: &inputs,
                host: &host(),
                grants: &ExecutionGrants {
                    max_attempts: 5,
                    ..grants()
                },
            },
            &mut |_| Control::Continue,
        ),
        Err(Error::Exhausted(Resource::Attempts))
    );
    let tiny = encode_manifest(
        &manifest(host(), Backend::SequentialNet, Limits {
            max_program_bytes: 1,
            ..Limits::default()
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        execute(
            ExecutionInput {
                manifest_bytes: &tiny,
                program_bytes: &program,
                input_bytes: &inputs,
                host: &host(),
                grants: &grants()
            },
            &mut |_| Control::Continue
        ),
        Err(Error::Exhausted(Resource::Decode))
    );
}
#[test]
fn untrusted_checked_ir_claim_is_reconstructed_not_believed() {
    let mut program: MaterializedProgram = decode(PROGRAM, &compile("-", 2).unwrap(), MAX_BYTES as u64).unwrap();
    program.program.source = "+".into();
    let bytes = encode_manifest(&manifest(host(), Backend::SequentialNet, Limits::default()).unwrap()).unwrap();
    let (receipt, _) = execute(
        ExecutionInput {
            manifest_bytes: &bytes,
            program_bytes: &encode(PROGRAM, &program).unwrap(),
            input_bytes: &encode_inputs(&[19, 7]).unwrap(),
            host: &host(),
            grants: &grants(),
        },
        &mut |_| Control::Continue,
    )
    .unwrap();
    assert_eq!(receipt.outcome, Outcome::Rejected(Error::Bindings));
}
#[test]
fn cancellation_at_setup_validation_reduction_or_readback_never_publishes_values() {
    for phase in [
        Phase::ParseCheck,
        Phase::Extract,
        Phase::Lower,
        Phase::Validate,
        Phase::Setup,
        Phase::Reduce,
        Phase::Readback,
    ] {
        let bytes = encode_manifest(
            &manifest(host(), Backend::SequentialNet, Limits {
                quantum_work: 1,
                ..Limits::default()
            })
            .unwrap(),
        )
        .unwrap();
        let (receipt, _) = execute(
            ExecutionInput {
                manifest_bytes: &bytes,
                program_bytes: &compile("dup *", 1).unwrap(),
                input_bytes: &encode_inputs(&[7]).unwrap(),
                host: &host(),
                grants: &grants(),
            },
            &mut |observation| {
                if observation.phase == phase {
                    Control::Cancel
                } else {
                    Control::Continue
                }
            },
        )
        .unwrap();
        assert_eq!(receipt.outcome, Outcome::Rejected(Error::Cancelled), "{phase:?}");
    }
}
#[test]
fn source_size_depth_and_node_boundaries_are_not_runtime_fuel() {
    let tiny_source = Limits {
        max_source_bytes: 1,
        ..Limits::default()
    };
    assert_eq!(
        run("dup *", &[5], Backend::SequentialNet, tiny_source, grants()).outcome,
        Outcome::Rejected(Error::Exhausted(Resource::Source))
    );
    let tiny_frontend = Limits {
        max_frontend_work: 1,
        ..Limits::default()
    };
    assert_eq!(
        run("dup *", &[5], Backend::SequentialNet, tiny_frontend, grants()).outcome,
        Outcome::Rejected(Error::Exhausted(Resource::Source))
    );
    let tiny_ir = Limits {
        max_ir_nodes: 1,
        ..Limits::default()
    };
    assert_eq!(
        run("dup *", &[5], Backend::SequentialNet, tiny_ir, grants()).outcome,
        Outcome::Rejected(Error::Exhausted(Resource::Ir))
    );
}
#[test]
fn exact_source_bytes_limit_is_inclusive() {
    let receipt = run(
        "-",
        &[19, 7],
        Backend::SequentialNet,
        Limits {
            max_source_bytes: 1,
            ..Limits::default()
        },
        grants(),
    );
    assert_eq!(receipt.outcome, Outcome::Completed(vec![Value::I64(12)]));
}
#[test]
fn real_validator_rejects_open_cyclic_and_wrong_ordered_nets() {
    use noble_ngraph::eval;
    use noble_ngraph::net;
    let checked = producer::checked(
        &Program {
            source: "-".into(),
            input_arity: 2,
        },
        &Limits::default(),
        &mut |_, _| Ok(()),
    )
    .unwrap()
    .checked;
    let inputs = eval::i64_inputs(&[19, 7]);
    let original = net::lower(&producer::Ids, &checked, &inputs, &net::Limits::default()).unwrap();
    let mut open = original.clone();
    open.agents[0].ports[0] = None;
    assert!(net::validate(&producer::Ids, &checked, &open, &inputs, &net::Limits::default()).is_err());
    let mut self_cycle = original.clone();
    self_cycle.agents[0].ports[0] = Some(net::Endpoint::Port(net::PortRef {
        agent: 0,
        port_index: 0.into(),
    }));
    assert!(net::validate(&producer::Ids, &checked, &self_cycle, &inputs, &net::Limits::default()).is_err());
    let mut incomplete = original.clone();
    incomplete.outputs.clear();
    let actual = net::reduce(&producer::Ids, net::ReduceRequest {
        checked: &checked,
        candidate: &incomplete,
        inputs: &inputs,
        limits: &net::Limits::default(),
        plan: net::Plan::default(),
    });
    assert!(!matches!(actual.obs, net::Obs::Normal(_)));
    let mut wrong_payload = original.clone();
    wrong_payload.agents[0].symbol = net::Symbol::Num(123);
    assert!(net::validate(&producer::Ids, &checked, &wrong_payload, &inputs, &net::Limits::default()).is_err());
}

#[test]
fn producer_erase_rule_fires_and_fuel_zero_never_returns_a_prefix() {
    use noble_ngraph::check;
    use noble_ngraph::eval;
    use noble_ngraph::ir;
    use noble_ngraph::net;
    use noble_ngraph::sem;
    let mut builder =
        ir::GraphBuilder::new(ir::SubProfile::I64, ir::Subject::authored(&producer::Ids, "aspen-erase-boundary"));
    let retained = builder.input(ir::ScalarType::I64);
    let _discarded = builder.input(ir::ScalarType::I64);
    builder.output(retained);
    let checked = check::check(&producer::Ids, builder.finish()).unwrap();
    let inputs = [sem::Value::I64(29), sem::Value::I64(-3)];
    let candidate = net::lower(&producer::Ids, &checked, &inputs, &net::Limits::default()).unwrap();
    assert_eq!(candidate.rule_counts(), (0, 0, 1));
    for fuel in [0, 1] {
        let limits = net::Limits {
            interaction_fuel: fuel,
            ..net::Limits::default()
        };
        let report = net::reduce(&producer::Ids, net::ReduceRequest {
            checked: &checked,
            candidate: &candidate,
            inputs: &inputs,
            limits: &limits,
            plan: net::Plan::default(),
        });
        if fuel == 0 {
            assert!(matches!(report.obs, net::Obs::Exhausted {
                limit: net::Limit::Fuel,
                ..
            }));
        } else {
            let reference = eval::reference(&checked, &inputs, None, &mut eval::Control::default());
            assert_eq!(reference.obs, eval::Obs::Normal(vec![sem::Value::I64(29)]));
            assert_eq!(report.obs, net::Obs::Normal(vec![sem::Value::I64(29)]));
            assert_eq!(report.interactions, 1);
        }
    }
}

#[test]
fn measured_setup_validation_readback_caps_are_inclusive() {
    let baseline = run("dup *", &[5], Backend::SequentialNet, Limits::default(), grants());
    for resource in [Resource::Setup, Resource::Validation, Resource::Readback] {
        let cap = match resource {
            Resource::Setup => baseline.counters.setup_work,
            Resource::Validation => baseline.counters.validation_work,
            Resource::Readback => baseline.counters.readback_work,
            _ => unreachable!(),
        };
        assert!(cap > 1);
        for bound in [cap - 1, cap] {
            let mut limits = Limits::default();
            match resource {
                Resource::Setup => limits.max_setup_work = bound,
                Resource::Validation => limits.max_validation_work = bound,
                Resource::Readback => limits.max_readback_work = bound,
                _ => unreachable!(),
            }
            let actual = run("dup *", &[5], Backend::SequentialNet, limits, grants());
            if bound == cap {
                assert_eq!(actual.outcome, Outcome::Completed(vec![Value::I64(25)]));
            } else {
                assert_eq!(actual.outcome, Outcome::Rejected(Error::Exhausted(resource)));
            }
        }
    }
}

#[test]
fn finite_manifest_rejects_unknown_profiles_unbounded_caps_and_grant_incoherence() {
    let limits = Limits {
        max_total_work: u64::MAX,
        ..Limits::default()
    };
    assert_eq!(manifest(host(), Backend::SequentialNet, limits), Err(Error::InvalidLimits));
    let mut limits = Limits::default();
    limits.process_cpu_hard_seconds = limits.process_cpu_soft_seconds - 1;
    assert_eq!(manifest(host(), Backend::SequentialNet, limits), Err(Error::InvalidLimits));
    let limits = Limits {
        max_aggregate_attempt_work: 1,
        ..Limits::default()
    };
    assert_eq!(manifest(host(), Backend::SequentialNet, limits), Err(Error::InvalidLimits));
    let mut wrong = manifest(host(), Backend::SequentialNet, Limits::default()).unwrap();
    wrong.bindings.plan_checker = "caller-asserted-checker".into();
    assert_eq!(encode_manifest(&wrong), Err(Error::Bindings));
}

#[test]
fn program_and_ordered_input_byte_bounds_are_inclusive() {
    let program = compile("-", 2).unwrap();
    let inputs = encode_inputs(&[19, 7]).unwrap();
    for program_bound in [program.len() as u64 - 1, program.len() as u64] {
        let bytes = encode_manifest(
            &manifest(host(), Backend::SequentialNet, Limits {
                max_program_bytes: program_bound,
                max_input_bytes: inputs.len() as u64,
                ..Limits::default()
            })
            .unwrap(),
        )
        .unwrap();
        let actual = execute(
            ExecutionInput {
                manifest_bytes: &bytes,
                program_bytes: &program,
                input_bytes: &inputs,
                host: &host(),
                grants: &grants(),
            },
            &mut |_| Control::Continue,
        );
        if program_bound < program.len() as u64 {
            assert_eq!(actual, Err(Error::Exhausted(Resource::Decode)));
        } else {
            assert_eq!(actual.unwrap().0.outcome, Outcome::Completed(vec![Value::I64(12)]));
        }
    }
    let bytes = encode_manifest(
        &manifest(host(), Backend::SequentialNet, Limits {
            max_input_bytes: inputs.len() as u64 - 1,
            ..Limits::default()
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        execute(
            ExecutionInput {
                manifest_bytes: &bytes,
                program_bytes: &program,
                input_bytes: &inputs,
                host: &host(),
                grants: &grants()
            },
            &mut |_| Control::Continue
        ),
        Err(Error::Exhausted(Resource::Decode))
    );
}

#[test]
fn fixed_producer_plan_scratch_boundary_is_enforced_before_commit() {
    for cap in [255, 256] {
        let actual = run(
            "-",
            &[19, 7],
            Backend::SequentialNet,
            Limits {
                max_plan_scratch_bytes: cap,
                ..Limits::default()
            },
            grants(),
        );
        if cap == 255 {
            assert_eq!(actual.outcome, Outcome::Rejected(Error::Exhausted(Resource::Memory)));
            assert_eq!(actual.counters.interactions, 0);
        } else {
            assert_eq!(actual.outcome, Outcome::Completed(vec![Value::I64(12)]));
        }
    }
}

#[test]
fn frontend_nodes_nesting_and_atomic_turns_have_independent_caps() {
    for limits in [
        Limits {
            max_source_nodes: 1,
            ..Limits::default()
        },
        Limits {
            max_source_depth: 1,
            ..Limits::default()
        },
    ] {
        let actual = run("[[ 1 ] run] run", &[], Backend::SequentialNet, limits, grants());
        assert_eq!(actual.outcome, Outcome::Rejected(Error::Exhausted(Resource::Source)));
    }
    assert_eq!(
        run_result(
            "-",
            &[19, 7],
            Backend::SequentialNet,
            Limits {
                max_atomic_work: 1,
                ..Limits::default()
            },
            grants()
        ),
        Err(Error::Exhausted(Resource::Decode))
    );
}

// r[verify molten.pure_net.outcomes]
// r[verify molten.pure_net.producer]
#[test]
fn zero_active_pair_net_still_requires_complete_ordered_readback() {
    use noble_ngraph::eval;
    use noble_ngraph::net;
    use noble_ngraph::sem;
    let checked = producer::checked(
        &Program {
            source: "swap".into(),
            input_arity: 2,
        },
        &Limits::default(),
        &mut |_, _| Ok(()),
    )
    .unwrap()
    .checked;
    let inputs = eval::i64_inputs(&[19, 7]);
    let mut candidate = net::lower(&producer::Ids, &checked, &inputs, &net::Limits::default()).unwrap();
    assert_eq!(candidate.rule_counts(), (0, 0, 0));
    let limits = net::Limits::default();
    let valid = net::reduce(&producer::Ids, net::ReduceRequest {
        checked: &checked,
        candidate: &candidate,
        inputs: &inputs,
        limits: &limits,
        plan: net::Plan::default(),
    });
    assert_eq!(valid.interactions, 0);
    assert_eq!(valid.obs, net::Obs::Normal(vec![sem::Value::I64(7), sem::Value::I64(19)]));
    candidate.outputs.clear();
    let invalid = net::reduce(&producer::Ids, net::ReduceRequest {
        checked: &checked,
        candidate: &candidate,
        inputs: &inputs,
        limits: &limits,
        plan: net::Plan::default(),
    });
    assert_eq!(invalid.interactions, 0);
    assert!(matches!(invalid.obs, net::Obs::Refused(_) | net::Obs::InternalInvalid(_)));
}

// Independent expressions keep a genuinely wide active queue, unlike a long
// dependency chain. Distinct literals avoid accidental source sharing/CSE.
fn wide_active_queue_source() -> (String, Vec<Value>) {
    let mut source = String::new();
    let mut expected = Vec::new();
    for n in 0..24i64 {
        source.push_str(&format!("{n} {} + ", n + 1000));
        expected.push(Value::I64(1000 + 2 * n));
    }
    (source, expected)
}

#[test]
fn wide_queue_precommit_reservations_replay_without_becoming_interactions() {
    let (source, expected) = wide_active_queue_source();
    let first = run(&source, &[], Backend::SequentialNet, Limits::default(), grants());
    let repeated = run(&source, &[], Backend::SequentialNet, Limits::default(), grants());
    let reference = run(&source, &[], Backend::IndependentReference, Limits::default(), grants());
    assert_eq!(first.outcome, Outcome::Completed(expected.clone()));
    assert_eq!(reference.outcome, Outcome::Completed(expected));
    assert_eq!(encode_receipt(&first).unwrap(), encode_receipt(&repeated).unwrap());
    let reservation = first
        .observations
        .windows(2)
        .find(|pair| pair[0].phase == Phase::Reduce && pair[1].phase == Phase::Validate)
        .expect("wide queue must account for validation between reduction turns");
    assert_eq!(
        reservation[0].interactions, reservation[1].interactions,
        "reserved validation work must not become a semantic rewrite"
    );
}

#[test]
fn wide_queue_validation_and_total_caps_stop_before_the_next_atomic_commit() {
    let (source, _) = wide_active_queue_source();
    let baseline = run(&source, &[], Backend::SequentialNet, Limits::default(), grants());
    let exact = run(
        &source,
        &[],
        Backend::SequentialNet,
        Limits {
            max_validation_work: baseline.counters.validation_work,
            ..Limits::default()
        },
        grants(),
    );
    assert_eq!(exact.outcome, baseline.outcome);
    let exhausted = run(
        &source,
        &[],
        Backend::SequentialNet,
        Limits {
            max_validation_work: baseline.counters.validation_work - 1,
            ..Limits::default()
        },
        grants(),
    );
    assert_eq!(exhausted.outcome, Outcome::Rejected(Error::Exhausted(Resource::Validation)));
    assert_eq!(exhausted.counters.interactions, baseline.counters.interactions - 1);
    let total = run(
        &source,
        &[],
        Backend::SequentialNet,
        Limits {
            max_total_work: baseline.counters.total_work * 3 / 4,
            ..Limits::default()
        },
        grants(),
    );
    assert_eq!(total.outcome, Outcome::Rejected(Error::Exhausted(Resource::TotalWork)));
    assert!(total.counters.interactions > 0);
    assert!(total.counters.interactions < baseline.counters.interactions);
}

#[test]
fn wide_queue_can_cancel_before_any_expensive_precommit_scan() {
    let (source, _) = wide_active_queue_source();
    let manifest = encode_manifest(&manifest(host(), Backend::SequentialNet, Limits::default()).unwrap()).unwrap();
    let mut popped = false;
    let (receipt, _) = execute(
        ExecutionInput {
            manifest_bytes: &manifest,
            program_bytes: &compile(&source, 0).unwrap(),
            input_bytes: &encode_inputs(&[]).unwrap(),
            host: &host(),
            grants: &grants(),
        },
        &mut |observation| {
            if observation.phase == Phase::Reduce {
                popped = true;
            }
            if popped && observation.phase == Phase::Validate {
                Control::Cancel
            } else {
                Control::Continue
            }
        },
    )
    .unwrap();
    assert_eq!(receipt.outcome, Outcome::Rejected(Error::Cancelled));
    assert_eq!(receipt.counters.queue_pops, 1);
    assert_eq!(receipt.counters.interactions, 0);
    assert_eq!(receipt.counters.plan_bytes, 0);
    assert_eq!(receipt.counters.plan_microsteps, 0);
    let tail = &receipt.observations[receipt.observations.len() - 2..];
    assert_eq!(tail[0].phase, Phase::Reduce);
    assert_eq!(tail[1].phase, Phase::Validate);
    assert_eq!(tail[1].interactions, tail[0].interactions);
}

#[test]
fn materialized_wide_queue_atomic_cap_is_checked_before_plan_compilation() {
    use std::cell::Cell;
    use std::cell::RefCell;

    use noble_ngraph::eval;
    use noble_ngraph::net;
    let (source, _) = wide_active_queue_source();
    let checked = producer::checked(&Program { source, input_arity: 0 }, &Limits::default(), &mut |_, _| Ok(()))
        .unwrap()
        .checked;
    let inputs = eval::i64_inputs(&[]);
    let candidate = net::lower(&producer::Ids, &checked, &inputs, &net::Limits::default()).unwrap();
    let baseline = run(&wide_active_queue_source().0, &[], Backend::SequentialNet, Limits::default(), grants());
    let reservation = baseline
        .observations
        .windows(2)
        .find(|pair| pair[0].phase == Phase::Reduce && pair[1].phase == Phase::Validate)
        .expect("a real precommit work reservation is required");
    let required = reservation[1].work - reservation[0].work;
    // Isolate the already-materialized reducer's atomic bound from larger
    // parser/decoder reservations; apply the same manifest cap to every hook.
    for cap in [required - 1, required] {
        let limits = Limits {
            max_atomic_work: cap,
            ..Limits::default()
        };
        let stopped = Cell::new(false);
        let popped = Cell::new(false);
        let reservations = RefCell::new(Vec::new());
        let report = net::reduce_with_hook(
            &producer::Ids,
            net::ReduceRequest {
                checked: &checked,
                candidate: &candidate,
                inputs: &inputs,
                limits: &net::Limits::default(),
                plan: net::Plan::default(),
            },
            &|phase, work, _| {
                if phase == net::WorkPhase::Reduction {
                    popped.set(true);
                }
                if phase == net::WorkPhase::Validation && popped.replace(false) {
                    reservations.borrow_mut().push(work);
                }
                if work > limits.max_atomic_work {
                    stopped.set(true);
                    true
                } else {
                    false
                }
            },
        );
        assert_eq!(reservations.borrow()[0], required);
        if cap < required {
            assert!(stopped.get());
            assert_eq!(report.obs, net::Obs::Cancelled);
            assert_eq!(report.interactions, 0);
            assert_eq!(report.plan_stats.emitted_plan_bytes, 0);
            assert_eq!(report.plan_stats.microsteps, 0);
        } else {
            assert!(!stopped.get());
            assert!(matches!(report.obs, net::Obs::Normal(_)));
            assert_eq!(report.interactions, baseline.counters.interactions);
            assert_eq!(reservations.borrow().len() as u64, report.interactions);
        }
    }
}
