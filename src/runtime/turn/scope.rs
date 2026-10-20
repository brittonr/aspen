use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeFacetState {
    Running,
    Stopped,
}

impl RuntimeFacetState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeFacet {
    pub facet_id: String,
    pub actor: String,
    pub parent: Option<String>,
    pub state: RuntimeFacetState,
    pub stop_handler_ref: Option<String>,
}

impl RuntimeFacet {
    pub fn to_value(&self) -> IoValue {
        record("runtime-facet-v1", vec![
            string(&self.facet_id),
            string(&self.actor),
            string(self.parent.as_deref().unwrap_or("")),
            string(self.state.as_str()),
            string(self.stop_handler_ref.as_deref().unwrap_or("")),
        ])
    }

    pub fn facet_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }
}
