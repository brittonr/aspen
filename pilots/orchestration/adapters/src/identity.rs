// SPDX-License-Identifier: AGPL-3.0-or-later
use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_codec as codec;
#[derive(Default)]
pub struct CanonicalIdentity;
impl IdentityPort for CanonicalIdentity {
    fn spec(&self, spec: &JobSpec) -> Result<Id, Error> {
        validate_job(spec)?;
        codec::identity("job-spec.v1", spec).map_err(super::invalid)
    }
    fn run(&self, cluster: Id, key: &str) -> Result<Id, Error> {
        if !valid_name(key) {
            return Err(Error::Denied);
        }
        codec::identity("run-id.v1", &(cluster, key)).map_err(super::invalid)
    }
    fn assignment(&self, cluster: Id, generation: u64, p: &Placement, spec: Id) -> Result<Id, Error> {
        codec::identity("assignment-id.v1", &(cluster, generation, p.run, &p.task, p.node, p.number, spec))
            .map_err(super::invalid)
    }
    fn request(&self, r: &ExecutionRequest) -> Result<Id, Error> {
        validate_request(r)?;
        codec::identity("execution-request.v1", r).map_err(super::invalid)
    }
    fn result(&self, r: &ExecutionResult) -> Result<Id, Error> {
        codec::identity("execution-result.v1", r).map_err(super::invalid)
    }
}
pub struct FirstFitPlacement;
impl PlacementPort for FirstFitPlacement {
    fn next(&self, s: &State) -> Result<Option<Placement>, Error> {
        next_placement(s).map_err(Into::into)
    }
}
pub fn verify_state(s: &State) -> Result<(), Error> {
    validate_state(s)?;
    let ids = CanonicalIdentity;
    for (key, sub) in &s.submissions {
        if ids.run(s.cluster, key)? != sub.run {
            return Err(Error::Invalid("run identity".into()));
        }
    }
    for r in s.runs.values() {
        if ids.spec(&r.spec)? != r.spec_ref {
            return Err(Error::Invalid("spec identity".into()));
        }
    }
    for a in s.attempts.values() {
        verify_request(&a.request, a.request_ref)?;
        if let AttemptPhase::Terminal { result_ref, result } = a.phase {
            if ids.result(&result)? != result_ref {
                return Err(Error::Invalid("result identity".into()));
            }
        }
    }
    Ok(())
}
pub fn verify_request(r: &ExecutionRequest, request_ref: Id) -> Result<(), Error> {
    let f = &r.fence;
    let ids = CanonicalIdentity;
    let p = Placement {
        run: f.run,
        task: f.task.clone(),
        node: f.node,
        number: f.epoch,
        inputs: r.inputs.clone(),
    };
    if ids.assignment(f.cluster, f.generation, &p, f.spec_ref)? != f.assignment || ids.request(r)? != request_ref {
        return Err(Error::Invalid("request/assignment identity".into()));
    }
    Ok(())
}
pub fn verify_worker(s: &WorkerState) -> Result<(), Error> {
    validate_worker(s)?;
    for r in s.records.values() {
        verify_request(&r.request, r.request_ref)?;
    }
    Ok(())
}
pub fn content(kind: ContentKind, bytes: Vec<u8>) -> Result<Content, Error> {
    if bytes.len() > MAX_CONTENT_BYTES {
        return Err(Error::Bound);
    }
    Ok(Content {
        manifest: ContentManifest {
            kind,
            raw_digest: codec::digest(&bytes),
            length: bytes.len() as u64,
        },
        bytes,
    })
}
pub fn content_id(c: &Content) -> Result<Id, Error> {
    if c.bytes.len() > MAX_CONTENT_BYTES
        || c.manifest.length != c.bytes.len() as u64
        || codec::digest(&c.bytes) != c.manifest.raw_digest
    {
        return Err(Error::Invalid("content identity/length".into()));
    }
    codec::identity("content-manifest.v1", &c.manifest).map_err(super::invalid)
}
