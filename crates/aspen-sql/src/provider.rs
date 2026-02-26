//! DataFusion TableProvider implementation for Redb KV storage.
//!
//! This module implements the DataFusion `TableProvider` trait for the Redb
//! key-value store, enabling SQL queries against the KV data.
//!
//! ## Index Integration
//!
//! The provider supports filter pushdown for both key-based predicates and
//! secondary index predicates:
//!
//! - **Key predicates**: `key = 'x'`, `key LIKE 'prefix%'`, `key > 'a'`
//! - **Indexed column predicates**: `mod_revision = 100`, `create_revision > 50`
//!
//! When an indexed column predicate is detected, the provider uses the
//! secondary index for efficient lookup instead of full table scan.

use std::any::Any;
use std::fmt::Debug;
use std::fmt::{self};
use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use aspen_core::layer::IndexRegistry;
use async_trait::async_trait;
use datafusion::catalog::Session;
use datafusion::common::Result;
use datafusion::datasource::TableProvider;
use datafusion::datasource::TableType;
use datafusion::execution::context::TaskContext;
use datafusion::logical_expr::Expr;
use datafusion::logical_expr::Operator;
use datafusion::logical_expr::TableProviderFilterPushDown;
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::DisplayAs;
use datafusion::physical_plan::DisplayFormatType;
use datafusion::physical_plan::ExecutionPlan;
use datafusion::physical_plan::PlanProperties;
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::physical_plan::execution_plan::Boundedness;
use datafusion::physical_plan::execution_plan::EmissionType;
use redb::Database;

use super::schema::KV_SCHEMA;
use super::stream::full_scan_stream;
use super::stream::index_scan_stream;
use super::stream::prefix_scan_stream;
use super::stream::range_scan_stream;

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
    /// Index registry for secondary index lookups.
    index_registry: Option<Arc<IndexRegistry>>,
}

impl Debug for RedbTableProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedbTableProvider")
            .field("table", &"kv")
            .field("has_indexes", &self.index_registry.is_some())
            .finish()
    }
}

