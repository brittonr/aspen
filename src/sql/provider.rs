//! DataFusion TableProvider implementation for Redb KV storage.
//!
//! This module implements the DataFusion `TableProvider` trait for the Redb
//! key-value store, enabling SQL queries against the KV data.

use std::any::Any;
use std::fmt::{self, Debug};
use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use async_trait::async_trait;
use datafusion::catalog::Session;
use datafusion::common::Result;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::execution::context::TaskContext;
use datafusion::logical_expr::{Expr, Operator, TableProviderFilterPushDown};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, SendableRecordBatchStream,
};
use redb::Database;

use super::schema::KV_SCHEMA;
use super::stream::{full_scan_stream, prefix_scan_stream, range_scan_stream};

/// A TableProvider that reads from Redb KV storage.
///
/// This provider exposes the Redb KV table as a virtual SQL table named `kv`
/// with the following columns:
/// - `key`: The key string
/// - `value`: The value string
/// - `version`: Per-key version counter
/// - `create_revision`: Raft log index when created
/// - `mod_revision`: Raft log index of last modification
/// - `expires_at_ms`: Optional TTL expiration timestamp
/// - `lease_id`: Optional attached lease ID
#[derive(Clone)]
pub struct RedbTableProvider {
    /// Reference to the Redb database.
    db: Arc<Database>,
}

impl Debug for RedbTableProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedbTableProvider")
            .field("table", &"kv")
            .finish()
    }
}

impl RedbTableProvider {
    /// Create a new TableProvider for the given Redb database.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TableProvider for RedbTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        KV_SCHEMA.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> Result<Vec<TableProviderFilterPushDown>> {
        // Analyze each filter to determine if we can push it down
        filters
            .iter()
            .map(|filter| {
                if can_pushdown_filter(filter) {
                    Ok(TableProviderFilterPushDown::Exact)
                } else {
                    Ok(TableProviderFilterPushDown::Unsupported)
                }
            })
            .collect()
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Extract key range from filters
        let key_range = extract_key_range(filters);

        Ok(Arc::new(RedbScanExec::new(
            self.db.clone(),
            projection.cloned(),
            key_range,
            limit,
        )))
    }
}

/// Extracted key range from filter predicates.
#[derive(Debug, Clone, Default)]
pub struct KeyRange {
    /// Start key (inclusive). None means scan from beginning.
    pub start: Option<Vec<u8>>,
    /// End key (exclusive). None means scan to end.
    pub end: Option<Vec<u8>>,
    /// Whether this is a prefix scan (end is computed from start).
    pub is_prefix: bool,
    /// Exact key match (overrides range).
    pub exact: Option<Vec<u8>>,
}

/// Check if a filter expression can be pushed down to storage.
fn can_pushdown_filter(expr: &Expr) -> bool {
    match expr {
        // key = 'value' -> exact lookup
        Expr::BinaryExpr(binary) => {
            if is_key_column(&binary.left) {
                matches!(
                    binary.op,
                    Operator::Eq
                        | Operator::Lt
                        | Operator::LtEq
                        | Operator::Gt
                        | Operator::GtEq
                        | Operator::LikeMatch
                )
            } else if is_key_column(&binary.right) {
                matches!(
                    binary.op,
                    Operator::Eq | Operator::Lt | Operator::LtEq | Operator::Gt | Operator::GtEq
                )
            } else {
                false
            }
        }
        // key LIKE 'prefix%' -> prefix scan
        Expr::Like(like) => is_key_column(&like.expr) && is_prefix_pattern(&like.pattern),
        _ => false,
    }
}

/// Check if an expression references the `key` column.
fn is_key_column(expr: &Expr) -> bool {
    matches!(expr, Expr::Column(col) if col.name == "key")
}

/// Check if a LIKE pattern is a prefix pattern (ends with %).
fn is_prefix_pattern(expr: &Expr) -> bool {
    if let Expr::Literal(scalar) = expr
        && let Some(s) = scalar.try_as_str().flatten()
    {
        return s.ends_with('%') && !s[..s.len() - 1].contains('%');
    }
    false
}

/// Extract a string literal value from an expression.
fn extract_string_literal(expr: &Expr) -> Option<String> {
    if let Expr::Literal(scalar) = expr {
        scalar.try_as_str().flatten().map(|s| s.to_string())
    } else {
        None
    }
}

