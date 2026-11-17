// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trusted process-owned calculation with a bounded async handoff, not a sandbox.
use std::collections::BTreeMap;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use async_trait::async_trait;
use molten_batch_application::Diagnostic;
use molten_batch_application::Error;
use molten_batch_application::ExecutionOutput;
use molten_batch_application::ExecutionPort;
use molten_batch_application::MaterializedWorkload;
use molten_batch_application::ObservabilityPort;
use molten_batch_core::*;
use molten_pure_net as net;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

use super::InstalledPureNet;
use crate::Disk;

const CANCELLED: u8 = 1;
const DEADLINE: u8 = 2;
const ABANDONED: u8 = 3;

#[derive(Default)]
struct Gate {
    reason: AtomicU8,
    permit: Mutex<bool>,
    changed: Condvar,
}
impl Gate {
    fn stop(&self, reason: u8) {
        // Synchronize notification with the condition check and wait transition.
        let guard = self.permit.lock();
        let _ = self.reason.compare_exchange(0, reason, Ordering::AcqRel, Ordering::Acquire);
        self.changed.notify_all();
        drop(guard);
    }
    fn release(&self) -> Result<(), Error> {
        let mut permit = self.permit.lock().map_err(|_| Error::Unknown)?;
        *permit = true;
        self.changed.notify_one();
        Ok(())
    }
    fn wait(&self) -> bool {
        let Ok(mut permit) = self.permit.lock() else {
            return false;
        };
        while !*permit && self.reason.load(Ordering::Acquire) == 0 {
            let Ok(next) = self.changed.wait(permit) else {
                return false;
            };
            permit = next;
        }
        *permit = false;
        self.reason.load(Ordering::Acquire) == 0
    }
}
struct StopOnDrop(Arc<Gate>);
impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.stop(ABANDONED);
    }
}
struct Slot {
    started: bool,
    gate: Arc<Gate>,
}

pub struct PureNetRuntime {
    disk: Arc<Disk>,
    manifest: Id,
    host: net::HostBindings,
    permits: Arc<Semaphore>,
    slots: Mutex<BTreeMap<Id, Slot>>,
    observations: Arc<dyn ObservabilityPort>,
}
impl PureNetRuntime {
    pub fn for_worker(disk: Arc<Disk>, installed: &InstalledPureNet, observations: Arc<dyn ObservabilityPort>) -> Self {
        Self {
            disk,
            manifest: installed.manifest_ref,
            host: installed.host.clone(),
            permits: Arc::new(Semaphore::new(installed.manifest.limits.max_concurrent_evaluations as usize)),
            slots: Mutex::new(BTreeMap::new()),
            observations,
        }
    }
    fn claim(&self, assignment: Id) -> Result<Arc<Gate>, Error> {
        let mut slots = self.slots.lock().map_err(|_| Error::Unknown)?;
        if !slots.contains_key(&assignment) && slots.len() >= MAX_WORKER_RECORDS {
            return Err(Error::Bound);
        }
        let slot = slots.entry(assignment).or_insert_with(|| Slot {
            started: false,
            gate: Arc::new(Gate::default()),
        });
        if slot.started {
            return Err(Error::Conflict);
        }
        slot.started = true;
        Ok(slot.gate.clone())
    }
}

fn failure(error: &net::Error) -> FailureCode {
    use net::Error as E;
    use net::Resource as R;
    match error {
        E::Malformed => FailureCode::Malformed,
        E::Unsupported => FailureCode::Unsupported,
        E::Bindings | E::InvalidLimits => FailureCode::Rejected,
        E::Incomplete => FailureCode::Incomplete,
        E::Stuck => FailureCode::Stuck,
        E::Cancelled => FailureCode::Interrupted,
        E::Exhausted(resource) => match resource {
            R::Decode | R::Inputs => FailureCode::DecodeLimit,
            R::Source | R::Ir | R::Setup => FailureCode::SetupLimit,
            R::Validation => FailureCode::ValidationLimit,
            R::Nodes | R::Wires => FailureCode::NodeLimit,
            R::Queue => FailureCode::QueueLimit,
            R::Memory => FailureCode::MemoryLimit,
            R::Reduction | R::Reference => FailureCode::ReductionLimit,
            R::Readback => FailureCode::ReadbackLimit,
            R::Observations => FailureCode::ObservationLimit,
            R::TotalWork | R::Attempts => FailureCode::AttemptLimit,
        },
    }
}
fn phase_index(phase: net::Phase) -> usize {
    match phase {
        net::Phase::Decode => 0,
        net::Phase::ParseCheck => 1,
        net::Phase::Extract => 2,
        net::Phase::Lower => 3,
        net::Phase::Validate => 4,
        net::Phase::Setup => 5,
        net::Phase::Reduce => 6,
        net::Phase::Readback => 7,
        net::Phase::Reference => 8,
        net::Phase::Complete => 9,
    }
}
fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}
fn rejected(code: FailureCode) -> ExecutionOutput {
    ExecutionOutput {
        stopped: true,
        output: Err(code),
        report: None,
    }
}

