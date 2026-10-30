mod state;
mod terminal;

use std::cell;
use std::rc;

use super::admission;
use super::allocation;
use super::continuation;
use super::host;
use super::machine;
use super::receipt;
use super::stream;

#[derive(Clone, Copy)]
enum Phase {
    Allocate,
    Run,
    FreeInput,
    FreeOutput,
}

pub struct Start {
    pub plan: admission::Plan,
    pub binding: continuation::Binding,
    pub current: rc::Rc<dyn continuation::CurrentAdmission>,
    pub host: rc::Rc<dyn host::HostPort>,
    pub input: Vec<u8>,
    pub logical_turn: u64,
}

pub struct Engine {
    machine: Option<machine::Machine>,
    store: ::spacewasm::Store,
    code: ::spacewasm::Vec<::spacewasm::Box<::spacewasm::TextPage>>,
    shared: rc::Rc<host::Shared>,
    plan: admission::Plan,
    binding: continuation::Binding,
    binding_ref: String,
    phase: Phase,
    input: Vec<u8>,
    output: receipt::Output,
    input_ptr: u32,
    output_ptr: u32,
    fuel: u64,
    created: u64,
    terminal: bool,
    events: Vec<receipt::Event>,
    budget: allocation::Budget,
}

impl Engine {
    // r[impl molten.spacewasm_core.execution]
    pub fn start(bytes: &[u8], start: Start) -> Result<Self, continuation::Failure> {
        start.plan.profile.validate().map_err(continuation::Failure::Denied)?;
        let binding_ref = start.binding.identity().map_err(continuation::Failure::Harness)?;
        let limits = &start.plan.profile.limits;
        if start.input.len() as u64 > limits.input_bytes {
            return Err(continuation::Failure::Memory);
        }
        let input =
            crate::codec::parse_canonical_bytes(&start.input).map_err(|_| continuation::Failure::InvalidOutput)?;
        let input_ref =
            crate::codec::canonical_hash(&input).map_err(|e| continuation::Failure::Harness(e.to_string()))?;
        if start.binding.artifact != start.plan.artifact_ref
            || start.binding.profile != start.plan.profile_ref
            || start.binding.authority != start.plan.authority_ref
            || start.binding.resource != start.plan.resource_ref
            || start.binding.actor != start.plan.actor
            || start.binding.generation != start.plan.generation
            || input_ref != start.plan.input_ref()
            || start.binding.input != input_ref
            || start.current.current().map_err(continuation::Failure::Denied)? != start.binding
        {
            return Err(continuation::Failure::Denied("execution binding mismatch".into()));
        }
        super::inspection::inspect(bytes, &start.plan.profile).map_err(continuation::Failure::Denied)?;
        if crate::codec::content_ref_from_bytes(bytes) != start.plan.artifact_ref {
            return Err(continuation::Failure::Denied("artifact changed after admission".into()));
        }
        let budget = allocation::Budget::enter(limits.allocation_bytes as usize)
            .map_err(|e| continuation::Failure::Allocation(format!("{e:?}")))?;
        budget
            .limit_ir(limits.ir_bytes as usize)
            .map_err(|_| continuation::Failure::Allocation("IR-allocation-bound".into()))?;
        let shared = rc::Rc::new(host::Shared {
            binding: start.binding.clone(),
            current: start.current,
            port: start.host,
            limit: limits.hostcall_bytes as usize,
            calls: cell::RefCell::new(Vec::new()),
            failure: cell::RefCell::new(None),
            pending: cell::Cell::new(None),
        });
        let host = host::module(&start.plan.hostcalls, shared.clone()).map_err(continuation::Failure::Allocation)?;
        let mut store =
            ::spacewasm::Store::new(1, [host]).map_err(|e| continuation::Failure::Allocation(format!("{e:?}")))?;
        let mut builder = ::spacewasm::CodeBuilder::<128>::new();
        let allocator = ::spacewasm::Rc::new(allocation::GuestAllocator::new(limits.memory_pages as usize * 65536))
            .map_err(|e| continuation::Failure::Allocation(format!("{e:?}")))?
            .into_wasm_memory_allocator();
        let mut stream = stream::BorrowedStream::new(bytes, 1024)
            .map_err(|e| continuation::Failure::Allocation(format!("{e:?}")))?;
        let module = ::spacewasm::Module::new::<128, 128, 1024>(
            "molten-pilot",
            &mut stream,
            &mut store,
            &mut builder,
            allocator,
            ::spacewasm::CompilerOptions {
                allow_memory_grow: false,
            },
        )
        .map_err(machine::decode_failure)?;
        let (code, _) = builder.finish().map_err(|e| continuation::Failure::Allocation(format!("{e:?}")))?;
        if code.len() as u64 * 512 > limits.ir_bytes {
            return Err(continuation::Failure::Denied("IR bound".into()));
        }
        store.push_module(module);
        let mut state = store
            .allocate(limits.stack_bytes as usize / 4)
            .map_err(|e| continuation::Failure::Allocation(format!("{e:?}")))?;
        machine::invoke(
            &mut state,
            "molten_alloc",
            &[::spacewasm::Value::I32(start.input.len() as i32)],
            limits.stack_bytes as usize,
        )
        .map_err(continuation::Failure::Denied)?;
        let machine = Some(machine::Machine::capture(state));
        drop(stream);
        let mut engine = Self {
            machine,
            store,
            code,
            shared,
            plan: start.plan,
            binding: start.binding,
            binding_ref,
            phase: Phase::Allocate,
            input: start.input,
            output: receipt::Output::Absent,
            input_ptr: 0,
            output_ptr: 0,
            fuel: 0,
            created: start.logical_turn,
            terminal: false,
            events: Vec::new(),
            budget,
        };
        engine.event("instantiation", "exact-streamed-module");
        Ok(engine)
    }

