// SPDX-License-Identifier: AGPL-3.0-or-later
use molten_batch_core::ContentKind;
use molten_batch_core::Id;
use molten_batch_core::MAX_CONTENT_BYTES;
use molten_orchestrator_codec as codec;
use molten_render_pilot::*;
fn small() -> Manifest {
    let mut m = frozen_manifest(Id([7; 32])).unwrap();
    m.width = 17;
    m.height = 11;
    m.columns = 3;
    m.rows = 2;
    m
}
fn tiles(m: &Manifest) -> Vec<Vec<u8>> {
    (0..m.tile_count()).map(|i| render_tile(m, i).unwrap()).collect()
}
// r[verify molten.batch_render.identity]
#[test]
fn complete_manifest_and_assignment_identity_changes() {
    let original = small();
    let id = original.id().unwrap();
    let tile = render_tile(&original, 0).unwrap();
    let mut variants = Vec::new();
    macro_rules! changed {
        ($field:ident, $value:expr) => {{
            let mut m = original.clone();
            m.$field = $value;
            variants.push(m);
        }};
    }
    changed!(seed, original.seed + 1);
    changed!(samples, 3);
    changed!(bounces, 2);
    changed!(renderer, Id([8; 32]));
    changed!(width, 18);
    let mut m = original.clone();
    m.camera.origin[0] += 1;
    variants.push(m);
    let mut m = original.clone();
    m.columns = 2;
    m.rows = 3;
    variants.push(m);
    let mut m = original.clone();
    m.runtime.fuel -= 1;
    variants.push(m);
    let mut m = original.clone();
    m.runtime.timeout_ms -= 1;
    variants.push(m);
    let mut m = original.clone();
    m.runtime.required_labels.insert("cohort".into(), "qualified-local".into());
    variants.push(m);
    let mut m = original.clone();
    let mut scene = m.scene().unwrap();
    scene.spheres[0].albedo[0] -= 1;
    m.scene_bytes = codec::frame(SCENE_SCHEMA, &scene).unwrap();
    variants.push(m);
    for m in variants {
        m.validate().unwrap();
        assert_ne!(id, m.id().unwrap());
        let mut all = tiles(&m);
        all[0] = tile.clone();
        assert!(assemble(&m, &all).is_err());
    }
    assert_ne!(
        Assignment {
            manifest: original.clone(),
            operation: Operation::Tile(0)
        }
        .id()
        .unwrap(),
        Assignment {
            manifest: original,
            operation: Operation::Tile(1)
        }
        .id()
        .unwrap()
    );
}
// r[verify molten.batch_render.assembly]
#[test]
fn checked_assembly_rejects_missing_overlap_stale_lengths_and_substitution() {
    let m = small();
    let good = tiles(&m);
    assert!(assemble(&m, &good[..good.len() - 1]).is_err());
    let mut duplicate = good.clone();
    duplicate[1] = duplicate[0].clone();
    assert!(assemble(&m, &duplicate).is_err());
    let mut swapped = good.clone();
    swapped.swap(0, 1);
    assert!(assemble(&m, &swapped).is_err());
    for offset in [0, 8, 40, 72, 76, 80, 84, 88, 92, TILE_HEADER] {
        let mut corrupt = good.clone();
        corrupt[0][offset] ^= 1;
        assert!(assemble(&m, &corrupt).is_err(), "offset {offset}");
    }
    let mut truncated = good.clone();
    truncated[0].pop();
    assert!(assemble(&m, &truncated).is_err());
    let mut trailing = good.clone();
    trailing[0].push(0);
    assert!(assemble(&m, &trailing).is_err());
    let mut forged = good.clone();
    forged[0][TILE_HEADER] ^= 1;
    let digest = codec::digest(&forged[0][TILE_HEADER..]);
    forged[0][92..124].copy_from_slice(&digest.0);
    assert!(assemble(&m, &forged).is_err(), "rehashing forged pixels is not acceptance");
    let final_bytes = assemble(&m, &good).unwrap();
    assert_eq!(verify_final(&m, &final_bytes).unwrap().len(), m.image_bytes());
    let mut forged_final = final_bytes.clone();
    forged_final[FINAL_HEADER] ^= 1;
    let digest = codec::digest(&forged_final[FINAL_HEADER..]);
    forged_final[48..80].copy_from_slice(&digest.0);
    assert!(decode_final(&m, &forged_final).is_ok());
    assert!(verify_final(&m, &forged_final).is_err());
    let mut stale = m.clone();
    stale.seed += 1;
    assert!(verify_final(&stale, &final_bytes).is_err());
    assert!(verify_final(&m, &final_bytes[..79]).is_err());
}
#[test]
fn rng_is_repeatable_and_partition_independent_on_this_native_profile() {
    let m = small();
    assert_eq!(render_tile(&m, 0).unwrap(), render_tile(&m, 0).unwrap());
    let image_a = assemble(&m, &tiles(&m)).unwrap();
    let mut other = m.clone();
    other.columns = 1;
    other.rows = 1;
    let image_b = assemble(&other, &tiles(&other)).unwrap();
    assert_eq!(decode_final(&m, &image_a).unwrap(), decode_final(&other, &image_b).unwrap());
    let pixels = decode_final(&m, &image_a).unwrap();
    assert!(pixels.windows(3).any(|p| p != &pixels[..3]), "frozen scene must not be a constant placeholder");
}
#[test]
fn analytically_constant_sky_fixture_has_exact_rgb8_pixels() {
    let mut m = small();
    let scene = Scene {
        spheres: vec![Sphere {
            center: [1_000_000_000; 3],
            radius: 1,
            albedo: [255; 3],
        }],
        sky_bottom: [64; 3],
        sky_top: [64; 3],
    };
    m.scene_bytes = codec::frame(SCENE_SCHEMA, &scene).unwrap();
    let output = assemble(&m, &tiles(&m)).unwrap();
    assert!(verify_final(&m, &output).unwrap().iter().all(|p| *p == 128));
}
#[test]
fn guest_abi_checks_literal_count_and_canonical_assignment() {
    let m = small();
    let assignment = Assignment {
        manifest: m.clone(),
        operation: Operation::Tile(0),
    }
    .encode()
    .unwrap();
    assert_eq!(execute(std::slice::from_ref(&assignment)).unwrap(), render_tile(&m, 0).unwrap());
    assert!(execute(&[]).is_err());
    assert!(execute(&[assignment.clone(), Vec::new()]).is_err());
    let mut noncanonical = assignment;
    noncanonical.push(b' ');
    assert!(execute(&[noncanonical]).is_err());
    let mut inputs = vec![Assignment {
        manifest: m.clone(),
        operation: Operation::Assemble,
    }
    .encode()
    .unwrap()];
    inputs.extend(tiles(&m));
    assert_eq!(execute(&inputs).unwrap(), assemble(&m, &inputs[1..]).unwrap());
}
#[test]
fn default_and_max_tile_plans_fit_existing_executor_without_profile_widening() {
    let fixture = frozen_manifest(Id([7; 32])).unwrap();
    assert_eq!(fixture.image_bytes(), 691_200);
    let prepared = prepare(fixture.clone()).unwrap();
    assert_eq!(prepared.job.tasks.len(), 33);
    assert!(prepared.job.tasks.iter().all(|t| t.workload
        == molten_batch_core::Workload::Component {
            artifact: fixture.renderer
        }));
    let total_inputs: usize = prepared.objects.iter().map(|(_, b)| b.len()).sum();
    let aggregate =
        fixture.image_bytes() + fixture.tile_count() as usize * TILE_HEADER + prepared.objects.last().unwrap().1.len();
    assert!(aggregate < MAX_CONTENT_BYTES);
    assert!(total_inputs + 8 * (fixture.image_bytes() * 2 + 63 * TILE_HEADER) + MAX_CONTENT_BYTES < 1024 * 1024 * 1024);
    let mut maximum = small();
    maximum.width = 67;
    maximum.height = 19;
    maximum.columns = 9;
    maximum.rows = 7;
    let maximum = prepare(maximum).unwrap();
    assert_eq!(maximum.job.tasks.len(), 64);
    let assembly = &maximum.job.tasks[0];
    assert_eq!(assembly.id, "assemble");
    assert_eq!(assembly.inputs.len() + assembly.dependencies.len(), 64);
    assert_eq!(assembly.dependencies.first().unwrap(), "tile-000");
    assert_eq!(assembly.dependencies.last().unwrap(), "tile-062");
    for (id, bytes) in maximum.objects {
        assert_eq!(id, content_id(ContentKind::Data, &bytes).unwrap());
    }
}
#[test]
fn malformed_manifest_fails_before_rendering() {
    let original = small();
    let mut bad = Vec::new();
    let mut m = original.clone();
    m.columns = 64;
    m.rows = 1;
    bad.push(m);
    let mut m = original.clone();
    m.columns = 0;
    bad.push(m);
    let mut m = original.clone();
    m.width = 0;
    bad.push(m);
    let mut m = original.clone();
    m.width = 1025;
    bad.push(m);
    let mut m = original.clone();
    m.seed = 0;
    bad.push(m);
    let mut m = original.clone();
    m.samples = 0;
    bad.push(m);
    let mut m = original.clone();
    m.bounces = 17;
    bad.push(m);
    let mut m = original.clone();
    m.scene_schema = "other".into();
    bad.push(m);
    let mut m = original.clone();
    m.rng = "ambient".into();
    bad.push(m);
    let mut m = original.clone();
    m.output = "jpeg".into();
    bad.push(m);
    let mut m = original.clone();
    m.runtime.engine_profile = "native-jit".into();
    bad.push(m);
    let mut m = original.clone();
    m.camera.target = m.camera.origin;
    bad.push(m);
    let mut m = original.clone();
    m.camera.up = [0; 3];
    bad.push(m);
    let mut m = original;
    m.scene_bytes.push(b' ');
    bad.push(m);
    for m in bad {
        assert!(m.validate().is_err());
        assert!(prepare(m).is_err());
    }
}

#[test]
fn frozen_render_job_needs_no_default_policy_widening() {
    let manifest = frozen_manifest(Id([7; 32])).unwrap();
    let prepared = prepare(manifest).unwrap();
    molten_batch_core::validate_job(&prepared.job).unwrap();
    for task in prepared.job.tasks {
        assert!(task.fuel <= 1_000_000_000);
        assert!(task.timeout_ms <= 60_000);
        assert!(task.resources.memory_bytes <= 268_435_456);
        assert!(task.max_output_bytes <= MAX_CONTENT_BYTES as u64);
    }
}