/// Extract key range constraints from filter expressions.
fn extract_key_range(filters: &[Expr]) -> KeyRange {
    let mut range = KeyRange::default();

    for filter in filters {
        match filter {
            Expr::BinaryExpr(binary) => {
                // Handle key op literal
                if is_key_column(&binary.left) {
                    if let Some(value) = extract_string_literal(&binary.right) {
                        match binary.op {
                            Operator::Eq => {
                                range.exact = Some(value.into_bytes());
                            }
                            Operator::Gt => {
                                let mut key = value.into_bytes();
                                key.push(0); // Strictly greater
                                update_start(&mut range.start, key);
                            }
                            Operator::GtEq => {
                                update_start(&mut range.start, value.into_bytes());
                            }
                            Operator::Lt => {
                                update_end(&mut range.end, value.into_bytes());
                            }
                            Operator::LtEq => {
                                let mut key = value.into_bytes();
                                key.push(0xFF); // Include this key
                                update_end(&mut range.end, key);
                            }
                            _ => {}
                        }
                    }
                }
                // Handle literal op key (reversed)
                else if is_key_column(&binary.right)
                    && let Some(value) = extract_string_literal(&binary.left)
                {
                    match binary.op {
                        Operator::Eq => {
                            range.exact = Some(value.into_bytes());
                        }
                        // literal < key means key > literal
                        Operator::Lt => {
                            let mut key = value.into_bytes();
                            key.push(0);
                            update_start(&mut range.start, key);
                        }
                        // literal <= key means key >= literal
                        Operator::LtEq => {
                            update_start(&mut range.start, value.into_bytes());
                        }
                        // literal > key means key < literal
                        Operator::Gt => {
                            update_end(&mut range.end, value.into_bytes());
                        }
                        // literal >= key means key <= literal
                        Operator::GtEq => {
                            let mut key = value.into_bytes();
                            key.push(0xFF);
                            update_end(&mut range.end, key);
                        }
                        _ => {}
                    }
                }
            }
            Expr::Like(like) if is_key_column(&like.expr) => {
                if let Some(pattern) = extract_string_literal(&like.pattern)
                    && pattern.ends_with('%')
                    && !pattern[..pattern.len() - 1].contains('%')
                {
                    let prefix = &pattern[..pattern.len() - 1];
                    range.start = Some(prefix.as_bytes().to_vec());
                    range.is_prefix = true;
                    // Compute end key using strinc
                    range.end = strinc(prefix.as_bytes());
                }
            }
            _ => {}
        }
    }

    range
}

/// Update start bound to the maximum (most restrictive).
fn update_start(current: &mut Option<Vec<u8>>, new: Vec<u8>) {
    match current {
        Some(existing) if existing.as_slice() >= new.as_slice() => {}
        _ => *current = Some(new),
    }
}

/// Update end bound to the minimum (most restrictive).
fn update_end(current: &mut Option<Vec<u8>>, new: Vec<u8>) {
    match current {
        Some(existing) if existing.as_slice() <= new.as_slice() => {}
        _ => *current = Some(new),
    }
}

/// Compute the strict upper bound for a byte string (FoundationDB strinc).
fn strinc(key: &[u8]) -> Option<Vec<u8>> {
    for i in (0..key.len()).rev() {
        if key[i] != 0xFF {
            let mut result = key[..=i].to_vec();
            result[i] += 1;
            return Some(result);
        }
    }
    None
}

/// ExecutionPlan for scanning Redb KV storage.
#[derive(Debug)]
pub struct RedbScanExec {
    /// Reference to the Redb database.
    db: Arc<Database>,
    /// Schema of the output (after projection).
    schema: SchemaRef,
    /// Column indices to project (None = all columns).
    projection: Option<Vec<usize>>,
    /// Key range to scan.
    key_range: KeyRange,
    /// Maximum rows to return.
    limit: Option<usize>,
    /// Plan properties for DataFusion optimizer.
    properties: PlanProperties,
}

