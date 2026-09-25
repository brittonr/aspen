
fn join_bounded_reader(
    reader: std::thread::JoinHandle<Result<Vec<u8>, String>>,
    label: &str,
) -> super::model::PerformanceResult<Vec<u8>> {
    reader
        .join()
        .map_err(|_| super::model::PerformanceDenial::new(format!("Sightglass {label} reader panicked")))?
        .map_err(|error| super::model::PerformanceDenial::new(format!("Sightglass {label} read failed: {error}")))
}

fn bounded_diagnostic(stderr: &[u8]) -> String {
    let bounded = stderr.get(..MAX_DIAGNOSTIC_STDERR_BYTES).unwrap_or(stderr);
    String::from_utf8_lossy(bounded).into_owned()
}
