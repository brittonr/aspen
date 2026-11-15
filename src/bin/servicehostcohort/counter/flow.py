#!/usr/bin/env python3
"""Run Aspen's pinned measured-counter flow; authorship alone never claims this passed.

Uses the existing internal molten CLI names. Operator keys stay in external user-owned
files; the host receives only an exact request-ref handshake after independent signing.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import dialogue


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source-trust", "signed-output", "issuer-key", "holder-key", "policy"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--negative-cohort", type=Path, help="Separately authored fresh cohort for actual post-semantic SIGKILL uncertainty")
    parser.add_argument("--boundary-cohort", type=Path, help="Separately authored fresh cohort for live stale/key/placement/survival denies")
    parser.add_argument("--ordering-cohort", type=Path, help="Separately authored fresh cohort for older-ready/new-request FIFO isolation")
    parser.add_argument("--qualify-revocation", action="store_true", help="Explicitly qualify real UCAN proof revocation in the separate boundary cohort")
    args = parser.parse_args()
    if args.timeout <= 0 or (args.qualify_revocation and args.boundary_cohort is None):
        parser.error("positive timeout and --boundary-cohort are required for explicit revocation qualification")
    root = Path(__file__).resolve().parent
    config = json.loads((root / "flow-config.json").read_text())
    if str(args.signed_output) != config["signed_output"]:
        raise RuntimeError("explicit signed output differs from authored source selection")
    molten = args.signed_output / "bin/molten"
    authorizer = args.signed_output / "bin/molten-service-host-cohort"
    transcript = []
    # An owned temporary directory contains only current public previews and invocation nonces.
    # Never copy either private seed or include it in the execution transcript.
    with tempfile.TemporaryDirectory(prefix="aspen-counter-dialogue-") as temporary:
        temporary = Path(temporary)
        sequence = 0

        def invoke(action, tick, body=None, kill_after_native=False, expect_failure=False, configuration=None, revoke_current=False):
            nonlocal sequence
            sequence += 1
            command = [str(molten), "addressable-actor", action, "--state-root", str(root),
                       "--logical-tick", str(tick), "--interactive-signing"]
            if configuration is not None:
                command += ["--configuration", configuration]
            input_path = root / ("counter-command-%03d.json" % sequence)
            if body is not None:
                with input_path.open("x") as stream:
                    json.dump(body, stream, sort_keys=True, separators=(",", ":"))
                command += ["--input", input_path.name]
            pid, exit_code, signing_requests, result = dialogue.execute(
                command, root, config, args, temporary, authorizer,
                kill_after_native=kill_after_native, expect_failure=expect_failure,
                revoke_current=revoke_current)
            # Every call has exited before the next reopen; this is actual process-loss/reopen scope.
            transcript.append({"action": action, "logical_tick": tick, "pid": pid,
                               "exit_code": exit_code, "request_refs": signing_requests, "result": result})
            return result

        def bootstrap():
            node_configuration = json.loads((root / "actor-configuration.json").read_text())
            initialized_node = subprocess.run([str(molten), "node", "init", "--state-root", str(root),
                "--node-id", node_configuration["node_id"], "--config-out", str(root / "node-init-config.preserves"),
                "--identity-receipt-out", str(root / "node-init-receipt.preserves"),
                "--profile-resolution-out", str(root / "node-init-profile-resolution.preserves")],
                check=True, capture_output=True, text=True, timeout=args.timeout)
            with (root / "node-init.stdout").open("x") as stream:
                stream.write(initialized_node.stdout)
            transcript.append({"action": "node-init", "exit_code": initialized_node.returncode,
                "receipt": (root / "node-init-receipt.preserves").read_text(), "summary": initialized_node.stdout})
            invoke("native", 2, {"kind": "install", "group": "molten.counter.delivery"})
            invoke("native", 3, {"kind": "install", "group": "molten.counter.actor"})
            proposed = invoke("placement-init", 4)
            require(proposed["state"] == "proposed", "placement was not initialized Proposed")
            for tick, kind in enumerate(("reserve", "assign", "acknowledge", "activate"), 5):
                placed = invoke("placement-step", tick, {"kind": kind})
            require(placed["state"] == "active", "actual native placement activation is absent")
            drained = invoke("native", 9, {"kind": "sleep"})
            require(drained["phase"] == "drained", "initial native checkpoint/drain did not complete")
            require(drained["checkpoint_ref"] == config["expected_initial_ref"], "actual initialized semantic bytes differ from selected key")
            initialized = invoke("initialize", 10)
            require(initialized["phase"] == "dormant", "public consumer did not initialize from real checkpoint")

        bootstrap()
        pending_before = invoke("native", 11, {"kind": "observe"})
        pending = invoke("result", 12, {"payload": "request-1.preserves"})
        require(pending["semantic_outcome"] == "unknown" and pending["reconciliation_required"],
                "read-before-native-completion was not an explicit unknown result")
        pending_after = invoke("native", 13, {"kind": "observe"})
        for key in ("state_ref", "generation", "incarnation", "callback_sequence", "event_sequence"):
            require(pending_before[key] == pending_after[key], "pending result read changed native " + key)
        first = invoke("apply", 20, {"action": "request", "payload": "request-1.preserves"})
        require(first["semantic_outcome"] == "applied", "first request has no durable actor+delivery completion")
        require(first["counter"]["value"] == 1, "first native result is not one")
        slept = invoke("apply", 30, {"action": "sleep", "operation_id": "pinned-sleep-one"})
        require(slept["phase"] == "dormant", "public actor did not enter durable sleep")
        checkpoint = invoke("native", 31, {"kind": "checkpoint-value"})
        require(checkpoint["state_ref"] == config["expected_checkpoint_refs"][0], "actual checkpoint differs from exact semantic bytes")
        before = invoke("native", 32, {"kind": "observe"})
        require(before["phase"] == "drained" and before["unresolved"] == 0, "sleep left native work unresolved")
        # A new process opens the same owners and invokes actual checkpoint restore/wake for request two.
        second = invoke("apply", 40, {"action": "request", "payload": "request-2.preserves"})
        require(second["semantic_outcome"] == "applied" and second["counter"]["value"] == 3,
                "restored counter request did not durably produce three")
        after = invoke("native", 41, {"kind": "observe"})
        require(after["phase"] == "running" and after["incarnation"] > before["incarnation"],
                "restore/wake did not acquire a fresh native incarnation")
        replay = invoke("apply", 42, {"action": "request", "payload": "request-1.preserves"})
        require(replay["semantic_outcome"] == "duplicate-replay" and replay["result"]["value"] == 1,
                "exact logical replay did not preserve the original result")
        final = invoke("result", 43)
        require(final["value"] == 3 and len(final["completed"]) == 2, "duplicate replay changed semantic counter state")
        require(final["state_ref"] == config["expected_checkpoint_refs"][1], "final semantic bytes differ from exact authored assertion")
        read_before = invoke("native", 44, {"kind": "observe"})
        original_result = invoke("result", 45, {"payload": "request-1.preserves"})
        require(original_result["semantic_outcome"] == "applied" and original_result["result"]["value"] == 1,
                "public result did not retain the first original value after later work")
        require(original_result["result"]["acknowledgement_ref"] and original_result["result"]["actor_completion_receipt_ref"],
                "original result omitted exact acknowledgement or successful actor receipt identity")
        for key in ("value", "native_event_ref", "native_commit_ref", "semantic_state_ref",
                    "acknowledgement_ref", "actor_completion_receipt_ref"):
            require(original_result["result"][key] == replay["result"][key],
                    "duplicate original result changed retained evidence " + key)
        read_after = invoke("native", 46, {"kind": "observe"})
        for key in ("state_ref", "generation", "incarnation", "callback_sequence", "event_sequence"):
            require(read_before[key] == read_after[key], "original result read retried an effect or changed native " + key)
        evidence = {"schema": "aspen-measured-counter-execution-v1", "bundle_ref": config["bundle_ref"],
                    "actor_key_ref": config["actor_key_ref"], "outcome": "pinned-flow-completed", "commands": transcript,
                    "non_claims": ["not-production-admission", "not-machine-loss-durability", "not-distributed-availability"]}
        # Only actual completed executions reach this exclusive evidence publication.
        with (root / "counter-execution.json").open("x") as stream:
            json.dump(evidence, stream, sort_keys=True, separators=(",", ":"))
        print(json.dumps({"execution_evidence": str(root / "counter-execution.json"),
                          "outcome": evidence["outcome"]}, sort_keys=True))

        selected_roots = {root.resolve()}

        def select_negative(path):
            nonlocal root, config, transcript
            candidate = path.resolve()
            require(candidate not in selected_roots, "negative cohorts must be distinct independently authored fresh roots")
            selected_roots.add(candidate)
            root = candidate
            config = json.loads((root / "flow-config.json").read_text())
            require(config["signed_output"] == str(args.signed_output), "negative cohort selects different independently signed output")
            transcript = []
            bootstrap()

        def unchanged(before, after):
            for key in ("state_ref", "generation", "incarnation", "callback_sequence", "event_sequence"):
                require(before[key] == after[key], "denied/read-only operation changed actual native " + key)

        def publish_negative(scenario):
            with (root / "counter-negative-execution.json").open("x") as stream:
                json.dump({"schema": "aspen-measured-counter-negative-execution-v1", "scenario": scenario,
                    "bundle_ref": config["bundle_ref"], "commands": transcript,
                    "non_claims": ["not-production-admission", "authored-campaign-is-not-execution"]}, stream,
                    sort_keys=True, separators=(",", ":"))

        if args.negative_cohort is not None:
            select_negative(args.negative_cohort)
            killed = invoke("apply", 20, {"action": "request", "payload": "request-1.preserves"}, kill_after_native=True)
            require(killed.get("injected_sigkill") is True, "fault did not reach actual post-semantic commit signing boundary")
            before = invoke("native", 21, {"kind": "observe"})
            unknown = invoke("result", 22, {"payload": "request-1.preserves"})
            require(unknown["semantic_outcome"] == "unknown" and unknown["reconciliation_required"],
                "post-semantic crash was incorrectly reported as completed")
            unknown_again = invoke("apply", 23, {"action": "request", "payload": "request-1.preserves"})
            require(unknown_again["semantic_outcome"] == "unknown" and unknown_again["reconciliation_required"],
                "unconfirmed semantic publication was automatically redispatched")
            after = invoke("native", 24, {"kind": "observe"})
            unchanged(before, after)
            publish_negative("actual-post-semantic-sigkill-unknown-no-automatic-retry")

        if args.ordering_cohort is not None:
            select_negative(args.ordering_cohort)
            actor = json.loads((root / "actor-configuration.json").read_text())
            manifest = json.loads((root / "delivery-manifest.json").read_text())
            older = config["expected_initial_ref"]

            def delivery_request(operation_id, tick, operation):
                return {"schema": "molten.coordination-delivery-request.v1", "queue_id": actor["queue_id"],
                    "operation_id": operation_id, "actor_id": actor["node_id"],
                    "service_generation": manifest["service_generation"], "consistency_epoch": 1, "engine_epoch": 1,
                    "time_profile_ref": manifest["time_profile_ref"], "logical_tick": tick, "currentness": "linearizable",
                    "authority_refs": [actor["profile"]["authority_profile_ref"]], "policy_refs": [manifest["policy_ref"]],
                    "resource_refs": [actor["profile"]["resource_profile_ref"]],
                    "evidence_refs": [actor["profile"]["evidence_profile_ref"]], "operation": operation}

            enqueued = invoke("delivery", 20, delivery_request(config["expected_initial_ref"], 20,
                {"kind": "enqueue", "item_ref": older, "content_ref": older, "metadata_ref": older, "metadata_bytes": 0}))
            require(enqueued["transition"]["decision"] == "applied", "older ready item was not admitted")
            before_queued = invoke("native", 21, {"kind": "observe"})
            queued = invoke("apply", 22, {"action": "request", "payload": "request-1.preserves"})
            require(queued["semantic_outcome"] == "progress" and queued["queued"],
                    "new Y claimed or executed older X instead of returning queued progress")
            unchanged(before_queued, invoke("native", 23, {"kind": "observe"}))
            pending = invoke("result", 24, {"payload": "request-1.preserves"})
            require(pending["semantic_outcome"] == "unknown" and pending["reconciliation_required"],
                    "queued Y acquired a fabricated completed result")
            claimed = invoke("delivery", 25, delivery_request(config["expected_checkpoint_refs"][0], 25, {"kind": "claim"}))
            require(claimed["transition"]["decision"] == "applied" and claimed["transition"]["token"]["item_ref"] == older,
                    "X was no longer ready for its own independently authorized claim")
            state = invoke("result", 26)
            require(state["value"] == 0 and state["completed"] == [], "FIFO isolation executed a semantic counter request")
            publish_negative("older-X-new-Y-queued-progress-without-unrelated-claim-or-effect")

        if args.boundary_cohort is not None:
            select_negative(args.boundary_cohort)
            baseline = invoke("native", 11, {"kind": "observe"})
            denied_key = invoke("apply", 12, {"action": "request", "payload": "request-wrong-key.preserves"}, expect_failure=True)
            require(denied_key.get("denied") is True, "wrong actor key was admitted")
            unchanged(baseline, invoke("native", 13, {"kind": "observe"}))
            fence = invoke("fence", 14)
            fence["generation"] += 1
            stale = invoke("apply", 15, {"operation_id": "negative-stale-generation", "fence": fence,
                "operation": {"kind": "idle-sleep", "checkpoint_ref": baseline["checkpoint_ref"],
                    "pending_mailbox_items": 0, "unresolved_effects": 0}})
            require(stale["semantic_outcome"] in ("stale", "denied"), "stale generation was applied")
            unchanged(baseline, invoke("native", 16, {"kind": "observe"}))
            wrong = json.loads((root / "actor-configuration.json").read_text())
            wrong["assignment_id"] = "not-the-current-placement"
            with (root / "wrong-placement-configuration.json").open("x") as stream:
                json.dump(wrong, stream, sort_keys=True, separators=(",", ":"))
            denied_placement = invoke("fence", 17, expect_failure=True, configuration="wrong-placement-configuration.json")
            require(denied_placement.get("denied") is True, "substituted placement was admitted")
            unchanged(baseline, invoke("native", 18, {"kind": "observe"}))
            if args.qualify_revocation:
                public_trust = root / "service-host-trust"
                original = public_trust.read_bytes()
                try:
                    rejected = invoke("fence", 19, expect_failure=True, revoke_current=True)
                    require(rejected.get("denied") is True and len(transcript[-1]["request_refs"]) == 1,
                        "actual Current did not reject the independently signed and then actually revoked capability")
                finally:
                    public_trust.write_bytes(original)
                unchanged(baseline, invoke("native", 20, {"kind": "observe"}))
            recovering = invoke("apply", 21, {"operation_id": "negative-enter-recovery",
                "operation": {"kind": "begin-recovery", "checkpoint_ref": baseline["checkpoint_ref"]}})
            require(recovering["phase"] == "recovering", "survival boundary did not enter actual recovery phase")
            before_survival = invoke("native", 22, {"kind": "observe"})
            for index, survival_class in enumerate(("processes", "streams", "sessions", "partial-callbacks", "in-flight-deltas")):
                survival = invoke("apply", 23 + 2 * index, {"operation_id": "negative-survival-" + survival_class,
                    "operation": {"kind": "recovery-succeeded", "checkpoint_ref": baseline["checkpoint_ref"],
                        "restored_classes": [survival_class], "durable_state_ref": baseline["checkpoint_ref"]}}, expect_failure=True)
                require(survival["semantic_outcome"] == "denied", "nondurable survival claim was admitted: " + survival_class)
                unchanged(before_survival, invoke("native", 24 + 2 * index, {"kind": "observe"}))
            publish_negative("actual-key-stale-placement-" + ("revocation-" if args.qualify_revocation else "") + "nondurable-survival-denies")


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


if __name__ == "__main__":
    main()