impl RedbScanExec {
    /// Create a new scan execution plan.
    pub fn new(
        db: Arc<Database>,
        projection: Option<Vec<usize>>,
        key_range: KeyRange,
        limit: Option<usize>,
    ) -> Self {
        // Compute the output schema based on projection.
        // For empty projection (e.g., COUNT(*)), we report an empty schema
        // but internally use a single column to count rows.
        let schema = match &projection {
            Some(indices) if indices.is_empty() => {
                // Empty projection means no columns needed - just counting rows
                Arc::new(arrow::datatypes::Schema::empty())
            }
            Some(indices) => Arc::new(KV_SCHEMA.project(indices).expect("valid projection")),
            None => KV_SCHEMA.clone(),
        };

        // Build plan properties
        let eq_props = EquivalenceProperties::new(schema.clone());
        let properties = PlanProperties::new(
            eq_props,
            datafusion::physical_plan::Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );

        Self {
            db,
            schema,
            projection,
            key_range,
            limit,
            properties,
        }
    }
}

impl DisplayAs for RedbScanExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "RedbScanExec: ")?;
                if let Some(exact) = &self.key_range.exact {
                    write!(f, "key={:?}", String::from_utf8_lossy(exact))?;
                } else if self.key_range.is_prefix {
                    if let Some(start) = &self.key_range.start {
                        write!(f, "prefix={:?}", String::from_utf8_lossy(start))?;
                    }
                } else {
                    write!(f, "range=[")?;
                    if let Some(start) = &self.key_range.start {
                        write!(f, "{:?}", String::from_utf8_lossy(start))?;
                    }
                    write!(f, ", ")?;
                    if let Some(end) = &self.key_range.end {
                        write!(f, "{:?}", String::from_utf8_lossy(end))?;
                    }
                    write!(f, ")")?;
                }
                if let Some(limit) = self.limit {
                    write!(f, ", limit={}", limit)?;
                }
                if let Some(proj) = &self.projection {
                    write!(f, ", projection={:?}", proj)?;
                }
                Ok(())
            }
        }
    }
}

impl ExecutionPlan for RedbScanExec {
    fn name(&self) -> &str {
        "RedbScanExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![] // Leaf node
    }

    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self) // No children to replace
    }

    fn execute(
        &self,
        _partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let stream = if let Some(exact) = &self.key_range.exact {
            // Exact key lookup - use range scan with tight bounds
            range_scan_stream(
                self.db.clone(),
                exact,
                &{
                    let mut end = exact.clone();
                    end.push(0);
                    end
                },
                self.projection.clone(),
                self.limit,
            )
        } else if self.key_range.is_prefix {
            // Prefix scan
            let prefix = self.key_range.start.as_deref().unwrap_or(&[]);
            prefix_scan_stream(self.db.clone(), prefix, self.projection.clone(), self.limit)
        } else {
            // Range scan or full scan
            let start = self.key_range.start.as_deref().unwrap_or(&[]);
            let end = self.key_range.end.as_deref().unwrap_or(&[]);
            if start.is_empty() && end.is_empty() {
                full_scan_stream(self.db.clone(), self.projection.clone(), self.limit)
            } else {
                range_scan_stream(
                    self.db.clone(),
                    start,
                    end,
                    self.projection.clone(),
                    self.limit,
                )
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::logical_expr::col;
    use datafusion::prelude::lit;

    #[test]
    fn extract_exact_key() {
        let filters = vec![col("key").eq(lit("mykey"))];
        let range = extract_key_range(&filters);
        assert_eq!(range.exact, Some(b"mykey".to_vec()));
    }

    #[test]
    fn extract_prefix_from_like() {
        let filter = Expr::Like(datafusion::logical_expr::Like {
            negated: false,
            expr: Box::new(col("key")),
            pattern: Box::new(lit("prefix%")),
            escape_char: None,
            case_insensitive: false,
        });
        let range = extract_key_range(&[filter]);
        assert!(range.is_prefix);
        assert_eq!(range.start, Some(b"prefix".to_vec()));
        assert_eq!(range.end, Some(b"prefiy".to_vec())); // strinc("prefix")
    }

    #[test]
    fn extract_range() {
        let filters = vec![col("key").gt_eq(lit("start")), col("key").lt(lit("stop"))];
        let range = extract_key_range(&filters);
        assert_eq!(range.start, Some(b"start".to_vec()));
        assert_eq!(range.end, Some(b"stop".to_vec()));
        assert!(!range.is_prefix);
        assert!(range.exact.is_none());
    }
}
