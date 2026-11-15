// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original diffuse sphere path tracer. No imported scene or renderer source.
use crate::require;
use crate::Manifest;
use crate::Result;
use crate::Scene;
use crate::Tile;
#[derive(Clone, Copy)]
pub(crate) struct V(f64, f64, f64);
impl V {
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1, self.2 + b.2)
    }
    fn sub(self, b: Self) -> Self {
        Self(self.0 - b.0, self.1 - b.1, self.2 - b.2)
    }
    fn scale(self, s: f64) -> Self {
        Self(self.0 * s, self.1 * s, self.2 * s)
    }
    fn mul(self, b: Self) -> Self {
        Self(self.0 * b.0, self.1 * b.1, self.2 * b.2)
    }
    fn dot(self, b: Self) -> f64 {
        self.0 * b.0 + self.1 * b.1 + self.2 * b.2
    }
    fn cross(self, b: Self) -> Self {
        Self(self.1 * b.2 - self.2 * b.1, self.2 * b.0 - self.0 * b.2, self.0 * b.1 - self.1 * b.0)
    }
    fn unit(self) -> Self {
        self.scale(1.0 / self.dot(self).sqrt())
    }
}
fn point(p: [i32; 3]) -> V {
    V(p[0] as f64 / 1e6, p[1] as f64 / 1e6, p[2] as f64 / 1e6)
}
fn color(p: [u8; 3]) -> V {
    V(p[0] as f64 / 255.0, p[1] as f64 / 255.0, p[2] as f64 / 255.0)
}
pub(crate) fn camera(m: &Manifest) -> Result<(V, V, V, V)> {
    let eye = point(m.camera.origin);
    let forward = point(m.camera.target).sub(eye);
    require(forward.dot(forward) > 1e-16, "camera eye equals target")?;
    let forward = forward.unit();
    let right = forward.cross(point(m.camera.up));
    require(right.dot(right) > 1e-16, "camera up parallel to view")?;
    let right = right.unit();
    Ok((eye, forward, right, right.cross(forward)))
}
struct Rng(u64);
impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let n = self.0.wrapping_mul(2_685_821_657_736_338_717);
        (n >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }
    fn hemisphere(&mut self, normal: V) -> V {
        // Fixed work and fixed draw count. Uniform cube direction, folded to hemisphere.
        let v = V(2.0 * self.unit() - 1.0, 2.0 * self.unit() - 1.0, 2.0 * self.unit() - 1.0);
        let v = if v.dot(v) < 1e-20 { normal } else { v.unit() };
        if v.dot(normal) < 0.0 {
            v.scale(-1.0)
        } else {
            v
        }
    }
}
#[derive(Clone, Copy)]
struct Surface {
    center: V,
    radius_squared: f64,
    albedo: V,
}
struct TraceScene {
    spheres: [Surface; 32],
    count: usize,
    sky_bottom: V,
    sky_top: V,
}
impl TraceScene {
    fn new(scene: Scene) -> Self {
        let empty = Surface {
            center: V(0.0, 0.0, 0.0),
            radius_squared: 0.0,
            albedo: V(0.0, 0.0, 0.0),
        };
        let mut prepared = Self {
            spheres: [empty; 32],
            count: scene.spheres.len(),
            sky_bottom: color(scene.sky_bottom),
            sky_top: color(scene.sky_top),
        };
        for (slot, sphere) in prepared.spheres.iter_mut().zip(scene.spheres) {
            let radius = sphere.radius as f64 / 1e6;
            *slot = Surface {
                center: point(sphere.center),
                radius_squared: radius * radius,
                albedo: color(sphere.albedo),
            };
        }
        prepared
    }
}
fn radiance(scene: &TraceScene, mut origin: V, mut direction: V, bounces: u32, rng: &mut Rng) -> V {
    let mut throughput = V(1.0, 1.0, 1.0);
    for _ in 0..bounces {
        let mut closest = f64::INFINITY;
        let mut hit = None;
        for sphere in &scene.spheres[..scene.count] {
            let offset = origin.sub(sphere.center);
            let half_b = offset.dot(direction);
            let discriminant = half_b * half_b - (offset.dot(offset) - sphere.radius_squared);
            if discriminant < 0.0 {
                continue;
            }
            let root = discriminant.sqrt();
            let near = -half_b - root;
            let far = -half_b + root;
            let t = if near > 1e-6 { near } else { far };
            if t > 1e-6 && t < closest {
                closest = t;
                hit = Some(sphere);
            }
        }
        match hit {
            Some(sphere) => {
                let at = origin.add(direction.scale(closest));
                let normal = at.sub(sphere.center).unit();
                origin = at.add(normal.scale(1e-5));
                direction = rng.hemisphere(normal);
                throughput = throughput.mul(sphere.albedo);
            }
            None => {
                let t = (direction.1 + 1.0) * 0.5;
                let sky = scene.sky_bottom.scale(1.0 - t).add(scene.sky_top.scale(t));
                return throughput.mul(sky);
            }
        }
    }
    // Paths still occluded at the declared bounce limit contribute no sky light.
    V(0.0, 0.0, 0.0)
}
pub(crate) fn pixels(m: &Manifest, tile: Tile) -> Result<Vec<u8>> {
    let scene = TraceScene::new(m.scene()?);
    let (eye, forward, right, up) = camera(m)?;
    let half_height = m.camera.half_height as f64 / 1e6;
    let half_width = half_height * m.width as f64 / m.height as f64;
    let mut bytes = Vec::with_capacity(tile.bytes());
    for y in tile.y..tile.y + tile.height {
        for x in tile.x..tile.x + tile.width {
            let pixel = y as u64 * m.width as u64 + x as u64;
            let mut sum = V(0.0, 0.0, 0.0);
            for sample in 0..m.samples {
                // Pixel/sample stream is independent of partition and scheduling order.
                let state = m.seed
                    ^ pixel.wrapping_mul(0x9e3779b97f4a7c15)
                    ^ (sample as u64 + 1).wrapping_mul(0xd1b54a32d192ed03);
                let mut rng = Rng(if state == 0 { 0xa0761d6478bd642f } else { state });
                let sx = ((x as f64 + rng.unit()) / m.width as f64 * 2.0 - 1.0) * half_width;
                let sy = (1.0 - (y as f64 + rng.unit()) / m.height as f64 * 2.0) * half_height;
                let direction = forward.add(right.scale(sx)).add(up.scale(sy)).unit();
                sum = sum.add(radiance(&scene, eye, direction, m.bounces, &mut rng));
            }
            let average = sum.scale(1.0 / m.samples as f64);
            for v in [average.0, average.1, average.2] {
                bytes.push((v.clamp(0.0, 1.0).sqrt() * 255.0 + 0.5) as u8);
            }
        }
    }
    Ok(bytes)
}
