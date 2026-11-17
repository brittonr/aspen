// SPDX-License-Identifier: AGPL-3.0-or-later
//! File/process boundary for inspectable preparation and final verification.
use std::fs::OpenOptions;
use std::fs::{self};
use std::io::Read;
use std::io::Write;
use std::path::Path;

use molten_batch_core::ContentKind;
use molten_batch_core::Id;
use molten_orchestrator_codec as codec;
use molten_render_pilot::content_id;
use molten_render_pilot::frozen_manifest;
use molten_render_pilot::prepare;
use molten_render_pilot::verify_final;
use molten_render_pilot::Manifest;
fn create(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    OpenOptions::new().write(true).create_new(true).open(path)?.write_all(bytes)?;
    Ok(())
}
fn read_bounded(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(molten_batch_core::MAX_CONTENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > molten_batch_core::MAX_CONTENT_BYTES {
        return Err("render object exceeds 4MiB".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("prepare") if args.len() == 3 || args.len() == 4 => {
            let component = read_bounded(&args[1])?;
            let component_id = content_id(ContentKind::Component, &component)?;
            let manifest = if args.len() == 4 { Manifest::decode(&read_bounded(&args[3])?)? } else { frozen_manifest(component_id)? };
            if manifest.renderer != component_id { return Err("manifest renderer differs from supplied component".into()); }
            let prepared = prepare(manifest)?;
            let out = Path::new(&args[2]);
            fs::create_dir(out)?;
            create(&out.join("manifest.frame"), &prepared.manifest.encode()?)?;
            create(&out.join("manifest.json"), &serde_json::to_vec_pretty(&prepared.manifest)?)?;
            create(&out.join("job.json"), &serde_json::to_vec_pretty(&prepared.job)?)?;
            create(&out.join("scene.json"), &serde_json::to_vec_pretty(&prepared.manifest.scene()?)?)?;
            let mut coverage = Vec::new();
            for i in 0..prepared.manifest.tile_count() {
                let tile = prepared.manifest.tile(i)?;
                let assignment = molten_render_pilot::Assignment {
                    manifest: prepared.manifest.clone(), operation: molten_render_pilot::Operation::Tile(i),
                }.id()?;
                coverage.push(serde_json::json!({"task": format!("tile-{i:03}"), "assignment": assignment,
                    "index": i, "x": tile.x, "y": tile.y, "width": tile.width, "height": tile.height}));
            }
            create(&out.join("coverage.json"), &serde_json::to_vec_pretty(&coverage)?)?;
            let mut objects = Vec::new();
            for (id, bytes) in prepared.objects {
                let name = format!("{id}.input");
                create(&out.join(&name), &bytes)?;
                objects.push(serde_json::json!({"id": id, "file": name, "bytes": bytes.len()}));
            }
            let plan = serde_json::json!({
                "component": component_id, "manifest": prepared.manifest.id()?,
                "job": codec::identity("job-spec.v2", &prepared.job)?,
                "tiles": prepared.manifest.tile_count(), "final_task": "assemble", "objects": objects,
                "evidence": "authored inputs; execution has not been observed by preparation"
            });
            create(&out.join("plan.json"), &serde_json::to_vec_pretty(&plan)?)?;
            println!("{}", serde_json::to_string(&plan)?);
        }
        Some("verify") if args.len() == 5 => {
            let manifest = Manifest::decode(&read_bounded(&args[1])?)?;
            let bytes = read_bounded(&args[2])?;
            let expected = Id::parse(&args[3]).map_err(|_| "invalid accepted output id")?;
            if content_id(ContentKind::Data, &bytes)? != expected { return Err("accepted output content identity mismatch".into()); }
            let pixels = verify_final(&manifest, &bytes)?;
            let ppm = Path::new(&args[4]);
            let mut file = OpenOptions::new().write(true).create_new(true).open(ppm)?;
            write!(file, "P6\n{} {}\n255\n", manifest.width, manifest.height)?;
            file.write_all(pixels)?;
            println!("{}", serde_json::json!({"verified": expected, "manifest": manifest.id()?, "rgb_bytes": pixels.len(),
                "raw_rgb_digest": codec::digest(pixels), "qualification": "local deterministic verification only"}));
        }
        _ => return Err("usage: molten-render-fixture prepare COMPONENT NEW_DIR [MANIFEST_FRAME] | verify MANIFEST_FRAME OUTPUT ACCEPTED_CONTENT_ID NEW_PPM".into()),
    }
    Ok(())
}
