// SPDX-License-Identifier: AGPL-3.0-or-later
use molten_orchestrator_codec as codec;

use crate::require;
use crate::Assignment;
use crate::Manifest;
use crate::Operation;
use crate::Result;
pub const TILE_HEADER: usize = 124;
pub const FINAL_HEADER: usize = 80;
const TILE_MAGIC: &[u8; 8] = b"MRNTIL01";
const FINAL_MAGIC: &[u8; 8] = b"MRNRGB01";
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}
pub fn render_tile(manifest: &Manifest, index: u32) -> Result<Vec<u8>> {
    manifest.validate()?;
    let tile = manifest.tile(index)?;
    let pixels = crate::ray::pixels(manifest, tile)?;
    let mut output = Vec::with_capacity(TILE_HEADER + pixels.len());
    output.extend_from_slice(TILE_MAGIC);
    output.extend_from_slice(&manifest.id()?.0);
    output.extend_from_slice(
        &Assignment {
            manifest: manifest.clone(),
            operation: Operation::Tile(index),
        }
        .id()?
        .0,
    );
    for n in [tile.index, tile.x, tile.y, tile.width, tile.height] {
        output.extend_from_slice(&n.to_le_bytes());
    }
    output.extend_from_slice(&codec::digest(&pixels).0);
    output.extend_from_slice(&pixels);
    Ok(output)
}
// r[impl molten.batch_render.assembly]
pub fn assemble(manifest: &Manifest, tiles: &[Vec<u8>]) -> Result<Vec<u8>> {
    manifest.validate()?;
    require(tiles.len() == manifest.tile_count() as usize, "missing or excess tile")?;
    let manifest_id = manifest.id()?;
    let mut image = vec![0; manifest.image_bytes()];
    for (index, bytes) in tiles.iter().enumerate() {
        let tile = manifest.tile(index as u32)?;
        require(bytes.len() == TILE_HEADER + tile.bytes(), "malformed tile length")?;
        require(&bytes[..8] == TILE_MAGIC && bytes[8..40] == manifest_id.0, "stale tile manifest")?;
        let assignment = Assignment {
            manifest: manifest.clone(),
            operation: Operation::Tile(index as u32),
        }
        .id()?;
        require(bytes[40..72] == assignment.0, "substituted tile assignment")?;
        let fields = [tile.index, tile.x, tile.y, tile.width, tile.height];
        require(
            fields.iter().enumerate().all(|(i, n)| u32_at(bytes, 72 + i * 4) == *n),
            "overlap or wrong tile coverage",
        )?;
        let pixels = &bytes[TILE_HEADER..];
        require(bytes[92..124] == codec::digest(pixels).0, "tile digest")?;
        // A digest alone cannot distinguish a self-consistent substituted image.
        // Compare deterministic expected pixels before emitting any final artifact.
        require(crate::ray::pixels(manifest, tile)? == pixels, "substituted tile pixels")?;
        let row_bytes = tile.width as usize * 3;
        for row in 0..tile.height as usize {
            let offset = ((tile.y as usize + row) * manifest.width as usize + tile.x as usize) * 3;
            image[offset..offset + row_bytes].copy_from_slice(&pixels[row * row_bytes..(row + 1) * row_bytes]);
        }
    }
    let mut output = Vec::with_capacity(FINAL_HEADER + image.len());
    output.extend_from_slice(FINAL_MAGIC);
    output.extend_from_slice(&manifest_id.0);
    output.extend_from_slice(&manifest.width.to_le_bytes());
    output.extend_from_slice(&manifest.height.to_le_bytes());
    output.extend_from_slice(&codec::digest(&image).0);
    output.extend_from_slice(&image);
    Ok(output)
}
/// Check structural/identity binding; use verify_final for deterministic pixel acceptance.
pub fn decode_final<'a>(manifest: &Manifest, bytes: &'a [u8]) -> Result<&'a [u8]> {
    manifest.validate()?;
    require(bytes.len() == FINAL_HEADER + manifest.image_bytes(), "final length")?;
    require(&bytes[..8] == FINAL_MAGIC && bytes[8..40] == manifest.id()?.0, "final manifest")?;
    require(u32_at(bytes, 40) == manifest.width && u32_at(bytes, 44) == manifest.height, "final dimensions")?;
    let pixels = &bytes[FINAL_HEADER..];
    require(bytes[48..80] == codec::digest(pixels).0, "final digest")?;
    Ok(pixels)
}
pub fn verify_final<'a>(manifest: &Manifest, bytes: &'a [u8]) -> Result<&'a [u8]> {
    let image = decode_final(manifest, bytes)?;
    // Tile-sized recomputation bounds temporary memory; no whole-image reference allocation.
    for i in 0..manifest.tile_count() {
        let tile = manifest.tile(i)?;
        let expected = crate::ray::pixels(manifest, tile)?;
        let row_bytes = tile.width as usize * 3;
        for row in 0..tile.height as usize {
            let offset = ((tile.y as usize + row) * manifest.width as usize + tile.x as usize) * 3;
            require(
                image[offset..offset + row_bytes] == expected[row * row_bytes..(row + 1) * row_bytes],
                "final substituted pixels",
            )?;
        }
    }
    Ok(image)
}
