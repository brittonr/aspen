# Content-replication verification

Recorded on 2026-08-27.

All declared fabric dependencies are archived. The same-core simulation archive is `.cairn/archive/2026-08-01-fabric-whole-system-simulation`.

## Pure core

The pure core defines the manifest, policies, epochs, inventory, operation history, actions, plan, status, issues, and non-claims.

The planner computes deterministic transfers, repairs, handoffs, reuse, deferrals, retention pins, and cleanup candidates.

Resume and operation identity bind the service generation, membership epoch, placement epoch, content, source, receiver, action, and attempt.

Nine positive and negative core tests pass. The tests cover stable ordering, placement domains, stale epochs, corruption, idempotency, conflicts, repair exhaustion, handoff, retention, cleanup, protected content, resources, and malformed manifests.

## Shell and adapters

The supervised shell binds authority, identity, membership, placement, time, content, transport, durable-state, retention, resources, observations, and receipts.

Nine shell tests cover receipt-last ordering, authority denial, stale placement, unsolicited delivery, cancellation, restart, cleanup, deterministic faults, live Iroh loopback, local content, and simulated durability.

Three multiprocess product tests move exact content bytes through two child processes. They cover transfer, repair, and wrong-payload denial.

The generic distinct-process harness now indexes its explicit request and payload inputs. Existing default CLI harness tests remain green.

The full all-target, all-feature package command passed 1,314 root-library tests, 216 core tests, 51 binary tests, 61 CLI tests, and 12 content-replication integration tests.

The focused Octet workspace passes with zero findings, warnings, and errors. Core and root Clippy pass for all targets and features with warnings denied.

`nix flake check --no-build --builders ''` passed. Strict Cairn validation and the proposal, design, and tasks gates also passed.

## Current non-claims

These checks prove the bounded tested profiles only. They do not prove permanent durability, global availability, or release eligibility.