    fn event(&mut self, phase: &str, detail: &str) {
        let usage = self.budget.usage();
        let linear_bytes = if self.store.modules().is_empty() {
            0
        } else {
            u64::from(self.store.get_memory(::spacewasm::ModuleRef(0)).size()) * 65536
        };
        let resource = Some(receipt::Resource {
            interpreter_bytes: usage.allocated as u64,
            interpreter_peak: usage.peak as u64,
            ir_bytes: usage.ir_allocated as u64,
            ir_peak: usage.ir_peak as u64,
            stack_bytes: if self.machine.is_some() {
                self.plan.profile.limits.stack_bytes
            } else {
                0
            },
            linear_bytes,
            instruction_bound: self.fuel,
        });
        self.events.push(receipt::Event {
            phase: phase.into(),
            binding: self.binding_ref.clone(),
            sequence: self.events.len() as u64,
            fuel_granted: self.fuel,
            detail: detail.into(),
            resource,
        });
    }

    pub fn resume(&mut self, context: &continuation::ResumeContext) -> continuation::Progress {
        if self.terminal {
            return continuation::Progress::ProofFailed(continuation::Failure::Harness(
                "terminal-continuation-reused".into(),
            ));
        }
        let current = context.current.current();
        if current.as_ref() != Ok(&self.binding)
            || context.logical_turn < self.created
            || self.shared.current.current().as_ref() != Ok(&self.binding)
            || context.logical_turn - self.created > self.plan.profile.limits.retention_turns
        {
            return self.terminate(Some(continuation::Failure::Denied("stale continuation".into())));
        }
        let result = self.segment();
        match result {
            Ok(progress) => progress,
            Err(error) => self.terminate(Some(error)),
        }
    }

    pub fn cleanup(mut self) -> Vec<receipt::Event> {
        self.release_guest();
        self.event("cleanup", "remaining-controller-drop-scheduled");
        std::mem::take(&mut self.events)
    }
}

