    // r[verify molten.runtime_spine.turn_causality]
    type Cause = super::super::RuntimeTurnCause;
    type CauseIssue = super::super::RuntimeCauseIssue;

    const TURN_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const OTHER_TURN_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const CAUSE_TEXT: &str = "runtime-turn-cause-v1";

    fn assert_step(actor: &str) -> Step {
        Step::Assert {
            actor: actor.to_string(),
            value: Value::string("service.ready").expect("the asserted value"),
        }
    }

    fn committed_turn_record(events: &[Event]) -> &Event {
        events
            .iter()
            .find(|event| matches!(event, Event::TurnCommitted { .. }))
            .expect("the committed turn record")
    }

    #[test]
    fn a_dependency_driven_turn_names_the_releasing_turn() {
        // r[verify molten.runtime_spine.turn_causality]
        let mut state = RuntimeState::new(7);
        let cause = Cause::PriorTurn {
            turn_ref: TURN_REF.to_string(),
        };
        let (events, _receipt) = state
            .apply_step_with_cause(&assert_step("actor:a"), &cause)
            .expect("the turn commits");
        let record = committed_turn_record(&events);
        let text = crate::preserves_rail::to_text(&record.to_value()).expect("the turn record text");
        assert!(text.contains("prior-turn"), "unexpected record: {text}");
        assert!(text.contains(TURN_REF), "unexpected record: {text}");
        assert_eq!(Cause::from_value(&cause.to_value()).expect("the cause decodes"), cause);
    }

    #[test]
    fn a_cleanup_turn_names_cleanup() {
        // r[verify molten.runtime_spine.turn_causality]
        let mut state = RuntimeState::new(11);
        state.apply_step(&assert_step("actor:a"));
        let cleanup = state.cleanup_actor_scope("actor:a").expect("the scope cleanup");
        assert_eq!(cleanup.cause, Cause::Cleanup);
        assert_eq!(cleanup.cause.as_str(), "cleanup");
    }

    #[test]
    fn a_delayed_turn_names_its_causing_turn_and_amount() {
        // r[verify molten.runtime_spine.turn_causality]
        let mut state = RuntimeState::new(13);
        let cause = Cause::Delay {
            causing_turn_ref: OTHER_TURN_REF.to_string(),
            amount_ticks: 25,
        };
        let (events, _receipt) = state
            .apply_step_with_cause(&assert_step("actor:a"), &cause)
            .expect("the turn commits");
        let text = crate::preserves_rail::to_text(&committed_turn_record(&events).to_value())
            .expect("the turn record text");
        assert!(text.contains("delay"), "unexpected record: {text}");
        assert!(text.contains(OTHER_TURN_REF), "unexpected record: {text}");
        assert!(text.contains("25"), "unexpected record: {text}");
    }

    #[test]
    fn the_cause_vocabulary_is_closed() {
        // r[verify molten.runtime_spine.turn_causality]
        let unknown = crate::preserves_rail::record(CAUSE_TEXT, vec![crate::preserves_rail::string("guess")]);
        let issue = Cause::from_value(&unknown).expect_err("an unknown cause is rejected");
        assert!(matches!(issue, CauseIssue::UnknownCause(kind) if kind == "guess"));

        let absent = crate::preserves_rail::record(CAUSE_TEXT, Vec::new());
        let issue = Cause::from_value(&absent).expect_err("a record without a cause is rejected");
        assert!(matches!(issue, CauseIssue::MalformedCause(_)));

        let not_a_cause = crate::preserves_rail::record("runtime-event-message-delivered-v1", vec![
            crate::preserves_rail::string("actor:a"),
        ]);
        let issue = Cause::from_value(&not_a_cause).expect_err("a value outside the vocabulary is rejected");
        assert!(matches!(issue, CauseIssue::MalformedCause(_)));
    }

    #[test]
    fn a_replay_whose_cause_differs_compares_as_divergent() {
        // r[verify molten.runtime_spine.turn_causality]
        let mut first = RuntimeState::new(17);
        let mut second = RuntimeState::new(17);
        let (first_events, _first_receipt) = first
            .apply_step_with_cause(
                &assert_step("actor:a"),
                &Cause::PriorTurn {
                    turn_ref: TURN_REF.to_string(),
                },
            )
            .expect("the turn commits");
        let (second_events, _second_receipt) = second
            .apply_step_with_cause(
                &assert_step("actor:a"),
                &Cause::PriorTurn {
                    turn_ref: OTHER_TURN_REF.to_string(),
                },
            )
            .expect("the turn commits");
        let first_ref = committed_turn_record(&first_events).event_ref().expect("the first turn ref");
        let second_ref = committed_turn_record(&second_events).event_ref().expect("the second turn ref");
        assert_ne!(first_ref, second_ref);

        let mut third = RuntimeState::new(17);
        let (same_events, _same_receipt) = third
            .apply_step_with_cause(
                &assert_step("actor:a"),
                &Cause::PriorTurn {
                    turn_ref: TURN_REF.to_string(),
                },
            )
            .expect("the turn commits");
        let same_ref = committed_turn_record(&same_events).event_ref().expect("the repeated turn ref");
        assert_eq!(first_ref, same_ref, "the same cause replays to the same turn record");
    }
