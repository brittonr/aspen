
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkWatcherInput {
    pub node: String,
    pub interface_state: String,
    pub address_state: String,
    pub default_route: String,
    pub relay_state: String,
    pub endpoint_state: String,
    pub observed_event_count: u64,
    pub retained_event_count: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSample {
    pub name: String,
    pub kind: String,
    pub value: u64,
    pub labels: Vec<(String, String)>,
}