impl Engine {
    // r[impl molten.spacewasm_core.resources]
    fn segment(&mut self) -> Result<continuation::Progress, continuation::Failure> {
        let limits = &self.plan.profile.limits;
        let grant = limits.segment_fuel.min(limits.total_fuel.saturating_sub(self.fuel));
        if grant == 0 {
            return Err(continuation::Failure::TotalFuel);
        }
        self.check_retention()?;
        let machine = self.machine.take().ok_or_else(|| continuation::Failure::Harness("missing state".into()))?;
        let mut state = machine.restore(&mut self.store);
        let (result, consumed) = run_segment(&self.code, &mut state, &self.shared, grant as usize);
        self.fuel += consumed as u64;
        let value = state.result;
        self.machine = Some(machine::Machine::capture(state));
        self.event("segment", match &result {
            ::spacewasm::InterpreterResult::Finished => "finished",
            ::spacewasm::InterpreterResult::OutOfFuel => "out-of-fuel",
            ::spacewasm::InterpreterResult::Pause => "host-pause",
            ::spacewasm::InterpreterResult::Trap(_) => "deterministic-trap",
            ::spacewasm::InterpreterResult::ReaderError(_) => "harness-failure",
        });
        match result {
            ::spacewasm::InterpreterResult::OutOfFuel | ::spacewasm::InterpreterResult::Pause => {
                let host_pause = result == ::spacewasm::InterpreterResult::Pause;
                self.event("yield", if host_pause { "host-pause" } else { "out-of-fuel" });
                self.check_retention()?;
                Ok(continuation::Progress::Yield {
                    host_pause,
                    continuation: self.binding.identity().map_err(continuation::Failure::Harness)?,
                })
            }
            ::spacewasm::InterpreterResult::Finished => self.advance(value),
            ::spacewasm::InterpreterResult::Trap(reason) => {
                if let Some(error) = self.shared.failure.borrow_mut().take() {
                    Err(error)
                } else if reason == ::spacewasm::TrapReason::StackOverflow {
                    Err(continuation::Failure::Stack)
                } else {
                    Err(continuation::Failure::Trap(trap_name(reason).into()))
                }
            }
            ::spacewasm::InterpreterResult::ReaderError(_) => Err(continuation::Failure::Harness("invalid-IR".into())),
        }
    }

    fn advance(
        &mut self,
        value: Option<::spacewasm::RawValue>,
    ) -> Result<continuation::Progress, continuation::Failure> {
        let machine = self.machine.take().ok_or_else(|| continuation::Failure::Harness("missing state".into()))?;
        let mut state = machine.restore(&mut self.store);
        let memory = state.store.get_memory(::spacewasm::ModuleRef(0)).clone();
        let (next, name, args) = match self.phase {
            Phase::Allocate => {
                self.input_ptr = value.ok_or(continuation::Failure::Memory)?.read_32();
                memory.store(self.input_ptr as usize, &self.input).map_err(|_| continuation::Failure::Memory)?;
                (Phase::Run, "molten_run", vec![
                    ::spacewasm::Value::I32(self.input_ptr as i32),
                    ::spacewasm::Value::I32(self.input.len() as i32),
                ])
            }
            Phase::Run => {
                let packed = value.ok_or(continuation::Failure::Memory)?.read_64();
                self.output_ptr = (packed >> 32) as u32;
                let length = (packed & 0xffff_ffff) as usize;
                if length as u64 > self.plan.profile.limits.output_bytes {
                    return Err(continuation::Failure::Memory);
                }
                let bytes = memory.load(self.output_ptr as usize, length).map_err(|_| continuation::Failure::Memory)?;
                let mut captured = Vec::new();
                captured
                    .try_reserve_exact(length)
                    .map_err(|_| continuation::Failure::Allocation("output-capture".into()))?;
                captured.extend_from_slice(bytes);
                if crate::codec::parse_canonical_bytes(&captured).is_err() {
                    self.output = receipt::Output::Rejected {
                        bytes: captured,
                        issue: "canonical-validation-failed".into(),
                    };
                    return Err(continuation::Failure::InvalidOutput);
                }
                self.output = receipt::Output::Canonical(captured);
                (Phase::FreeInput, "molten_dealloc", vec![
                    ::spacewasm::Value::I32(self.input_ptr as i32),
                    ::spacewasm::Value::I32(self.input.len() as i32),
                ])
            }
            Phase::FreeInput if self.output_ptr != self.input_ptr => {
                let length = self
                    .output
                    .bytes()
                    .ok_or_else(|| continuation::Failure::Proof("missing-deallocator-output".into()))?
                    .len();
                (Phase::FreeOutput, "molten_dealloc", vec![
                    ::spacewasm::Value::I32(self.output_ptr as i32),
                    ::spacewasm::Value::I32(length as i32),
                ])
            }
            Phase::FreeInput | Phase::FreeOutput => {
                self.machine = Some(machine::Machine::capture(state));
                return Ok(self.terminate(None));
            }
        };
        machine::invoke(&mut state, name, &args, self.plan.profile.limits.stack_bytes as usize)
            .map_err(continuation::Failure::Denied)?;
        self.machine = Some(machine::Machine::capture(state));
        self.phase = next;
        self.check_retention()?;
        Ok(continuation::Progress::Yield {
            host_pause: false,
            continuation: self.binding.identity().map_err(continuation::Failure::Harness)?,
        })
    }

