//! Fixed no-WASI effect linker; product admission precedes every invocation.

impl super::EngineAdapter {
    pub(crate) fn execute_hosted<H: super::super::operated::EffectRequestHost + 'static>(
        &self,
        request: &super::super::shell::ExecutionRequest<'_>,
        deadline_ticks: u64,
        host: H,
    ) -> super::super::shell::ExecutionOutcome {
        self.hosted_inner(request, deadline_ticks, host)
            .unwrap_or_else(|denial| super::denied_outcome(request, denial))
    }

    fn hosted_inner<H: super::super::operated::EffectRequestHost + 'static>(
        &self,
        request: &super::super::shell::ExecutionRequest<'_>,
        deadline_ticks: u64,
        host: H,
    ) -> super::super::super::model::Result<super::super::shell::ExecutionOutcome> {
        let admitted = super::admit(request)?;
        if request.profile.wit.package != super::super::super::effect::HOST_WIT_PACKAGE
            || request.profile.wit.world != super::super::super::effect::HOST_WIT_WORLD
            || request.facts.imports != [super::super::super::effect::HOST_REQUEST_IMPORT]
        {
            return Err(super::super::super::model::Denial::new(
                "the hosted runtime requires the exact admitted effect world and import",
            ));
        }
        let grant = request
            .import_grants
            .iter()
            .find(|grant| grant.import == super::super::super::effect::HOST_REQUEST_IMPORT)
            .ok_or_else(|| super::super::super::model::Denial::new("the effect import has no current grant"))?;
        let descriptor = hardened_wasmtime_runtime::restricted::imports::Descriptor {
            function: hardened_wasmtime_runtime::artifact::shape::InterfaceFunction {
                qualified_name: hardened_wasmtime_shell::qualified_name(&[
                    super::super::super::effect::HOST_REQUEST_IMPORT,
                ]),
                schema_identity: request.profile.wit.source_ref.clone(),
            },
            world: hardened_wasmtime_runtime::artifact::shape::WorldIdentity {
                package: request.profile.wit.package.clone(),
                world: request.profile.wit.world.clone(),
            },
            capability_id: grant.recorded_effect_ref.clone(),
            adapter_id: super::super::super::effect::HOST_PROFILE_ID.to_string(),
        };
        let mut admitted_grant = Some(grant.clone());
        let inspection =
            super::stage(request, &admitted, super::super::super::evidence::receipt::Stage::Inspection, Vec::new())?;
        let invocation = match self.prepare_bound(request, deadline_ticks, std::slice::from_ref(&descriptor)) {
            Ok(invocation) => invocation,
            Err(denial) => return super::denied(request, &admitted, denial, vec![inspection]),
        };
        let observation = invocation.run(
            super::super::super::admission::COMPONENT_INVOKE_EXPORT,
            &admitted.input_bytes,
            host,
            move |requested, linker| {
                if requested != &descriptor {
                    return Err(hardened_wasmtime_shell::HostDenied);
                }
                let admitted_grant = admitted_grant.take().ok_or(hardened_wasmtime_shell::HostDenied)?;
                linker
                    .root()
                    .func_wrap(
                        super::super::super::effect::HOST_REQUEST_IMPORT,
                        move |mut store: wasmtime::StoreContextMut<'_, hardened_wasmtime_shell::State<H>>,
                              (bytes,): (Vec<u8>,)| {
                            let response = invoke_host(&mut store.data_mut().host, &bytes, &admitted_grant)
                                .map_err(|_| wasmtime::Error::new(hardened_wasmtime_shell::HostDenied))?;
                            Ok((Ok::<Vec<u8>, String>(response),))
                        },
                    )
                    .map_err(|_| hardened_wasmtime_shell::HostDenied)
            },
        );
        super::finish(request, &admitted, vec![inspection], observation)
    }
}

fn invoke_host<H: super::super::operated::EffectRequestHost>(
    host: &mut H,
    bytes: &[u8],
    admitted_grant: &super::super::super::admission::ImportGrant,
) -> super::super::super::model::Result<Vec<u8>> {
    let decoded = super::super::super::effect::decode_host_request(bytes)?;
    if decoded.grant() != admitted_grant {
        return Err(super::super::super::model::Denial::new(
            "guest effect grant differs from the exact admitted runtime grant",
        ));
    }
    host.invoke(bytes)
}

#[cfg(test)]
mod tests;
