
    fn deny_case(root: &Path) -> DenyCase {
        let requester = fake_ref("requester-deny");
        let object = fake_ref("object-deny");
        let remotes = vec![fake_ref("remote-a"), fake_ref("remote-b")];
        let peers = vec![fake_ref("peer-a"), fake_ref("peer-b")];
        let empty_refs: &[String] = &[];
        let [policy, authority, support, index, gc] = [
            (ADMISSION_KIND_POLICY, "policy-deny", empty_refs),
            (ADMISSION_KIND_AUTHORITY, "authority-deny", empty_refs),
            (ADMISSION_KIND_SUPPORTING_EVIDENCE, "support-deny", empty_refs),
            (ADMISSION_KIND_REFERENCE_INDEX, "index-deny", empty_refs),
            (ADMISSION_KIND_REMOTE_GC, "remote-gc-deny", remotes.as_slice()),
        ]
        .map(|(kind, label, remote_refs)| {
            seed_ref(SeedInput {
                root,
                kind,
                label: label.to_string(),
                requester_ref: &requester,
                object_ref: &object,
                remote_refs,
            })
        });
        DenyCase {
            requester,
            object,
            remotes,
            peers,
            wrong_peer: fake_ref("peer-wrong"),
            policy,
            authority,
            support,
            index,
            gc,
        }
    }

    fn clear_ref(input: ClearInput<'_>) -> String {
        store_test_remote_clearance(TestRemoteClearanceInput {
            root: input.root,
            label: input.label,
            requester_ref: &input.case.requester,
            peer_ref: input.peer,
            object_ref: &input.case.object,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: input.remote,
            policy_ref: &input.case.policy,
            authority_ref: &input.case.authority,
            is_current: input.is_current,
            revoked_refs: input.revoked_refs,
            retained_refs: input.retained_refs,
        })
    }

    fn denial_refs(root: &Path, case: &DenyCase) -> DenyRefs {
        let empty_refs: &[String] = &[];
        let revoked_refs = vec![fake_ref("remote-revocation")];
        let retained_refs = vec![fake_ref("remote-retained-object")];
        DenyRefs {
            partial: clear_ref(ClearInput {
                root,
                case,
                label: "clearance-a",
                peer: &case.peers[0],
                remote: &case.remotes[0],
                is_current: true,
                revoked_refs: empty_refs,
                retained_refs: empty_refs,
            }),
            wrong_peer: clear_ref(ClearInput {
                root,
                case,
                label: "wrong-peer-clearance",
                peer: &case.wrong_peer,
                remote: &case.remotes[0],
                is_current: true,
                revoked_refs: empty_refs,
                retained_refs: empty_refs,
            }),
            stale: clear_ref(ClearInput {
                root,
                case,
                label: "stale-clearance",
                peer: &case.peers[0],
                remote: &case.remotes[0],
                is_current: false,
                revoked_refs: &revoked_refs,
                retained_refs: empty_refs,
            }),
            retained: clear_ref(ClearInput {
                root,
                case,
                label: "retained-clearance",
                peer: &case.peers[0],
                remote: &case.remotes[0],
                is_current: true,
                revoked_refs: empty_refs,
                retained_refs: &retained_refs,
            }),
        }
    }

    fn assert_denial(root: &Path, case: &DenyCase, evidence: &DestructiveEvidence, reason: &str, expected: &[&str]) {
        let admission = admit_destructive_evidence(DestructiveAdmissionInput {
            root,
            evidence,
            object_ref: &case.object,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect(reason);
        assert_eq!(admission.decision, "deny");
        for needle in expected {
            assert!(
                admission.diagnostics.iter().any(|diagnostic| diagnostic.contains(needle)),
                "missing diagnostic {needle} in {:?}",
                admission.diagnostics
            );
        }
    }

    struct LiveCase {
        requester: String,
        peer: String,
        object: String,
        remote: String,
        policy: String,
        authority: String,
        support: String,
        index: String,
        gc: String,
    }

    fn live_case(root: &Path, label: &str) -> LiveCase {
        let requester = fake_ref(&format!("{label}-requester"));
        let peer = fake_ref(&format!("{label}-peer"));
        let object = fake_ref(&format!("{label}-object"));
        let remote = fake_ref(&format!("{label}-remote"));
        let seeds = seed_set(root, label, &requester, &object, std::slice::from_ref(&remote));
        let [policy, authority, support, index, gc] = seeds;
        LiveCase {
            requester,
            peer,
            object,
            remote,
            policy,
            authority,
            support,
            index,
            gc,
        }
    }

    fn assert_case_pass(root: &Path, case: &LiveCase, clearance: String) {
        let admission = admit_destructive_evidence(DestructiveAdmissionInput {
            root,
            evidence: &DestructiveEvidence {
                requester_ref: Some(case.requester.clone()),
                policy_refs: vec![case.policy.clone()],
                authority_refs: vec![case.authority.clone()],
                evidence_refs: vec![case.support.clone()],
                retained_refs: Vec::new(),
                remote_peer_refs: vec![case.peer.clone()],
                remote_refs: vec![case.remote.clone()],
                reference_index_refs: vec![case.index.clone()],
                remote_gc_refs: vec![case.gc.clone()],
                remote_clearance_refs: vec![clearance],
                is_reference_index_complete: true,
            },
            object_ref: &case.object,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect("admit live clearance");
        assert_eq!(admission.decision, "pass");
        assert!(admission.has_remote_gc_clearance);
    }

    struct Pair {
        request_value: IoValue,
        response_value: IoValue,
        request_ref: String,
        response_ref: String,
    }

    struct Traffic {
        request_ingress: String,
        response_ingress: String,
        request_receive: IoValue,
        response_receive: IoValue,
        request_send: IoValue,
        response_send: IoValue,
    }

    struct Material {
        case: LiveCase,
        request_value: IoValue,
        response_value: IoValue,
        request_control: IoValue,
        response_control: IoValue,
        traffic: Traffic,
    }

    fn request_pair(root: &Path, case: &LiveCase) -> Pair {
        pair_with_label(root, case, "multihost-peer-evidence")
    }

    fn pair_with_label(root: &Path, case: &LiveCase, label: &str) -> Pair {
        let request = store_remote_gc_clearance_request(root, &RemoteGcClearanceRequestInput {
            requester_ref: &case.requester,
            peer_ref: &case.peer,
            object_ref: &case.object,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &case.remote,
            policy_ref: &case.policy,
            authority_ref: &case.authority,
            evidence_refs: std::slice::from_ref(&case.support),
        })
        .expect("request");
        let response = store_remote_gc_clearance_response(RemoteGcClearanceResponseInput {
            root,
            request_value: &request.value,
            evidence_refs: &[fake_ref(label)],
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("response");
        Pair {
            request_value: request.value,
            response_value: response.value,
            request_ref: request.request_ref,
            response_ref: response.response_ref,
        }
    }

    fn control_values(pair: &Pair) -> (IoValue, IoValue) {
        let request_control = remote_clearance_live_control_request_value(&LiveControlRequestInput {
            target_ref: &pair.request_ref,
            payload_ref: None,
            authority_refs: &[],
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: std::slice::from_ref(&pair.request_ref),
        })
        .expect("request control")
        .1;
        let response_control = remote_clearance_live_control_request_value(&LiveControlRequestInput {
            target_ref: &pair.response_ref,
            payload_ref: Some(&pair.request_ref),
            authority_refs: &[],
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[pair.request_ref.clone(), pair.response_ref.clone()],
        })
        .expect("response control")
        .1;
        (request_control, response_control)
    }

    fn traffic_values() -> Traffic {
        let request_ingress = fake_ref("multihost-request-ingress");
        let response_ingress = fake_ref("multihost-response-ingress");
        let request_publish = fake_ref("multihost-request-publish");
        let response_publish = fake_ref("multihost-response-publish");
        let request_receive = fake_live_transport_receipt("receive", "peer-node", "request-envelope", &request_ingress);
        let response_receive =
            fake_live_transport_receipt("receive", "requester-node", "response-envelope", &response_ingress);
        let request_send = fake_live_send_receipt(
            "requester-node",
            "peer-node",
            "request-envelope",
            &request_publish,
            "request-ticket",
        );
        let response_send = fake_live_send_receipt(
            "peer-node",
            "requester-node",
            "response-envelope",
            &response_publish,
            "response-ticket",
        );
        Traffic {
            request_ingress,
            response_ingress,
            request_receive,
            response_receive,
            request_send,
            response_send,
        }
    }