    fn check_retention(&mut self) -> Result<(), continuation::Failure> {
        let memory = self.store.get_memory(::spacewasm::ModuleRef(0)).size() as u128 * 65536;
        let calls = self.shared.calls.borrow();
        let call_bytes: usize = calls
            .iter()
            .map(|call| {
                call.name.capacity() + call.request.capacity() + call.response.capacity() + call.authority.capacity()
            })
            .sum();
        let event_bytes: usize = self
            .events
            .iter()
            .map(|event| event.phase.capacity() + event.binding.capacity() + event.detail.capacity())
            .sum();
        let retained = self.budget.usage().allocated as u128
            + memory
            + (self.input.capacity() + self.output.capacity() + call_bytes + event_bytes) as u128
            + (calls.capacity() * std::mem::size_of::<receipt::Hostcall>()) as u128
            + (self.events.capacity() * std::mem::size_of::<receipt::Event>()) as u128
            + std::mem::size_of::<Self>() as u128;
        if retained > self.plan.profile.limits.continuation_bytes as u128 {
            Err(continuation::Failure::Allocation("continuation-retention-bound".into()))
        } else {
            Ok(())
        }
    }
}

fn run_segment(
    code: &[::spacewasm::Box<::spacewasm::TextPage>],
    state: &mut ::spacewasm::InterpreterState<'_>,
    shared: &host::Shared,
    grant: usize,
) -> (::spacewasm::InterpreterResult, usize) {
    // The upstream runner exposes no consumed count. Single-instruction stepping
    // keeps the total-work law exact and pushes host results before admitted pause.
    for consumed in 1..=grant {
        let result = ::spacewasm::InterpreterRunner::run(&::spacewasm::Interpreter::default(), code, state, 1);
        if shared.pending.take().is_some() {
            return (::spacewasm::InterpreterResult::Pause, consumed);
        }
        if result != ::spacewasm::InterpreterResult::OutOfFuel {
            return (result, consumed);
        }
    }
    (::spacewasm::InterpreterResult::OutOfFuel, grant)
}

fn trap_name(reason: ::spacewasm::TrapReason) -> &'static str {
    match reason {
        ::spacewasm::TrapReason::Unreachable => "unreachable",
        ::spacewasm::TrapReason::Host => "host",
        ::spacewasm::TrapReason::DivideByZero => "divide-by-zero",
        ::spacewasm::TrapReason::InvalidTableIndex => "invalid-table-index",
        ::spacewasm::TrapReason::InvalidTableFunctionType => "invalid-table-function-type",
        ::spacewasm::TrapReason::UninitializedTableElement => "uninitialized-table-element",
        ::spacewasm::TrapReason::GlobalGetFailed => "global-get-failed",
        ::spacewasm::TrapReason::GlobalSetFailed => "global-set-failed",
        ::spacewasm::TrapReason::OutOfMemory => "out-of-memory",
        ::spacewasm::TrapReason::MemoryRefNotUnique => "memory-not-unique",
        ::spacewasm::TrapReason::MemoryOutOfBounds => "memory-out-of-bounds",
        ::spacewasm::TrapReason::StackOverflow => "stack-overflow",
        ::spacewasm::TrapReason::UnrepresentableResult => "unrepresentable-result",
        ::spacewasm::TrapReason::IntegerOverflow => "integer-overflow",
        ::spacewasm::TrapReason::BadConversionToInteger => "bad-integer-conversion",
    }
}