impl RedbTableProvider {
    /// Create a new TableProvider for the given Redb database.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            index_registry: None,
        }
    }

    /// Create a new TableProvider with secondary index support.
    ///
    /// When indexes are provided, the provider can use secondary indexes
    /// for efficient lookups on indexed columns like `mod_revision`.
    pub fn with_indexes(db: Arc<Database>, index_registry: Arc<IndexRegistry>) -> Self {
        Self {
            db,
            index_registry: Some(index_registry),
        }
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

    fn supports_filters_pushdown(&self, filters: &[&Expr]) -> Result<Vec<TableProviderFilterPushDown>> {
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
        // Extract scan strategy from filters
        let strategy = extract_scan_strategy(filters);

        Ok(Arc::new(RedbScanExec::new(
            self.db.clone(),
            self.index_registry.clone(),
            projection.cloned(),
            strategy,
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

/// Specification for an index-based scan.
#[derive(Debug, Clone)]
pub struct IndexScanSpec {
    /// Name of the index to use (e.g., "idx_mod_revision").
    pub index_name: String,
    /// Start value for range scan (None = beginning).
    pub start_value: Option<Vec<u8>>,
    /// End value for range scan (None = end).
    pub end_value: Option<Vec<u8>>,
    /// Whether this is an exact match (overrides range).
    pub is_exact: bool,
    /// The exact value to look up (for exact match).
    pub exact_value: Option<Vec<u8>>,
}

/// Combined scan strategy for the execution plan.
///
/// The execution plan uses this to determine the optimal scan method:
/// 1. If `index_scan` is set and supported, use index scan
/// 2. Otherwise, fall back to key range scan
#[derive(Debug, Clone, Default)]
pub struct ScanStrategy {
    /// Key range constraints (from key column predicates).
    pub key_range: KeyRange,
    /// Index scan specification (from indexed column predicates).
    pub index_scan: Option<IndexScanSpec>,
}

/// Check if a filter expression can be pushed down to storage.
fn can_pushdown_filter(expr: &Expr) -> bool {
    match expr {
        // key = 'value' -> exact lookup
        Expr::BinaryExpr(binary) => {
            let left_key = is_key_column(&binary.left);
            let right_key = is_key_column(&binary.right);
            let left_indexed = is_indexed_column(&binary.left);
            let right_indexed = is_indexed_column(&binary.right);

            if left_key {
                matches!(
                    binary.op,
                    Operator::Eq | Operator::Lt | Operator::LtEq | Operator::Gt | Operator::GtEq | Operator::LikeMatch
                )
            } else if right_key {
                matches!(binary.op, Operator::Eq | Operator::Lt | Operator::LtEq | Operator::Gt | Operator::GtEq)
            } else if left_indexed || right_indexed {
                // Support indexed column predicates: mod_revision = 100, etc.
                matches!(binary.op, Operator::Eq | Operator::Lt | Operator::LtEq | Operator::Gt | Operator::GtEq)
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

/// Check if an expression references an indexed column.
///
/// Indexed columns have secondary indexes maintained by the storage layer:
/// - `mod_revision`: Index `idx_mod_revision`
/// - `create_revision`: Index `idx_create_revision`
/// - `expires_at_ms`: Index `idx_expires_at`
/// - `lease_id`: Index `idx_lease_id`
fn is_indexed_column(expr: &Expr) -> bool {
    matches!(expr, Expr::Column(col) if matches!(
        col.name.as_str(),
        "mod_revision" | "create_revision" | "expires_at_ms" | "lease_id"
    ))
}

/// Get the index name for an indexed column.
fn index_name_for_column(col_name: &str) -> Option<&'static str> {
    match col_name {
        "mod_revision" => Some("idx_mod_revision"),
        "create_revision" => Some("idx_create_revision"),
        "expires_at_ms" => Some("idx_expires_at"),
        "lease_id" => Some("idx_lease_id"),
        _ => None,
    }
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

/// Extract an integer literal value from an expression.
fn extract_int_literal(expr: &Expr) -> Option<i64> {
    if let Expr::Literal(scalar) = expr {
        // Try to extract as i64 or u64
        use datafusion::scalar::ScalarValue;
        match scalar {
            ScalarValue::Int8(Some(v)) => Some(*v as i64),
            ScalarValue::Int16(Some(v)) => Some(*v as i64),
            ScalarValue::Int32(Some(v)) => Some(*v as i64),
            ScalarValue::Int64(Some(v)) => Some(*v),
            ScalarValue::UInt8(Some(v)) => Some(*v as i64),
            ScalarValue::UInt16(Some(v)) => Some(*v as i64),
            ScalarValue::UInt32(Some(v)) => Some(*v as i64),
            ScalarValue::UInt64(Some(v)) => Some(*v as i64),
            _ => None,
        }
    } else {
        None
    }
}

/// Get the column name from an expression if it's a column reference.
fn get_column_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Column(col) => Some(&col.name),
        _ => None,
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

/// Extract scan strategy (key range + index scan) from filter expressions.
fn extract_scan_strategy(filters: &[Expr]) -> ScanStrategy {
    let key_range = extract_key_range(filters);
    let index_scan = extract_index_scan(filters);

    ScanStrategy { key_range, index_scan }
}

/// Extract index scan specification from filter expressions.
///
/// Looks for predicates on indexed columns (mod_revision, create_revision, etc.)
/// and extracts the first usable index scan specification.
fn extract_index_scan(filters: &[Expr]) -> Option<IndexScanSpec> {
    let mut spec: Option<IndexScanSpec> = None;

    for filter in filters {
        if let Expr::BinaryExpr(binary) = filter {
            // Check if left side is an indexed column
            let (col_name, value) = if let Some(col) = get_column_name(&binary.left)
                && is_indexed_column(&binary.left)
            {
                if let Some(v) = extract_int_literal(&binary.right) {
                    (col, v)
                } else {
                    continue;
                }
            } else if let Some(col) = get_column_name(&binary.right)
                && is_indexed_column(&binary.right)
            {
                // Reversed: literal op column
                if let Some(v) = extract_int_literal(&binary.left) {
                    (col, v)
                } else {
                    continue;
                }
            } else {
                continue;
            };

            // Get the index name for this column
            let Some(index_name) = index_name_for_column(col_name) else {
                continue;
            };

            // Convert value to big-endian bytes (matching index key format)
            let value_bytes = value.to_be_bytes().to_vec();

            // Create or update the spec
            let current = spec.get_or_insert_with(|| IndexScanSpec {
                index_name: index_name.to_string(),
                start_value: None,
                end_value: None,
                is_exact: false,
                exact_value: None,
            });

            // Only process if same index (can't combine different indexes)
            if current.index_name != index_name {
                continue;
            }

            // Determine if the column is on the left or right
            let col_on_left = get_column_name(&binary.left).is_some() && is_indexed_column(&binary.left);

            match binary.op {
                Operator::Eq => {
                    current.is_exact = true;
                    current.exact_value = Some(value_bytes);
                }
                Operator::Gt => {
                    if col_on_left {
                        // col > value: start = value + 1
                        let start = (value + 1).to_be_bytes().to_vec();
                        current.start_value = Some(start);
                    } else {
                        // value > col: end = value
                        current.end_value = Some(value_bytes);
                    }
                }
                Operator::GtEq => {
                    if col_on_left {
                        // col >= value: start = value
                        current.start_value = Some(value_bytes);
                    } else {
                        // value >= col: end = value + 1
                        let end = (value + 1).to_be_bytes().to_vec();
                        current.end_value = Some(end);
                    }
                }
                Operator::Lt => {
                    if col_on_left {
                        // col < value: end = value
                        current.end_value = Some(value_bytes);
                    } else {
                        // value < col: start = value + 1
                        let start = (value + 1).to_be_bytes().to_vec();
                        current.start_value = Some(start);
                    }
                }
                Operator::LtEq => {
                    if col_on_left {
                        // col <= value: end = value + 1
                        let end = (value + 1).to_be_bytes().to_vec();
                        current.end_value = Some(end);
                    } else {
                        // value <= col: start = value
                        current.start_value = Some(value_bytes);
                    }
                }
                _ => {}
            }
        }
    }

    spec
}

/// ExecutionPlan for scanning Redb KV storage.
pub struct RedbScanExec {
    /// Reference to the Redb database.
    db: Arc<Database>,
    /// Index registry for secondary index lookups.
    index_registry: Option<Arc<IndexRegistry>>,
    /// Schema of the output (after projection).
    schema: SchemaRef,
    /// Column indices to project (None = all columns).
    projection: Option<Vec<usize>>,
    /// Scan strategy (key range and/or index scan).
    strategy: ScanStrategy,
    /// Maximum rows to return.
    limit: Option<usize>,
    /// Plan properties for DataFusion optimizer.
    properties: PlanProperties,
}

impl Debug for RedbScanExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedbScanExec")
            .field("db", &"<Database>")
            .field("has_index_registry", &self.index_registry.is_some())
            .field("schema", &self.schema)
            .field("projection", &self.projection)
            .field("strategy", &self.strategy)
            .field("limit", &self.limit)
            .finish()
    }
}

impl RedbScanExec {
    /// Create a new scan execution plan.
    pub fn new(
        db: Arc<Database>,
        index_registry: Option<Arc<IndexRegistry>>,
        projection: Option<Vec<usize>>,
        strategy: ScanStrategy,
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
            // SAFETY: DataFusion provides projection indices that are valid for KV_SCHEMA.
            // Invalid indices would be a bug in DataFusion's query planning.
            Some(indices) => Arc::new(KV_SCHEMA.project(indices).expect("DataFusion projection indices must be valid")),
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
            index_registry,
            schema,
            projection,
            strategy,
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

                // Show index scan if present
                if let Some(idx_scan) = &self.strategy.index_scan {
                    if idx_scan.is_exact {
                        write!(f, "index={}, exact_match", idx_scan.index_name)?;
                    } else {
                        write!(f, "index={}, range_scan", idx_scan.index_name)?;
                    }
                } else if let Some(exact) = &self.strategy.key_range.exact {
                    write!(f, "key={:?}", String::from_utf8_lossy(exact))?;
                } else if self.strategy.key_range.is_prefix {
                    if let Some(start) = &self.strategy.key_range.start {
                        write!(f, "prefix={:?}", String::from_utf8_lossy(start))?;
                    }
                } else {
                    write!(f, "range=[")?;
                    if let Some(start) = &self.strategy.key_range.start {
                        write!(f, "{:?}", String::from_utf8_lossy(start))?;
                    }
                    write!(f, ", ")?;
                    if let Some(end) = &self.strategy.key_range.end {
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

    fn with_new_children(self: Arc<Self>, _children: Vec<Arc<dyn ExecutionPlan>>) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self) // No children to replace
    }

    fn execute(&self, _partition: usize, _context: Arc<TaskContext>) -> Result<SendableRecordBatchStream> {
        // Try index scan first if we have an index registry and index scan spec
        if let Some(index_scan) = &self.strategy.index_scan
            && let Some(index_registry) = &self.index_registry
        {
            // Use index scan
            let stream = index_scan_stream(
                self.db.clone(),
                index_registry.clone(),
                index_scan,
                self.projection.clone(),
                self.limit,
            );
            return Ok(Box::pin(stream));
        }

        // Fall back to key-based scan
        let key_range = &self.strategy.key_range;
        let stream = if let Some(exact) = &key_range.exact {
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
        } else if key_range.is_prefix {
            // Prefix scan
            let prefix = key_range.start.as_deref().unwrap_or(&[]);
            prefix_scan_stream(self.db.clone(), prefix, self.projection.clone(), self.limit)
        } else {
            // Range scan or full scan
            let start = key_range.start.as_deref().unwrap_or(&[]);
            let end = key_range.end.as_deref().unwrap_or(&[]);
            if start.is_empty() && end.is_empty() {
                full_scan_stream(self.db.clone(), self.projection.clone(), self.limit)
            } else {
                range_scan_stream(self.db.clone(), start, end, self.projection.clone(), self.limit)
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use datafusion::logical_expr::col;
    use datafusion::prelude::lit;

    use super::*;

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

    #[test]
    fn extract_index_scan_exact() {
        let filters = vec![col("mod_revision").eq(lit(100_i64))];
        let strategy = extract_scan_strategy(&filters);

        // Should produce an index scan
        assert!(strategy.index_scan.is_some());
        let idx = strategy.index_scan.unwrap();
        assert_eq!(idx.index_name, "idx_mod_revision");
        assert!(idx.is_exact);
        assert!(idx.exact_value.is_some());
    }

    #[test]
    fn extract_index_scan_range() {
        let filters = vec![
            col("mod_revision").gt_eq(lit(50_i64)),
            col("mod_revision").lt(lit(100_i64)),
        ];
        let strategy = extract_scan_strategy(&filters);

        // Should produce an index scan with range
        assert!(strategy.index_scan.is_some());
        let idx = strategy.index_scan.unwrap();
        assert_eq!(idx.index_name, "idx_mod_revision");
        assert!(!idx.is_exact);
        assert!(idx.start_value.is_some());
        assert!(idx.end_value.is_some());
    }

    #[test]
    fn extract_no_index_scan_for_value() {
        // value column is not indexed
        let filters = vec![col("value").eq(lit("test"))];
        let strategy = extract_scan_strategy(&filters);

        // Should not produce an index scan
        assert!(strategy.index_scan.is_none());
    }

    #[test]
    fn is_indexed_column_test() {
        assert!(is_indexed_column(&col("mod_revision")));
        assert!(is_indexed_column(&col("create_revision")));
        assert!(is_indexed_column(&col("expires_at_ms")));
        assert!(is_indexed_column(&col("lease_id")));
        assert!(!is_indexed_column(&col("key")));
        assert!(!is_indexed_column(&col("value")));
    }
}
