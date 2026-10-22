// The retractable service demand record and the demand set derived from the
// live dataspace assertions.
//
// A demand is a requirement, not a counter: one live demand record per owner
// and service. The demand assertion is owned by the demanding scope, so
// retracting it withdraws the demand and owner-scope cleanup removes a dead
// demander's facts with the rest of its scope.

pub const SERVICE_DEMAND_SCHEMA: &str = "runtime-service-demand-v1";
const SERVICE_DEMAND_RECORD: &str = "runtime-service-demand-v1";
const SERVICE_DEMAND_FIELD_COUNT: usize = 5;
const MAX_SERVICE_DEMAND_RECORDS: usize = 1_024;

/// The two demand forms. Dependency-gated demand waits for the declared
/// dependencies; force-run demand bypasses that order only because the record
/// declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceDemandKind {
    DependencyGated,
    ForceRun,
}

impl ServiceDemandKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DependencyGated => "dependency-gated",
            Self::ForceRun => "force-run",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dependency-gated" => Some(Self::DependencyGated),
            "force-run" => Some(Self::ForceRun),
            _ => None,
        }
    }
}

/// One demand assertion: the service, the demand form, the demanding scope,
/// and the operation identity that created the demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDemandAssertion {
    pub service_ref: String,
    pub kind: ServiceDemandKind,
    pub demander_ref: String,
    pub operation_identity: String,
}

impl ServiceDemandAssertion {
    /// The canonical demand record.
    pub fn to_value(&self) -> IoValue {
        crate::preserves_rail::record(SERVICE_DEMAND_RECORD, vec![
            crate::preserves_rail::string(SERVICE_DEMAND_SCHEMA),
            crate::preserves_rail::string(&self.service_ref),
            crate::preserves_rail::string(self.kind.as_str()),
            crate::preserves_rail::string(&self.demander_ref),
            crate::preserves_rail::string(&self.operation_identity),
        ])
    }

    /// The demand record as an assertion value.
    pub fn to_assertion_value(&self) -> Result<Value> {
        Value::new(self.to_value())
    }

    /// Decode one demand record.
    pub fn from_value(value: &IoValue) -> Result<Self> {
        let fields = value
            .collect_simple_record(SERVICE_DEMAND_RECORD, None)
            .ok_or_else(|| MoltenError::invalid_harness("the service demand record is malformed".to_string()))?;
        let field_count = fields.fields_iter().count();
        if field_count != SERVICE_DEMAND_FIELD_COUNT {
            return Err(MoltenError::invalid_harness(format!(
                "the service demand record has {field_count} fields, expected {SERVICE_DEMAND_FIELD_COUNT}"
            )));
        }
        let schema = crate::preserves_rail::required_string_field(&fields[0], "demand schema")
            .map_err(|error| MoltenError::invalid_harness(format!("the demand schema is unreadable: {error}")))?;
        if schema != SERVICE_DEMAND_SCHEMA {
            return Err(MoltenError::invalid_harness(
                "the service demand schema is not admitted".to_string(),
            ));
        }
        let service_ref = crate::preserves_rail::required_string_field(&fields[1], "demand service")
            .map_err(|error| MoltenError::invalid_harness(format!("the demanded service is unreadable: {error}")))?;
        let kind = crate::preserves_rail::required_string_field(&fields[2], "demand kind")
            .map_err(|error| MoltenError::invalid_harness(format!("the demand kind is unreadable: {error}")))?;
        let demander_ref = crate::preserves_rail::required_string_field(&fields[3], "demand demander")
            .map_err(|error| MoltenError::invalid_harness(format!("the demander is unreadable: {error}")))?;
        let operation_identity = crate::preserves_rail::required_string_field(&fields[4], "demand operation")
            .map_err(|error| MoltenError::invalid_harness(format!("the demand operation identity is unreadable: {error}")))?;
        let kind = ServiceDemandKind::parse(&kind).ok_or_else(|| {
            MoltenError::invalid_harness(format!("the demand kind {kind} is not admitted"))
        })?;
        Ok(Self {
            service_ref,
            kind,
            demander_ref,
            operation_identity,
        })
    }
}

// r[impl molten.runtime_spine.demand_assertion_lifetime]
/// The live demand set derived from the dataspace's assertions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceDemandSet {
    demands: Vec<ServiceDemandAssertion>,
}

impl ServiceDemandSet {
    /// Derive the live demand set from the live assertions.
    ///
    /// Assertions that are not demand records are ignored. A demand record that
    /// is malformed is a typed failure, because a half-read demand must never
    /// look like no demand.
    pub fn derive(assertions: &[Assertion]) -> std::result::Result<Self, Vec<String>> {
        let mut diagnostics = Vec::with_capacity(4);
        let mut demands: Vec<ServiceDemandAssertion> =
            Vec::with_capacity(MAX_SERVICE_DEMAND_RECORDS.min(assertions.len()));
        for assertion in assertions {
            let value = assertion.value.as_iovalue();
            if !is_demand_record(value) {
                continue;
            }
            match ServiceDemandAssertion::from_value(value) {
                Ok(demand) => {
                    let is_live = !demands.iter().any(|existing| {
                        existing.demander_ref == demand.demander_ref && existing.service_ref == demand.service_ref
                    });
                    if is_live {
                        if demands.len() >= MAX_SERVICE_DEMAND_RECORDS {
                            diagnostics.push("the live service demand set exceeds its bound".to_string());
                            break;
                        }
                        demands.push(demand);
                    }
                }
                Err(error) => diagnostics.push(error.to_string()),
            }
        }
        if diagnostics.is_empty() {
            Ok(Self { demands })
        } else {
            Err(diagnostics)
        }
    }

    /// The demanded services, in a stable order.
    pub fn demanded_service_refs(&self) -> Vec<String> {
        let mut refs: Vec<String> = Vec::with_capacity(self.demands.len());
        for demand in &self.demands {
            if !refs.contains(&demand.service_ref) {
                refs.push(demand.service_ref.clone());
            }
        }
        refs.sort();
        refs
    }

    /// The services whose live demand declares force-run.
    pub fn force_run_service_refs(&self) -> Vec<String> {
        let mut refs: Vec<String> = Vec::with_capacity(self.demands.len());
        for demand in &self.demands {
            if demand.kind == ServiceDemandKind::ForceRun && !refs.contains(&demand.service_ref) {
                refs.push(demand.service_ref.clone());
            }
        }
        refs.sort();
        refs
    }

    /// The demanding scopes that still demand the service.
    pub fn demanders_for(&self, service_ref: &str) -> Vec<String> {
        let mut refs: Vec<String> = Vec::with_capacity(self.demands.len());
        for demand in &self.demands {
            if demand.service_ref == service_ref && !refs.contains(&demand.demander_ref) {
                refs.push(demand.demander_ref.clone());
            }
        }
        refs.sort();
        refs
    }

    /// The number of live demands for the service.
    pub fn live_demand_count(&self, service_ref: &str) -> u64 {
        let mut count = 0_u64;
        for demand in &self.demands {
            if demand.service_ref == service_ref {
                count = count.saturating_add(1);
            }
        }
        count
    }

    /// Whether the last live demand for the service is withdrawn.
    ///
    /// The rule is about demand only: the normal shutdown admission, resource,
    /// and receipt gates still apply.
    pub fn is_shutdown_eligible(&self, service_ref: &str) -> bool {
        self.live_demand_count(service_ref) == 0
    }
}

fn is_demand_record(value: &IoValue) -> bool {
    value.collect_simple_record(SERVICE_DEMAND_RECORD, None).is_some()
}
