fn write_explicit_bundle_output(out: &std::path::Path, bundle: &preserves::IOValue) -> crate::error::Result<()> {
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
    }
    std::fs::write(out, crate::preserves_rail::to_text(bundle)?).map_err(crate::error::MoltenError::from)
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/iroh/parts/exchange/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/iroh/parts/exchange/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/iroh/parts/exchange/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/iroh/parts/exchange/p003/body.rs"));
