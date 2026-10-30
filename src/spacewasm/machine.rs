pub(crate) struct Machine {
    pc: ::spacewasm::JumpTarget,
    jumped: bool,
    fp: u32,
    sp: usize,
    stack: ::spacewasm::Stack,
    memory: ::spacewasm::Rc<::spacewasm::Memory>,
    table: ::spacewasm::Rc<[::spacewasm::TableElement]>,
    module: ::spacewasm::ModuleRef,
    result: Option<::spacewasm::RawValue>,
}

impl Machine {
    pub fn capture(state: ::spacewasm::InterpreterState<'_>) -> Self {
        Self {
            pc: state.pc,
            jumped: state.jumped,
            fp: state.fp,
            sp: state.sp,
            stack: state.stack,
            memory: state.memory,
            table: state.table,
            module: state.module,
            result: state.result,
        }
    }

    pub fn restore(self, store: &mut ::spacewasm::Store) -> ::spacewasm::InterpreterState<'_> {
        ::spacewasm::InterpreterState {
            pc: self.pc,
            jumped: self.jumped,
            fp: self.fp,
            sp: self.sp,
            stack: self.stack,
            memory: self.memory,
            table: self.table,
            module: self.module,
            result: self.result,
            store,
        }
    }
}

pub(crate) fn function(store: &::spacewasm::Store, name: &str) -> Result<::spacewasm::WasmRef, String> {
    let module = store.modules().first().ok_or("module missing")?;
    let export = module.exports.iter().find(|export| &*export.name == name).ok_or("ABI export missing")?;
    let ::spacewasm::ExportDesc::Func(index) = &export.desc else {
        return Err("ABI export kind".into());
    };
    match module.get_func_ref(*index) {
        Some(::spacewasm::Ref::Module(index)) => Ok(::spacewasm::WasmRef {
            module: ::spacewasm::ModuleRef(0),
            index,
        }),
        _ => Err("ABI export must be module function".into()),
    }
}

pub(crate) fn invoke(
    state: &mut ::spacewasm::InterpreterState<'_>,
    name: &str,
    args: &[::spacewasm::Value],
    stack_bytes: usize,
) -> Result<(), String> {
    let reference = function(state.store, name)?;
    let function = &state.store.modules()[0].functions[reference.index as usize];
    let argument_words: usize = args
        .iter()
        .map(|arg| match arg {
            ::spacewasm::Value::I32(_) | ::spacewasm::Value::F32(_) => 1,
            ::spacewasm::Value::I64(_) | ::spacewasm::Value::F64(_) => 2,
        })
        .sum();
    let required = argument_words + usize::from(function.stack_usage) + 2 + usize::from(function.local_size);
    if required > stack_bytes / 4 {
        return Err("stack bound".into());
    }
    state.reset();
    state.invoke(reference, args).map_err(|error| format!("{error:?}"))
}

pub(crate) fn decode_failure(error: ::spacewasm::ParseError) -> super::continuation::Failure {
    match error.err.err {
        ::spacewasm::ValidationError::AllocError(_) => {
            super::continuation::Failure::Allocation("interpreter-or-IR-allocation-bound".into())
        }
        ::spacewasm::ValidationError::MemoryError(
            ::spacewasm::MemoryError::OutOfMemory | ::spacewasm::MemoryError::AllocationFailed,
        ) => super::continuation::Failure::Allocation("guest-linear-memory-allocation".into()),
        ::spacewasm::ValidationError::MemoryError(_) => super::continuation::Failure::Memory,
        ::spacewasm::ValidationError::StackTooLarge | ::spacewasm::ValidationError::ControlFlowTooDeep => {
            super::continuation::Failure::Stack
        }
        _ => super::continuation::Failure::Denied("module-decode-rejected".into()),
    }
}
