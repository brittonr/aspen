// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original deterministic render workload; no scheduler, effects or progress state.
#![forbid(unsafe_code)]
mod model;
mod ray;
mod wire;

pub use model::*;
use molten_batch_core::ContentKind;
use molten_batch_core::ContentManifest;
use molten_batch_core::Id;
use molten_batch_core::JobSpec;
use molten_batch_core::TaskSpec;
use molten_batch_core::MAX_CONTENT_BYTES;
use molten_orchestrator_codec as codec;
use serde::Deserialize;
use serde::Serialize;
pub use wire::assemble;
pub use wire::decode_final;
pub use wire::render_tile;
pub use wire::verify_final;
pub use wire::FINAL_HEADER;
pub use wire::TILE_HEADER;

pub type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<codec::Error> for Error {
    fn from(e: codec::Error) -> Self {
        Self(e.to_string())
    }
}
pub(crate) fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error(message.into()))
    }
}
pub fn content_id(kind: ContentKind, bytes: &[u8]) -> Result<Id> {
    require(!bytes.is_empty() && bytes.len() <= MAX_CONTENT_BYTES, "content bound")?;
    Ok(codec::identity("content-manifest.v1", &ContentManifest {
        kind,
        raw_digest: codec::digest(bytes),
        length: bytes.len() as u64,
    })?)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Operation {
    Tile(u32),
    Assemble,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub manifest: Manifest,
    pub operation: Operation,
}
impl Assignment {
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.manifest.validate()?;
        if let Operation::Tile(i) = self.operation {
            self.manifest.tile(i)?;
        }
        Ok(codec::frame("render-assignment.v1", self)?)
    }
    pub fn id(&self) -> Result<Id> {
        Ok(codec::identity("render-assignment.v1", self)?)
    }
}
/// Import-free ABI implementation. Guest traps on Err, never emits a success-shaped error.
pub fn execute(inputs: &[Vec<u8>]) -> Result<Vec<u8>> {
    require(!inputs.is_empty(), "missing assignment")?;
    require(
        inputs
            .iter()
            .try_fold(0usize, |n, b| n.checked_add(b.len()))
            .is_some_and(|n| n <= MAX_CONTENT_BYTES),
        "input bound",
    )?;
    let assignment: Assignment = codec::unframe("render-assignment.v1", &inputs[0])?;
    assignment.manifest.validate()?;
    match assignment.operation {
        Operation::Tile(i) => {
            require(inputs.len() == 1, "tile input count")?;
            render_tile(&assignment.manifest, i)
        }
        Operation::Assemble => assemble(&assignment.manifest, &inputs[1..]),
    }
}
/// Literal objects are returned in task order; caller publishes them before submission.
pub struct Prepared {
    pub manifest: Manifest,
    pub objects: Vec<(Id, Vec<u8>)>,
    pub job: JobSpec,
}
// r[impl molten.batch_render.identity]
pub fn prepare(manifest: Manifest) -> Result<Prepared> {
    manifest.validate()?;
    let count = manifest.tile_count();
    let mut objects = Vec::with_capacity(count as usize + 1);
    let mut tasks = Vec::with_capacity(count as usize + 1);
    let names: Vec<String> = (0..count).map(|i| format!("tile-{i:03}")).collect();
    for operation in (0..count).map(Operation::Tile).chain(std::iter::once(Operation::Assemble)) {
        let (id, dependencies, max_output_bytes) = match operation {
            Operation::Tile(i) => {
                (names[i as usize].clone(), Vec::new(), manifest.tile(i)?.bytes() as u64 + TILE_HEADER as u64)
            }
            Operation::Assemble => {
                ("assemble".into(), names.clone(), manifest.image_bytes() as u64 + FINAL_HEADER as u64)
            }
        };
        let bytes = Assignment {
            manifest: manifest.clone(),
            operation,
        }
        .encode()?;
        let input = content_id(ContentKind::Data, &bytes)?;
        objects.push((input, bytes));
        tasks.push(TaskSpec {
            id,
            component: manifest.renderer,
            inputs: vec![input],
            dependencies,
            resources: manifest.runtime.resources,
            required_labels: manifest.runtime.required_labels.clone(),
            fuel: manifest.runtime.fuel,
            timeout_ms: manifest.runtime.timeout_ms,
            max_output_bytes,
            max_attempts: manifest.runtime.max_attempts,
            retry_delay_ticks: manifest.runtime.retry_delay_ticks,
        });
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    let job = JobSpec {
        name: "original-render-v1".into(),
        tasks,
    };
    molten_batch_core::validate_job(&job).map_err(|e| Error(e.to_string()))?;
    Ok(Prepared { manifest, objects, job })
}
