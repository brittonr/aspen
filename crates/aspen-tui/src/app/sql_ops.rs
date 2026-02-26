//! SQL query operations.

use super::state::App;
use crate::types::MAX_SQL_QUERY_SIZE;
use crate::types::SqlQueryResult;
use crate::types::save_sql_history;

impl App {
    /// Execute SQL query against the cluster.
    pub(crate) async fn execute_sql_query(&mut self) {
        let query = self.sql_state.query_buffer.trim().to_string();

        // Validate query is not empty
        if query.is_empty() {
            self.set_status("No query to execute (press Enter to edit query first)");
            return;
        }

        // Validate query size
        if query.len() > MAX_SQL_QUERY_SIZE {
            self.set_status(&format!("Query too large ({} bytes, max {})", query.len(), MAX_SQL_QUERY_SIZE));
            return;
        }

        // Add to history and save
        self.sql_state.add_to_history(query.clone());
        save_sql_history(&self.sql_state.history);

        // Reset result state
        self.sql_state.selected_row = 0;
        self.sql_state.result_scroll_col = 0;

        self.set_status("Executing query...");

        // Execute query
        match self
            .client
            .execute_sql(
                query,
                self.sql_state.consistency.as_str().to_string(),
                Some(1000), // Default limit
                Some(5000), // Default timeout
            )
            .await
        {
            Ok(result) => {
                if result.is_success {
                    let row_count = result.row_count.unwrap_or(0);
                    let exec_time = result.execution_time_ms.unwrap_or(0);
                    let is_truncated = result.is_truncated.unwrap_or(false);

                    self.sql_state.last_result = Some(SqlQueryResult::from_response(
                        result.columns.unwrap_or_default(),
                        result.rows.unwrap_or_default(),
                        row_count,
                        is_truncated,
                        exec_time,
                    ));

                    let truncated_msg = if is_truncated { " (truncated)" } else { "" };
                    self.set_status(&format!("{} rows in {}ms{}", row_count, exec_time, truncated_msg));
                } else {
                    let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                    self.sql_state.last_result = Some(SqlQueryResult::error(error_msg.clone()));
                    self.set_status(&format!("Query failed: {}", error_msg));
                }
            }
            Err(e) => {
                self.sql_state.last_result = Some(SqlQueryResult::error(format!("{}", e)));
                self.set_status(&format!("Query failed: {}", e));
            }
        }
    }
}
