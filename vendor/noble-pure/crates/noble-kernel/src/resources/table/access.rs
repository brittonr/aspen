impl super::Table {
    /// Atomically remove guest availability and account one native pin.
    /// Execute native work only after this returns successfully.
    // r[impl RA-TYPE-01]
    // r[impl RA-STATE-01]
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; begin commits guest unavailability and exactly one pin only after scope and handle preflight; the table is exclusive retained transition state."
    )]
    pub fn begin(
        &mut self,
        owner: crate::resources::Owner,
        required: crate::resources::Requirement,
    ) -> Result<crate::resources::Admitted, crate::resources::Rejected<crate::resources::Owner>>
    {
        let serial = match super::next(
            self.scope,
            self.limits.scopes,
            crate::resources::Error::ScopeExhausted,
        ) {
            Ok(serial) => serial,
            Err(error) => {
                return Err(crate::resources::Rejected {
                    error,
                    input: owner,
                })
            }
        };
        let decision = match self.decide(
            owner.handle,
            crate::resources::Event::Begin { required, serial },
        ) {
            Ok(decision) => decision,
            Err(error) => {
                return Err(crate::resources::Rejected {
                    error,
                    input: owner,
                })
            }
        };
        match self.commit(decision) {
            Ok(()) => {}
            Err(error) => {
                return Err(crate::resources::Rejected {
                    error,
                    input: owner,
                })
            }
        }
        self.scope = serial;
        let scope = crate::resources::Scope {
            handle: owner.handle,
            serial,
        };
        Ok(crate::resources::Admitted {
            borrow: crate::resources::Borrow { scope },
            decision,
        })
    }

    /// Revalidate an adapter-local token before new native access. Cancellation
    /// forbids new access, but does not invalidate an already-running native pin.
    pub fn native_access(
        &self,
        borrow: &crate::resources::Borrow,
    ) -> Result<crate::resources::Snapshot, crate::resources::Error> {
        match self.decide(
            borrow.scope.handle,
            crate::resources::Event::Access {
                serial: borrow.scope.serial,
            },
        ) {
            Ok(decision) => Ok(decision.record),
            Err(error) => Err(error),
        }
    }

    /// Trusted shell notification that the matching native scope can no longer
    /// access storage. Untrusted guests must not directly invoke this hook.
    /// A duplicate is a zero-accounting decision, never another returned owner.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; completion commits only a matching retained scope and returns an owner solely for the OwnerReturned action; mutation is confined to the owned table."
    )]
    pub fn complete(
        &mut self,
        scope: crate::resources::Scope,
        result: crate::resources::Completion,
    ) -> Result<crate::resources::Completed, crate::resources::Error> {
        let decision = attempt!(self.decide(
            scope.handle,
            crate::resources::Event::Complete {
                serial: scope.serial,
                result,
            },
        ));
        match self.commit(decision) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        let owner = match decision.action {
            crate::resources::Action::OwnerReturned(_) => Some(crate::resources::Owner {
                handle: decision.record.handle,
            }),
            crate::resources::Action::Unchanged
            | crate::resources::Action::BorrowAdmitted
            | crate::resources::Action::AccessRevoked
            | crate::resources::Action::LocalRelease
            | crate::resources::Action::RetirementCompleted
            | crate::resources::Action::OwnerTransferred => None,
        };
        Ok(crate::resources::Completed { owner, decision })
    }

    /// Revoke only the named scope; an old cancellation cannot retire a later
    /// call. Existing pins remain charged until matching native completion.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; revoke applies the checked scope-specific retirement transition to the exclusively owned table without releasing native pins."
    )]
    pub fn revoke(
        &mut self,
        scope: crate::resources::Scope,
        reason: crate::resources::Retirement,
    ) -> Result<crate::resources::Decision, crate::resources::Error> {
        self.apply(
            scope.handle,
            crate::resources::Event::Revoke {
                serial: scope.serial,
                reason,
            },
        )
    }

    /// Host cleanup independent of any guest continuation. Unlike guest release,
    /// this may retire a busy owner, preserving its pin and primary failure.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; retire applies host cleanup to the exclusively owned table while retaining busy pins and the primary retirement reason."
    )]
    pub fn retire(
        &mut self,
        claim: crate::resources::Handle,
        reason: crate::resources::Retirement,
    ) -> Result<crate::resources::Decision, crate::resources::Error> {
        self.apply(claim, crate::resources::Event::Retire(reason))
    }

    /// Consume a live guest owner and decide one local release. Busy guest
    /// release is rejected; the unchanged owner is returned on every failure.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; release commits the checked live-owner release transition and returns the original obligation on every rejection."
    )]
    pub fn release(
        &mut self,
        owner: crate::resources::Owner,
        required: crate::resources::Requirement,
    ) -> Result<crate::resources::Decision, crate::resources::Rejected<crate::resources::Owner>>
    {
        match self.apply(owner.handle, crate::resources::Event::Release(required)) {
            Ok(decision) => Ok(decision),
            Err(error) => Err(crate::resources::Rejected {
                error,
                input: owner,
            }),
        }
    }
}
