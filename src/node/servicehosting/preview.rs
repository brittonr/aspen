/// Pure request projection used by an independent exact-operation credential producer.
/// A preview never grants authority or starts a callback.
pub enum Input<'a> {
    Install(&'a crate::system_extension::RecoveryGroupDeclaration),
    Status,
    Supervise {
        group_id: &'a str,
        exit: &'a crate::system_extension::RecoveryExit,
    },
    Submit {
        member: &'a str,
        presentation: &'a crate::system_extension::WorkPresentation,
        payload: &'a crate::system_extension::NativeCallbackValue,
    },
    Readmit {
        member: &'a str,
        work_ref: &'a str,
    },
    Deliver(&'a super::DeliveryInput),
    Upgrade {
        member: &'a str,
        admission_ref: &'a str,
    },
    Reconcile {
        group_id: &'a str,
    },
    ReadmitMigrated {
        member: &'a str,
        input: &'a super::MigratedWorkInput,
    },
}

impl<A: super::AuthorityPort, R: super::RuntimePort> super::Host<A, R> {
    pub fn preview(&self, input: Input<'_>, logical_tick: u64) -> crate::error::Result<super::OperationRequest> {
        let (operation, subject, input_ref) = match input {
            Input::Install(declaration) => {
                ("install", declaration.group_id.as_str(), super::codec::declaration_ref(declaration)?)
            }
            Input::Status => ("status", "node-service-host", self.state_ref()?),
            Input::Supervise { group_id, exit } => ("supervise", group_id, super::groups::exit_ref(exit)?),
            Input::Submit {
                member,
                presentation,
                payload,
            } => ("submit", member, super::work::submission_ref(presentation, payload)?),
            Input::Readmit { member, work_ref } => ("readmit", member, work_ref.to_string()),
            Input::Deliver(input) => ("deliver", input.member.as_str(), super::work::ingress_ref(input)?),
            Input::Upgrade { member, admission_ref } => ("upgrade", member, admission_ref.to_string()),
            Input::Reconcile { group_id } => ("reconcile", group_id, self.state_ref()?),
            Input::ReadmitMigrated { member, input } => {
                ("readmit-migrated", member, super::work::migration::input_ref(input)?)
            }
        };
        self.request(operation, subject, &input_ref, logical_tick)
    }

    pub(crate) fn request(
        &self,
        operation: &str,
        subject: &str,
        input_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<super::OperationRequest> {
        crate::preserves_rail::ContentRef::parse(input_ref)?;
        Ok(super::OperationRequest {
            operation: operation.to_string(),
            subject: subject.to_string(),
            logical_tick,
            state_ref: self.state_ref()?,
            input_ref: input_ref.to_string(),
        })
    }

    /// Observe the actual durable native record without opening another owner.
    pub(crate) fn active_instance(
        &self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        let group = self.member_group(member)?;
        self.available_group(&group, logical_tick)?;
        let actual = self.runtime.instance(member)?;
        if self.state.members.get(member) != Some(&actual)
            || actual.lifecycle.phase != crate::system_extension::LifecyclePhase::Running
            || !actual.unresolved.is_empty()
        {
            return Err(super::invalid("scope effect runtime is stale, uncertain, or bound to another native service"));
        }
        Ok(actual)
    }
}
