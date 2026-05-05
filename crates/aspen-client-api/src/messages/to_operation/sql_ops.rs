use alloc::string::String;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

fn sql_resource() -> String {
    "query".to_string()
}

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // SQL queries
        ClientRpcRequest::ExecuteSql { .. } => Some(Some(Operation::SqlRead {
            resource: sql_resource(),
        })),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use aspen_auth_core::Capability;

    use super::*;

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        to_operation(request).flatten().expect("request should require auth")
    }

    #[test]
    fn sql_queries_use_domain_specific_capability() {
        let operation = operation_for(&ClientRpcRequest::ExecuteSql {
            query: "select * from kv".to_string(),
            params: "[]".to_string(),
            consistency: "linearizable".to_string(),
            limit: Some(10),
            timeout_ms: Some(1_000),
        });

        assert!(matches!(operation, Operation::SqlRead { resource } if resource == "query"));
    }

    #[test]
    fn generic_sql_prefix_does_not_authorize_sql_queries() {
        let generic_read = Capability::Read {
            prefix: "_sql:".to_string(),
        };
        let sql_read = Capability::SqlRead {
            resource_prefix: String::new(),
        };
        let operation = operation_for(&ClientRpcRequest::ExecuteSql {
            query: "select * from kv".to_string(),
            params: "[]".to_string(),
            consistency: "linearizable".to_string(),
            limit: Some(10),
            timeout_ms: Some(1_000),
        });

        assert!(!generic_read.authorizes(&operation));
        assert!(sql_read.authorizes(&operation));
    }
}
