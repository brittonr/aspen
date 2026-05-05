use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

fn observability_read(resource: &str) -> Operation {
    Operation::ObservabilityRead {
        resource: resource.to_string(),
    }
}

fn observability_write(resource: &str) -> Operation {
    Operation::ObservabilityWrite {
        resource: resource.to_string(),
    }
}

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Trace writes
        ClientRpcRequest::TraceIngest { .. } => Some(Some(observability_write("trace:"))),
        // Trace reads
        ClientRpcRequest::TraceList { .. }
        | ClientRpcRequest::TraceGet { .. }
        | ClientRpcRequest::TraceSearch { .. } => Some(Some(observability_read("trace:"))),
        // Metric writes
        ClientRpcRequest::MetricIngest { .. } => Some(Some(observability_write("metric:"))),
        // Metric reads
        ClientRpcRequest::MetricList { .. } | ClientRpcRequest::MetricQuery { .. } => {
            Some(Some(observability_read("metric:")))
        }
        // Alert writes
        ClientRpcRequest::AlertCreate { .. } | ClientRpcRequest::AlertDelete { .. } => {
            Some(Some(observability_write("alert:rule:")))
        }
        // Alert evaluate (reads metrics, writes state)
        ClientRpcRequest::AlertEvaluate { .. } => Some(Some(observability_write("alert:state:"))),
        // Alert reads
        ClientRpcRequest::AlertList | ClientRpcRequest::AlertGet { .. } => Some(Some(observability_read("alert:"))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use aspen_auth_core::Capability;

    use super::*;
    use crate::messages::observability::AlertComparison;
    use crate::messages::observability::AlertRuleWire;
    use crate::messages::observability::AlertSeverity;
    use crate::messages::observability::MetricDataPoint;
    use crate::messages::observability::MetricTypeWire;

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        let Some(Some(operation)) = to_operation(request) else {
            panic!("request should map to a protected operation");
        };
        operation
    }

    #[test]
    fn observability_requests_use_domain_specific_capabilities() {
        let trace_ingest = operation_for(&ClientRpcRequest::TraceIngest { spans: Vec::new() });
        assert!(matches!(trace_ingest, Operation::ObservabilityWrite { resource } if resource == "trace:"));

        let trace_list = operation_for(&ClientRpcRequest::TraceList {
            start_time_us: None,
            end_time_us: None,
            limit: None,
            continuation_token: None,
        });
        assert!(matches!(trace_list, Operation::ObservabilityRead { resource } if resource == "trace:"));

        let metric_ingest = operation_for(&ClientRpcRequest::MetricIngest {
            data_points: vec![MetricDataPoint {
                name: "cpu".to_string(),
                metric_type: MetricTypeWire::Gauge,
                timestamp_us: 1,
                value: 1.0,
                labels: Vec::new(),
                histogram_buckets: None,
                histogram_sum: None,
                histogram_count: None,
            }],
            ttl_seconds: None,
        });
        assert!(matches!(metric_ingest, Operation::ObservabilityWrite { resource } if resource == "metric:"));

        let metric_query = operation_for(&ClientRpcRequest::MetricQuery {
            name: "cpu".to_string(),
            start_time_us: None,
            end_time_us: None,
            label_filters: Vec::new(),
            aggregation: None,
            step_us: None,
            limit: None,
        });
        assert!(matches!(metric_query, Operation::ObservabilityRead { resource } if resource == "metric:"));

        let alert_create = operation_for(&ClientRpcRequest::AlertCreate {
            rule: AlertRuleWire {
                name: "cpu-high".to_string(),
                metric_name: "cpu".to_string(),
                label_filters: Vec::new(),
                aggregation: "avg".to_string(),
                window_duration_us: 60_000_000,
                comparison: AlertComparison::GreaterThan,
                threshold: 0.9,
                for_duration_us: 0,
                severity: AlertSeverity::Warning,
                description: "cpu high".to_string(),
                is_enabled: true,
                created_at_us: 1,
                updated_at_us: 1,
            },
        });
        assert!(matches!(alert_create, Operation::ObservabilityWrite { resource } if resource == "alert:rule:"));

        let alert_list = operation_for(&ClientRpcRequest::AlertList);
        assert!(matches!(alert_list, Operation::ObservabilityRead { resource } if resource == "alert:"));
    }

    #[test]
    fn generic_sys_prefixes_do_not_authorize_observability_requests() {
        let generic_read = Capability::Read {
            prefix: "_sys:metrics:".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "_sys:metrics:".to_string(),
        };
        let observability_read = Capability::ObservabilityRead {
            resource_prefix: "metric:".to_string(),
        };
        let observability_write = Capability::ObservabilityWrite {
            resource_prefix: "metric:".to_string(),
        };

        let query = operation_for(&ClientRpcRequest::MetricQuery {
            name: "cpu".to_string(),
            start_time_us: None,
            end_time_us: None,
            label_filters: Vec::new(),
            aggregation: None,
            step_us: None,
            limit: None,
        });
        let ingest = operation_for(&ClientRpcRequest::MetricIngest {
            data_points: Vec::new(),
            ttl_seconds: None,
        });

        assert!(!generic_read.authorizes(&query));
        assert!(!generic_write.authorizes(&ingest));
        assert!(observability_read.authorizes(&query));
        assert!(observability_write.authorizes(&ingest));
    }
}
