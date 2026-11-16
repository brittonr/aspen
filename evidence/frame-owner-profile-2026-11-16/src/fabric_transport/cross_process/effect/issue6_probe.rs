// Task-owned diagnostic instrumentation; not a product API or production baseline.
pub(crate) mod issue6_probe {
    use std::collections::BTreeMap;
    use std::sync::{LazyLock, Mutex};
    use std::time::Duration;

    static SAMPLES: LazyLock<Mutex<BTreeMap<&'static str, u64>>> =
        LazyLock::new(|| Mutex::new(BTreeMap::new()));

    pub(crate) fn reset() {
        SAMPLES.lock().expect("probe samples").clear();
    }

    pub(crate) fn record(stage: &'static str, elapsed: Duration) {
        let ns = u64::try_from(elapsed.as_nanos()).expect("bounded duration");
        SAMPLES.lock().expect("probe samples").insert(stage, ns);
    }

    pub(crate) fn take() -> BTreeMap<&'static str, u64> {
        std::mem::take(&mut *SAMPLES.lock().expect("probe samples"))
    }
}
