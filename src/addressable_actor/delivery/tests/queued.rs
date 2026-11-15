use super::*;
use crate::coordination_delivery::DeliveryCommitPort;

struct Queue {
    _temporary: cap_tempfile::TempDir,
    storage: crate::node_state::Scope,
    bridge: Bridge<crate::fabric_time::VirtualClockAdapter>,
}

impl Queue {
    fn new() -> Self {
        let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("scoped node root");
        let root = crate::node_state::DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
        root.create_layout().expect("node layout");
        let storage = root.namespace(crate::node_state::NamespaceKind::Storage).expect("storage namespace");
        let mut policy = policy();
        policy.ready_capacity = 2;
        let profile = time_profile();
        let manifest = DeliveryManifest {
            schema: DELIVERY_MANIFEST_SCHEMA.to_string(),
            extension_id: "actor-delivery".to_string(),
            service_id: "actor-delivery-service".to_string(),
            service_generation: 1,
            implementation_ref: reference("implementation"),
            time_profile_ref: profile.profile_ref.clone(),
            policy_ref: identify_delivery_policy(&policy),
            port_bindings: REQUIRED_DELIVERY_PORTS
                .into_iter()
                .map(|port| (port.to_string(), reference(port)))
                .collect(),
            non_claims: required_delivery_non_claims(),
        };
        let local = crate::coordination_delivery::Local::open(&storage, 1).expect("actual delivery owner");
        let clock = crate::fabric_time::VirtualClockAdapter::new(&profile, 0, 0).expect("clock");
        let bridge = Bridge::new(local, manifest, policy, profile, clock).expect("delivery bridge");
        let mut queue = Self {
            _temporary: temporary,
            storage,
            bridge,
        };
        queue.persist(DeliveryState::empty("actor-queue", &queue.bridge.manifest.policy_ref, 1, 1));
        queue
    }

    fn reopen(self) -> Self {
        let Self {
            _temporary,
            storage,
            bridge,
        } = self;
        let Bridge {
            local,
            manifest,
            policy,
            time_profile,
            ..
        } = bridge;
        drop(local);
        let local = crate::coordination_delivery::Local::open(&storage, 1).expect("reopened delivery owner");
        let clock = crate::fabric_time::VirtualClockAdapter::new(&time_profile, 0, 0).expect("reopened clock");
        let bridge = Bridge::new(local, manifest, policy, time_profile, clock).expect("reopened delivery bridge");
        Self {
            _temporary,
            storage,
            bridge,
        }
    }

    fn enqueue(&self, label: &str) -> DeliveryRequest {
        let item_ref = reference(label);
        observed_request(&self.bridge.manifest, &format!("{label}-enqueue"), DeliveryOperation::Enqueue {
            item_ref: item_ref.clone(),
            content_ref: item_ref.clone(),
            metadata_ref: item_ref,
            metadata_bytes: 0,
        })
    }

    fn state(&self) -> crate::coordination_delivery::PublishedState {
        self.bridge.local.load("actor-queue").expect("actual owner read").expect("retained queue")
    }

    fn persist(&mut self, state: DeliveryState) {
        let prior = self.bridge.local.load("actor-queue").expect("current owner head");
        let observation = self
            .bridge
            .local
            .compare_and_commit(&crate::coordination_delivery::CommitRequest {
                queue_id: "actor-queue".to_string(),
                expected: crate::coordination_delivery::ExpectedState {
                    state_ref: prior.as_ref().map(|head| head.state_ref.clone()),
                    revision: prior.as_ref().map_or(0, |head| head.state.revision),
                },
                next: crate::coordination_delivery::PublishedState::from_state(state),
                requested_engine_epoch: 1,
            })
            .expect("persist through actual delivery CAS");
        assert_eq!(observation.disposition, crate::coordination_delivery::CommitDisposition::Applied);
    }

    fn apply(&mut self, request: &DeliveryRequest) -> Option<DeliveryToken> {
        let prior = self.state();
        let transition = plan_delivery_transition(&DeliveryTransitionInput {
            manifest: &self.bridge.manifest,
            policy: &self.bridge.policy,
            time_profile: &self.bridge.time_profile,
            state: &prior.state,
            request,
        });
        assert_eq!(transition.decision, DeliveryDecisionKind::Applied, "{:?}", transition.issue);
        self.persist(transition.next_state);
        transition.token
    }
}

