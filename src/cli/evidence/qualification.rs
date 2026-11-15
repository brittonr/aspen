//! Imperative CLI adapter; no operation or release admission semantics.

pub(crate) fn run(
    source_root: std::path::PathBuf,
    scenario: String,
    out: std::path::PathBuf,
) -> molten::error::Result<()> {
    let root = molten::node_state::DirectoryRoot::open_existing(&source_root)?;
    if scenario == "describe" {
        if out != std::path::Path::new("-") {
            return Err(molten::error::Failure::invalid_harness("subject description requires --out -"));
        }
        let observed = molten::capability_qualification::observe_subject(&root)?;
        println!(
            "{}",
            serde_json::to_string(&observed)
                .map_err(|_| molten::error::Failure::invalid_harness("cannot render observed native subject"))?
        );
        return Ok(());
    }
    if scenario == "readback" {
        if out != std::path::Path::new("-") {
            return Err(molten::error::Failure::invalid_harness("runtime readback requires --out -"));
        }
        let observed = molten::capability_qualification::observe_runs(&root)?;
        println!(
            "{}",
            serde_json::to_string(&observed)
                .map_err(|_| molten::error::Failure::invalid_harness("cannot render actual runtime readback"))?
        );
        return Ok(());
    }
    let scenario = molten::capability_qualification::Scenario::parse(&scenario)?;
    let produced = molten::capability_qualification::produce(&root, scenario)?;
    let parent = out.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
    let filename = out
        .file_name()
        .ok_or_else(|| molten::error::Failure::invalid_harness("qualification output requires a filename"))?;
    let parent = std::fs::canonicalize(parent).map_err(molten::error::Failure::from)?;
    let directory = molten::node_state::DirectoryRoot::open_existing(&parent)?;
    let relative = molten::node_state::RelativePath::parse(
        filename
            .to_str()
            .ok_or_else(|| molten::error::Failure::invalid_harness("qualification output filename must be UTF-8"))?,
    )?;
    directory.write_new(&relative, molten::preserves_rail::to_text(&produced.value)?.as_bytes())?;
    println!(
        "qualification execution {} scenario={} decision={} environment=simulation",
        produced.receipt_ref,
        scenario.id(),
        produced.decision
    );
    if produced.decision != "pass" {
        return Err(molten::error::Failure::invalid_harness(
            "native qualification execution failed; canonical failure retained",
        ));
    }
    Ok(())
}