fn finish_calculation(
    result: Result<(net::Receipt, Vec<u8>), net::Error>,
    output_bound: u64,
    deadline: Instant,
    timed_out: bool,
) -> ExecutionOutput {
    // A ready computation may win select! against an equally ready timeout.
    // Successful publication must not depend on which branch was polled first.
    let timed_out = timed_out || Instant::now() >= deadline;
    let (receipt, encoded) = match result {
        Ok(receipt) => receipt,
        Err(error) => {
            return rejected(if timed_out {
                FailureCode::FuelOrDeadline
            } else {
                failure(&error)
            });
        }
    };
    if encoded.len() as u64 > output_bound {
        return rejected(FailureCode::OutputLimit);
    }
    let code = match &receipt.outcome {
        net::Outcome::Completed(_) if !timed_out => {
            return ExecutionOutput {
                stopped: true,
                output: Ok(encoded),
                report: None,
            };
        }
        net::Outcome::Completed(_) => FailureCode::FuelOrDeadline,
        net::Outcome::Rejected(error) if !timed_out => failure(error),
        net::Outcome::Rejected(_) => FailureCode::FuelOrDeadline,
    };
    ExecutionOutput {
        stopped: true,
        output: Err(code),
        report: Some(encoded),
    }
}

#[async_trait]
impl ExecutionPort for PureNetRuntime {
    // r[impl molten.pure_net.quanta]
    // r[impl molten.pure_net.lifecycle]
    // r[impl molten.pure_net.recovery]
    async fn run(
        &self,
        request: ExecutionRequest,
        material: MaterializedWorkload,
        inputs: Vec<Vec<u8>>,
    ) -> Result<ExecutionOutput, Error> {
        let Workload::PureNet {
            manifest: expected_manifest,
            artifact: expected_program,
        } = request.workload
        else {
            return Ok(rejected(FailureCode::Rejected));
        };
        let MaterializedWorkload::PureNet { manifest, artifact } = material else {
            return Ok(rejected(FailureCode::Rejected));
        };
        if expected_manifest != self.manifest
            || manifest.manifest.kind != ContentKind::PureNetManifest
            || artifact.manifest.kind != ContentKind::PureNetProgram
            || crate::content_id(&manifest)? != expected_manifest
            || crate::content_id(&artifact)? != expected_program
            || inputs.len() != 1
        {
            return Ok(rejected(FailureCode::Rejected));
        }
        let gate = self.claim(request.fence.assignment)?;
        if gate.reason.load(Ordering::Acquire) != 0 {
            return Ok(rejected(FailureCode::Interrupted));
        }
        let Ok(permit) = self.permits.clone().try_acquire_owned() else {
            return Ok(rejected(FailureCode::ConcurrencyLimit));
        };
        let _stop_on_drop = StopOnDrop(gate.clone());
        let (events, mut observations) = mpsc::channel::<net::Observation>(1);
        let control = gate.clone();
        let disk = self.disk.clone();
        let host = self.host.clone();
        let assignment = request.fence.assignment;
        let request_epoch = request.fence.epoch;
        let output_bound = request.max_output_bytes;
        let grants = net::ExecutionGrants {
            fuel: request.fuel,
            memory_bytes: request.resources.memory_bytes,
            max_output_bytes: request.max_output_bytes,
            max_attempts: request.max_attempts,
        };
        let mut input = inputs.into_iter();
        let ordered_inputs = input.next().ok_or(Error::Bound)?;
        let deadline = Instant::now() + Duration::from_millis(request.timeout_ms);
        let mut computation = tokio::task::spawn_blocking(move || {
            // An aborted async waiter cannot release exclusive store ownership
            // while its process-owned calculation is still alive.
            let _disk_lifetime = disk;
            let _permit = permit;
            let started = Instant::now();
            let mut mark = started;
            let mut active_phase = net::Phase::Decode;
            let mut phase_ns = [0_u64; 10];
            let mut gate_wait_ns = 0_u64;
            let mut handoffs = 0_u64;
            let result = net::execute(
                net::ExecutionInput {
                    manifest_bytes: &manifest.bytes,
                    program_bytes: &artifact.bytes,
                    input_bytes: &ordered_inputs,
                    host: &host,
                    grants: &grants,
                },
                &mut |observation| {
                    let now = Instant::now();
                    let index = phase_index(active_phase);
                    phase_ns[index] = phase_ns[index].saturating_add(nanos(now.duration_since(mark)));
                    active_phase = observation.phase;
                    mark = now;
                    if now >= deadline {
                        control.stop(DEADLINE);
                    }
                    if control.reason.load(Ordering::Acquire) != 0 {
                        return net::Control::Cancel;
                    }
                    if events.blocking_send(observation.clone()).is_err() {
                        control.stop(ABANDONED);
                        return net::Control::Cancel;
                    }
                    let waiting = Instant::now();
                    let continued = control.wait();
                    gate_wait_ns = gate_wait_ns.saturating_add(nanos(waiting.elapsed()));
                    handoffs = handoffs.saturating_add(1);
                    mark = Instant::now();
                    if continued {
                        net::Control::Continue
                    } else {
                        net::Control::Cancel
                    }
                },
            );
            let index = phase_index(active_phase);
            phase_ns[index] = phase_ns[index].saturating_add(nanos(mark.elapsed()));
            // Diagnostic-only shell measurements: never enter output/semantic identity.
            eprintln!(
                "pure-net-measurement {}",
                serde_json::json!({
                    "assignment": assignment,
                    "request_epoch": request_epoch,
                    "phase_order": ["Decode", "ParseCheck", "Extract", "Lower", "Validate", "Setup", "Reduce", "Readback", "Reference", "Complete"],
                    "phase_ns_excluding_gate_wait": phase_ns,
                    "gate_wait_ns": gate_wait_ns,
                    "handoffs": handoffs,
                    "calculation_wall_ns": nanos(started.elapsed()),
                    "authoritative": false
                })
            );
            result
        });
        let timeout = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline));
        tokio::pin!(timeout);
        let mut timed_out = false;
        let mut observations_open = true;
        let result = loop {
            tokio::select! {
                result = &mut computation => break result.map_err(|_| Error::Unknown)?,
                observation = observations.recv(), if observations_open => {
                    if observation.is_some() {
                        self.observations.record(Diagnostic::Boundary("pure-net-local-quantum"));
                        // The blocking reducer is parked until this async turn
                        // yields; no CPU loop or restart occurs at the boundary.
                        tokio::task::yield_now().await;
                        gate.release()?;
                    } else {
                        observations_open = false;
                    }
                },
                _ = &mut timeout, if !timed_out => {
                    timed_out = true;
                    gate.stop(DEADLINE);
                }
            }
        };
        Ok(finish_calculation(
            result,
            output_bound,
            deadline,
            timed_out || gate.reason.load(Ordering::Acquire) == DEADLINE,
        ))
    }

    fn cancel(&self, assignment: Id) -> Result<(), Error> {
        let mut slots = self.slots.lock().map_err(|_| Error::Unknown)?;
        if !slots.contains_key(&assignment) && slots.len() >= MAX_WORKER_RECORDS {
            return Err(Error::Bound);
        }
        slots
            .entry(assignment)
            .or_insert_with(|| Slot {
                started: false,
                gate: Arc::new(Gate::default()),
            })
            .gate
            .stop(CANCELLED);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify molten.pure_net.bounds]
    #[test]
    fn expired_completion_cannot_publish_when_the_timeout_branch_has_not_run() {
        let host = net::HostBindings {
            executable: Id([1; 32]),
            dependency_closure: Id([2; 32]),
        };
        let manifest = net::encode_manifest(
            &net::manifest(host.clone(), net::Backend::SequentialNet, net::Limits::default()).unwrap(),
        )
        .unwrap();
        let grants = net::ExecutionGrants {
            fuel: 256_000_000,
            memory_bytes: 64 * 1024 * 1024,
            max_output_bytes: 262_144,
            max_attempts: 1,
        };
        let (receipt, encoded) = net::execute(
            net::ExecutionInput {
                manifest_bytes: &manifest,
                program_bytes: &net::compile("-", 2).unwrap(),
                input_bytes: &net::encode_inputs(&[19, 7]).unwrap(),
                host: &host,
                grants: &grants,
            },
            &mut |_| net::Control::Continue,
        )
        .unwrap();
        assert_eq!(receipt.outcome, net::Outcome::Completed(vec![net::Value::I64(12)]));
        let late = finish_calculation(
            Ok((receipt.clone(), encoded.clone())),
            grants.max_output_bytes,
            Instant::now().checked_sub(Duration::from_secs(1)).unwrap(),
            false,
        );
        assert!(late.stopped);
        assert_eq!(late.output, Err(FailureCode::FuelOrDeadline));
        assert_eq!(late.report, Some(encoded.clone()));
        let current = finish_calculation(
            Ok((receipt, encoded.clone())),
            grants.max_output_bytes,
            Instant::now() + Duration::from_secs(3600),
            false,
        );
        assert!(current.stopped);
        assert_eq!(current.output, Ok(encoded));
        assert_eq!(current.report, None);
    }
}
