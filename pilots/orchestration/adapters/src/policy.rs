// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeSet;

use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_codec as codec;

use crate::CapabilityRoot;

pub struct FileAdmission {
    root: CapabilityRoot,
    expected: Id,
}
impl FileAdmission {
    pub fn new(root: CapabilityRoot, expected: Id) -> Result<Self, Error> {
        let this = Self { root, expected };
        this.read()?;
        Ok(this)
    }
    fn read(&self) -> Result<(Policy, BTreeSet<Id>), Error> {
        let policy: Policy = self.root.read_json("policy.json")?;
        validate_policy(&policy)?;
        if policy_id(&policy)? != self.expected {
            return Err(Error::Denied);
        }
        let revoked: BTreeSet<Id> = self.root.read_json("revoked.json")?;
        if revoked.len() > 4096 {
            return Err(Error::Bound);
        }
        Ok((policy, revoked))
    }
}
pub fn policy_id(p: &Policy) -> Result<Id, Error> {
    validate_policy(p)?;
    codec::identity("operator-policy.v1", p).map_err(crate::invalid)
}
impl AdmissionPort for FileAdmission {
    fn submit(&self, spec: &JobSpec, policy: Id) -> Result<(), Error> {
        validate_job(spec)?;
        if policy != self.expected {
            return Err(Error::Denied);
        }
        let (p, revoked) = self.read()?;
        for task in &spec.tasks {
            if !policy_allows_task(&p, &revoked, task)? {
                return Err(Error::Denied);
            }
        }
        Ok(())
    }
    fn assignment(&self, r: &ExecutionRequest, action: AdmissionAction) -> Result<(), Error> {
        if r.policy_ref != self.expected {
            return Err(Error::Denied);
        }
        let (p, revoked) = self.read()?;
        if !policy_allows_request(
            &p,
            &revoked,
            r,
            matches!(action, AdmissionAction::Reserve | AdmissionAction::Execute),
        )? {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
