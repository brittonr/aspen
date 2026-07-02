
    fn flow_parts(target: &FilePath, installed: &JobInstall, sync_ref: &str) -> FlowParts {
        let context_ref = install_job_execute_authority_context(target, &installed.job_ref);
        let source_gate_ref = install_clean_octet_gate(target);
        let resource_refs = vec![test_ref("worker-resource-a"), test_ref("worker-resource-b")];
        let admission_request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &installed.job_ref,
            sync_ref,
            stage_ids: &[],
            target_peer: "peer:b",
            policy_refs: &[test_ref("worker-admission-policy")],
            capability_refs: std::slice::from_ref(&context_ref),
            evidence_refs: &[sync_ref.to_string(), source_gate_ref],
            resource_refs: &resource_refs,
        })
        .expect("worker admission request");
        let admission = admission_loopback(target, &admission_request).expect("worker admission");
        assert_eq!(admission.plan.decision, "pass");
        let admission_ref =
            crate::preserves_rail::canonical_hash(&admission.receipt_value).expect("worker admission ref");
        let execution_request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &installed.job_ref,
            admission_ref: &admission_ref,
            stage_ids: &admission.plan.stage_order,
            target_peer: "peer:b",
            storage_profile_ref: &test_ref("worker-storage-profile"),
            cache_profile_ref: &test_ref("worker-cache-profile"),
            chunk_profile_ref: &test_ref("worker-chunk-profile"),
            policy_refs: &[test_ref("worker-admission-policy")],
            capability_refs: std::slice::from_ref(&context_ref),
            resource_refs: &resource_refs,
        })
        .expect("worker execution request");
        let execution_request_ref =
            crate::preserves_rail::canonical_hash(&execution_request).expect("worker execution request ref");
        FlowParts {
            context_ref,
            resource_refs,
            admission,
            admission_ref,
            execution_request,
            execution_request_ref,
        }
    }

    fn request_parts(installed: &JobInstall, sync_ref: &str, flow: &FlowParts) -> RequestParts {
        let peer_bootstrap_ref = test_ref("worker-peer-bootstrap");
        let identity_ref = test_ref("worker-node-identity");
        let evidence_refs = vec![
            sync_ref.to_string(),
            flow.admission_ref.clone(),
            flow.execution_request_ref.clone(),
            peer_bootstrap_ref.clone(),
            identity_ref.clone(),
        ];
        let worker_request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &installed.job_ref,
            target_peer: "peer:b",
            stage_ids: &flow.admission.plan.stage_order,
            sync_ref,
            admission_ref: &flow.admission_ref,
            execution_request_ref: &flow.execution_request_ref,
            authority_refs: std::slice::from_ref(&flow.context_ref),
            resource_refs: &flow.resource_refs,
            peer_bootstrap_refs: std::slice::from_ref(&peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&identity_ref),
            evidence_refs: &evidence_refs,
        })
        .expect("worker request");
        RequestParts {
            peer_bootstrap_ref,
            identity_ref,
            evidence_refs,
            worker_request,
        }
    }

    fn worker_fixture(name: &str) -> WorkerFixture {
        let root = temp_dir(name);
        let source = root.join("source");
        let target = root.join("target");
        let ledger = root.join("worker-ledger");
        let seed = seed_artifacts(&source);
        let installed_job = seed_graph(&source, &seed);
        let sync_ref = synced_ref(&source, &target, &installed_job, &seed);
        let flow = flow_parts(&target, &installed_job, &sync_ref);
        let request = request_parts(&installed_job, &sync_ref, &flow);
        let (delivery, delivery_log) =
            deliver_worker_request(&root.join("transport"), &request.worker_request, "peer:b", true);
        let FlowParts {
            context_ref,
            resource_refs,
            admission,
            admission_ref,
            execution_request,
            execution_request_ref,
        } = flow;
        let RequestParts {
            peer_bootstrap_ref,
            identity_ref,
            evidence_refs,
            worker_request,
        } = request;
        WorkerFixture {
            root,
            source,
            target,
            ledger,
            installed_job,
            admission,
            sync_ref,
            admission_ref,
            execution_request_ref,
            execution_request,
            context_ref,
            resource_refs,
            peer_bootstrap_ref,
            identity_ref,
            evidence_refs,
            worker_request,
            delivery,
            delivery_log,
        }
    }

    fn deliver_worker_request(
        transport_root: &FilePath,
        request_value: &IoValue,
        target_peer: &str,
        replayable: bool,
    ) -> (crate::remote_dataspace::Delivery, crate::remote_dataspace::DeliveryLog) {
        let envelope = job_worker_envelope(JobWorkerEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "source-worker",
            to_peer: target_peer,
            topic: "molten.job.worker",
            request_value,
        })
        .expect("worker envelope");
        crate::remote_dataspace::publish_local_gossip(transport_root, &envelope, "peer:a")
            .expect("publish worker request");
        let delivery = crate::remote_dataspace::deliver_local_gossip(
            transport_root,
            "molten.job.worker",
            &envelope.envelope_ref,
            target_peer,
        )
        .expect("deliver worker request");
        let delivery_log = crate::remote_dataspace::delivery_log(std::slice::from_ref(&delivery), replayable)
            .expect("worker delivery log");
        (delivery, delivery_log)
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
