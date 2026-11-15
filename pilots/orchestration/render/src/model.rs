// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;

use molten_batch_core::Id;
use molten_batch_core::Resources;
use molten_batch_core::MAX_CONTENT_BYTES;
use molten_batch_core::PROFILE;
use molten_orchestrator_codec as codec;
use serde::Deserialize;
use serde::Serialize;

use crate::require;
use crate::Result;

pub const SCENE_SCHEMA: &str = "molten.render.spheres.v1";
pub const RNG: &str = "xorshift64star-pixel-sample-v1";
pub const OUTPUT: &str = "rgb8-gamma2-top-left-v1";
pub const RUNTIME: &str = "wasmtime45-cranelift-nan-canonical-f64-v1";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sphere {
    pub center: [i32; 3],
    pub radius: u32,
    pub albedo: [u8; 3],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub sky_bottom: [u8; 3],
    pub sky_top: [u8; 3],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub origin: [i32; 3],
    pub target: [i32; 3],
    pub up: [i32; 3],
    /// Half of the sensor height at unit focal distance, in millionths.
    pub half_height: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeProfile {
    pub batch_profile: String,
    pub engine_profile: String,
    pub resources: Resources,
    pub required_labels: BTreeMap<String, String>,
    pub fuel: u64,
    pub timeout_ms: u64,
    pub max_attempts: u32,
    pub retry_delay_ticks: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub scene_schema: String,
    pub scene_bytes: Vec<u8>,
    pub camera: Camera,
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub rng: String,
    pub seed: u64,
    pub bounces: u32,
    pub columns: u32,
    pub rows: u32,
    pub renderer: Id,
    pub runtime: RuntimeProfile,
    pub output: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub index: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
impl Tile {
    pub fn bytes(self) -> usize {
        self.width as usize * self.height as usize * 3
    }
}
impl Manifest {
    pub fn id(&self) -> Result<Id> {
        Ok(codec::identity("render-manifest.v1", self)?)
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        Ok(codec::frame("render-manifest.v1", self)?)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let m: Self = codec::unframe("render-manifest.v1", bytes)?;
        m.validate()?;
        Ok(m)
    }
    pub fn scene(&self) -> Result<Scene> {
        Ok(codec::unframe(SCENE_SCHEMA, &self.scene_bytes)?)
    }
    pub fn tile_count(&self) -> u32 {
        self.columns * self.rows
    }
    pub fn image_bytes(&self) -> usize {
        self.width as usize * self.height as usize * 3
    }
    pub fn tile(&self, index: u32) -> Result<Tile> {
        require(self.columns > 0 && self.rows > 0 && index < self.tile_count(), "tile index")?;
        let column = index % self.columns;
        let row = index / self.columns;
        let x = column * self.width / self.columns;
        let y = row * self.height / self.rows;
        Ok(Tile {
            index,
            x,
            y,
            width: (column + 1) * self.width / self.columns - x,
            height: (row + 1) * self.height / self.rows - y,
        })
    }
    // r[impl molten.batch_render.identity]
    pub fn validate(&self) -> Result<()> {
        require(
            self.version == 1 && self.scene_schema == SCENE_SCHEMA && self.rng == RNG && self.output == OUTPUT,
            "unsupported render schema",
        )?;
        require(self.width > 0 && self.height > 0 && self.width <= 1024 && self.height <= 1024, "dimensions")?;
        require(
            self.columns > 0
                && self.rows > 0
                && self.columns <= self.width
                && self.rows <= self.height
                && self.columns <= 63
                && self.rows <= 63
                && self.tile_count() <= 63,
            "partition",
        )?;
        require(
            self.samples > 0 && self.samples <= 64 && self.bounces > 0 && self.bounces <= 16 && self.seed > 0,
            "sampling",
        )?;
        require(
            self.renderer != Id::default()
                && self.runtime.batch_profile == PROFILE
                && self.runtime.engine_profile == RUNTIME,
            "runtime profile",
        )?;
        molten_batch_core::validate_resources(self.runtime.resources).map_err(|e| crate::Error(e.to_string()))?;
        molten_batch_core::validate_labels(&self.runtime.required_labels).map_err(|e| crate::Error(e.to_string()))?;
        require(
            self.runtime.resources.memory_bytes >= 16 * 1024 * 1024
                && self.runtime.fuel > 0
                && self.runtime.fuel <= 1_000_000_000
                && self.runtime.timeout_ms > 0
                && self.runtime.timeout_ms <= 60_000
                && self.runtime.max_attempts > 0
                && self.runtime.max_attempts <= 8,
            "runtime limits",
        )?;
        require(self.scene_bytes.len() <= 32 * 1024, "scene bytes")?;
        let scene = self.scene()?;
        require(!scene.spheres.is_empty() && scene.spheres.len() <= 32, "sphere count")?;
        let bounded = |p: &[i32; 3]| p.iter().all(|v| v.unsigned_abs() <= 1_000_000_000);
        require(
            scene.spheres.iter().all(|s| bounded(&s.center) && s.radius > 0 && s.radius <= 1_000_000_000),
            "sphere bounds",
        )?;
        require(
            bounded(&self.camera.origin)
                && bounded(&self.camera.target)
                && bounded(&self.camera.up)
                && self.camera.half_height > 0
                && self.camera.half_height <= 4_000_000,
            "camera bounds",
        )?;
        crate::ray::camera(self)?;
        // Every input, including all tile envelopes plus literal assembly assignment, fits one existing
        // task.
        let assignment = crate::Assignment {
            manifest: self.clone(),
            operation: crate::Operation::Assemble,
        };
        let literal_len = codec::frame("render-assignment.v1", &assignment)?.len();
        require(
            self.image_bytes() + self.tile_count() as usize * crate::TILE_HEADER + literal_len <= MAX_CONTENT_BYTES
                && self.image_bytes() + crate::FINAL_HEADER <= MAX_CONTENT_BYTES,
            "aggregate object/input bound",
        )?;
        Ok(())
    }
}
/// Frozen scene in integer millionths: a ground sphere and three diffuse spheres.
pub fn frozen_manifest(renderer: Id) -> Result<Manifest> {
    let scene = Scene {
        spheres: vec![
            Sphere {
                center: [0, -100_500_000, -1_000_000],
                radius: 100_000_000,
                albedo: [180, 180, 160],
            },
            Sphere {
                center: [0, 0, -1_000_000],
                radius: 500_000,
                albedo: [190, 65, 45],
            },
            Sphere {
                center: [-1_000_000, 0, -1_200_000],
                radius: 500_000,
                albedo: [45, 95, 200],
            },
            Sphere {
                center: [1_000_000, 0, -1_000_000],
                radius: 500_000,
                albedo: [65, 185, 90],
            },
        ],
        sky_bottom: [255, 255, 255],
        sky_top: [125, 175, 255],
    };
    let m = Manifest {
        version: 1,
        scene_schema: SCENE_SCHEMA.into(),
        scene_bytes: codec::frame(SCENE_SCHEMA, &scene)?,
        camera: Camera {
            origin: [3_000_000, 1_500_000, 2_000_000],
            target: [0, 0, -1_000_000],
            up: [0, 1_000_000, 0],
            half_height: 400_000,
        },
        width: 640,
        height: 360,
        samples: 2,
        rng: RNG.into(),
        seed: 0x4d4f4c54454e,
        bounces: 3,
        columns: 8,
        rows: 4,
        renderer,
        output: OUTPUT.into(),
        runtime: RuntimeProfile {
            batch_profile: PROFILE.into(),
            engine_profile: RUNTIME.into(),
            resources: Resources {
                cpu_millis: 1000,
                memory_bytes: 64 * 1024 * 1024,
                slots: 1,
            },
            required_labels: BTreeMap::new(),
            fuel: 1_000_000_000,
            timeout_ms: 60_000,
            max_attempts: 3,
            retry_delay_ticks: 1,
        },
    };
    m.validate()?;
    Ok(m)
}