#[test]
fn request_resumes_after_fifo_predecessor_at_same_and_later_ticks() {
    for resume_tick in [2, 3] {
        let mut queue = Queue::new();
        let older = queue.enqueue("older");
        let mut newer = queue.enqueue("newer");
        assert!(!queue.bridge.retained_enqueue(&newer).expect("absent ready request"));
        queue.apply(&older);
        queue.apply(&newer);
        let older_ref = reference("older");
        let newer_ref = reference("newer");
        let mut claim = observed_request(&queue.bridge.manifest, "newer-claim", DeliveryOperation::Claim);
        let queued = queue.state();
        assert!(queue.bridge.retained_enqueue(&newer).expect("exact retained enqueue"));
        assert_eq!(queue.bridge.claimable_item(&claim).expect("current FIFO preview"), Some(older_ref.clone()));
        assert_eq!(queue.state(), queued, "queueing Y must not claim or complete unrelated X");
        assert!(queue.bridge.active_token("actor-queue", &older_ref).expect("X token").is_none());
        assert!(queue.bridge.active_token("actor-queue", &newer_ref).expect("Y token").is_none());

        let claim_older = observed_request(&queue.bridge.manifest, "older-claim", DeliveryOperation::Claim);
        let token = queue.apply(&claim_older).expect("X claimed by its own request");
        assert_eq!(token.item_ref, older_ref);
        let mut acknowledge =
            observed_request(&queue.bridge.manifest, "older-ack", DeliveryOperation::Acknowledge { token });
        acknowledge.authority_refs.push(queue.bridge.policy.completion_authority_ref.clone());
        queue.apply(&acknowledge);

        newer.logical_tick = resume_tick;
        if resume_tick != 2 {
            newer.operation_id = reference("newer-enqueue-later-tick");
        }
        queue = queue.reopen();
        claim.logical_tick = resume_tick;
        let before_resume = queue.state();
        let repeated_enqueue = plan_delivery_transition(&DeliveryTransitionInput {
            manifest: &queue.bridge.manifest,
            policy: &queue.bridge.policy,
            time_profile: &queue.bridge.time_profile,
            state: &before_resume.state,
            request: &newer,
        });
        if resume_tick == 2 {
            assert_eq!(repeated_enqueue.decision, DeliveryDecisionKind::DuplicateReplay);
        } else {
            assert_eq!(repeated_enqueue.decision, DeliveryDecisionKind::Denied);
            assert_eq!(repeated_enqueue.issue, Some(DeliveryIssue::ItemAlreadyExists));
        }
        assert_eq!(repeated_enqueue.next_state, before_resume.state);
        assert!(queue.bridge.retained_enqueue(&newer).expect("same Y survives retry tick"));
        assert_eq!(queue.bridge.claimable_item(&claim).expect("current Y claim preview"), Some(newer_ref.clone()));
        assert_eq!(queue.state(), before_resume, "retained observations never mutate the owner");
        let resumed = queue.apply(&claim).expect("current fenced Y claim");
        assert_eq!(resumed.item_ref, newer_ref);
        assert_eq!(resumed.claimed_at_tick, resume_tick);
        assert_eq!(
            queue.bridge.active_token("actor-queue", &newer_ref).expect("actual retained Y token"),
            Some(resumed)
        );
        let after = queue.state().state;
        assert!(after.ready.is_empty());
        assert!(after.completed.contains_key(&older_ref));
        assert!(!after.completed.contains_key(&newer_ref));
        assert_eq!(after.in_flight[&newer_ref].item, queued.state.ready[&newer_ref].item);
        assert_eq!(after.next_sequence, queued.state.next_sequence, "retry does not enqueue Y twice");
    }
}

#[test]
fn retained_binding_substitutions_reject_without_mutation() {
    for field in ["item", "content", "metadata", "metadata-bytes", "policy"] {
        let mut queue = Queue::new();
        let enqueue = queue.enqueue("newer");
        queue.apply(&enqueue);
        let mut state = queue.state().state;
        state.revision += 1;
        let item = &mut state.ready.get_mut(&reference("newer")).expect("queued Y").item;
        match field {
            "item" => item.item_ref = reference("substituted"),
            "content" => item.content_ref = reference("substituted"),
            "metadata" => item.metadata_ref = reference("substituted"),
            "metadata-bytes" => item.metadata_bytes = 1,
            "policy" => item.policy_ref = reference("substituted"),
            _ => unreachable!(),
        }
        queue.persist(state);
        let before = queue.state();
        assert!(queue.bridge.retained_enqueue(&enqueue).is_err(), "{field}");
        assert_eq!(queue.state(), before, "{field} mismatch must not mutate retained work");
        assert!(queue.bridge.active_token("actor-queue", &reference("newer")).expect("no claim").is_none());
    }
}

#[test]
fn active_terminal_and_ambiguous_rows_cannot_resume_as_ready_work() {
    for location in ["active", "completed", "dead-letter", "ambiguous"] {
        let mut queue = Queue::new();
        let enqueue = queue.enqueue("newer");
        queue.apply(&enqueue);
        let ready = queue.state().state.ready[&reference("newer")].clone();
        let claim = observed_request(&queue.bridge.manifest, "newer-claim", DeliveryOperation::Claim);
        let token = queue.apply(&claim).expect("claimed Y");
        if location == "completed" {
            let mut ack =
                observed_request(&queue.bridge.manifest, "newer-ack", DeliveryOperation::Acknowledge { token });
            ack.authority_refs.push(queue.bridge.policy.completion_authority_ref.clone());
            queue.apply(&ack);
        } else if location != "active" {
            let mut state = queue.state().state;
            state.revision += 1;
            if location == "ambiguous" {
                state.ready.insert(reference("newer"), ready);
            } else {
                state.in_flight.clear();
                state.dead_letter.insert(reference("newer"), DeadLetterDelivery {
                    item: ready.item,
                    entered_at_tick: 2,
                    cycle: ready.cycle,
                    attempts_in_cycle: 1,
                    total_attempts: 1,
                    reason: "poison".to_string(),
                });
            }
            queue.persist(state);
        }
        let before = queue.state();
        assert!(queue.bridge.retained_enqueue(&enqueue).is_err(), "{location}");
        assert_eq!(queue.state(), before, "{location} is not a fresh enqueue or a resumable ready row");
    }
}
