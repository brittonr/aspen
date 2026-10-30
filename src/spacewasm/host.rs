use std::cell;
use std::ops;
use std::rc;

use super::allocation;
use super::continuation;
use super::receipt;

pub struct Reply {
    pub bytes: Vec<u8>,
    pub pause: bool,
}

pub trait HostPort {
    /// Must re-resolve current Basalt/UCAN, policy and resource admission before dispatch.
    fn dispatch(
        &self,
        binding: &continuation::Binding,
        name: &str,
        bytes: &[u8],
    ) -> Result<Reply, continuation::Failure>;
}

pub(crate) struct Shared {
    pub binding: continuation::Binding,
    pub current: rc::Rc<dyn continuation::CurrentAdmission>,
    pub port: rc::Rc<dyn HostPort>,
    pub limit: usize,
    pub calls: cell::RefCell<Vec<receipt::Hostcall>>,
    pub failure: cell::RefCell<Option<continuation::Failure>>,
    pub pending: cell::Cell<Option<u64>>,
}

// r[impl molten.spacewasm_core.execution]
pub(crate) fn module(names: &[String], shared: rc::Rc<Shared>) -> Result<::spacewasm::HostModule, String> {
    let mut functions = ::spacewasm::Vec::new(names.len() as u32).map_err(|e| format!("{e:?}"))?;
    for name in names {
        let name = match name.as_str() {
            "assert" => "assert",
            "send" => "send",
            _ => return Err("undeclared hostcall".into()),
        };
        let shared = shared.clone();
        let callback = move |state: &::spacewasm::InterpreterState<'_>, args: &[::spacewasm::Value]| match invoke(
            state, args, name, &shared,
        ) {
            Ok(reply) => {
                if reply.pause {
                    shared.pending.set(Some(reply.descriptor));
                }
                ops::ControlFlow::Continue(Some(::spacewasm::Value::I64(reply.descriptor as i64)))
            }
            Err(error) => {
                *shared.failure.borrow_mut() = Some(error);
                ops::ControlFlow::Break(::spacewasm::HostFunctionBreak::Trap)
            }
        };
        allocation::reserve(&callback).map_err(|e| format!("{e:?}"))?;
        functions.push(::spacewasm::HostFunction::new(
            name,
            ::spacewasm::HostValList::new("ii"),
            ::spacewasm::HostValList::new("I"),
            callback,
        ));
    }
    Ok(::spacewasm::HostModule {
        name: "molten:hostcall",
        functions,
        globals: ::spacewasm::Vec::new(0).map_err(|e| format!("{e:?}"))?,
        memory: ::spacewasm::Vec::new(0).map_err(|e| format!("{e:?}"))?,
        table: ::spacewasm::Vec::new(0).map_err(|e| format!("{e:?}"))?,
    })
}

struct Completed {
    descriptor: u64,
    pause: bool,
}

fn invoke(
    state: &::spacewasm::InterpreterState<'_>,
    args: &[::spacewasm::Value],
    name: &str,
    shared: &Shared,
) -> Result<Completed, continuation::Failure> {
    let [::spacewasm::Value::I32(ptr), ::spacewasm::Value::I32(len)] = args else {
        return Err(continuation::Failure::Trap("hostcall-signature".into()));
    };
    if *ptr < 0 || *len < 0 || *len as usize > shared.limit {
        return Err(continuation::Failure::Memory);
    }
    let bytes = state.memory.load(*ptr as usize, *len as usize).map_err(|_| continuation::Failure::Memory)?;
    let request = crate::codec::parse_canonical_bytes(bytes)
        .map_err(|_| continuation::Failure::Trap("noncanonical-hostcall-request".into()))?;
    if shared.current.current().map_err(continuation::Failure::Denied)? != shared.binding {
        return Err(continuation::Failure::Denied("stale hostcall authority".into()));
    }
    let request_ref =
        crate::codec::canonical_hash(&request).map_err(|e| continuation::Failure::Harness(e.to_string()))?;
    shared
        .calls
        .borrow_mut()
        .try_reserve(1)
        .map_err(|_| continuation::Failure::Allocation("hostcall-log".into()))?;
    let result = shared.port.dispatch(&shared.binding, name, bytes);
    let response = match &result {
        Ok(reply) => receipt::Response::Returned(crate::codec::content_ref_from_bytes(&reply.bytes)),
        Err(error) => receipt::Response::Failed(error.clone()),
    };
    shared.calls.borrow_mut().push(receipt::Hostcall {
        name: name.into(),
        request: request_ref,
        response,
        authority: shared.binding.authority.clone(),
    });
    let reply = result?;
    if reply.bytes.len() > shared.limit {
        return Err(continuation::Failure::Uncertain("post-effect-response-budget".into()));
    }
    crate::codec::parse_canonical_bytes(&reply.bytes)
        .map_err(|_| continuation::Failure::Uncertain("post-effect-noncanonical-response".into()))?;
    state
        .memory
        .store(4096, &reply.bytes)
        .map_err(|_| continuation::Failure::Uncertain("post-effect-response-memory".into()))?;
    Ok(Completed {
        descriptor: (4096u64 << 32) | reply.bytes.len() as u64,
        pause: reply.pause,
    })
}
