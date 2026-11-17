//! Bounded CPU batch evaluation of independent rows (issue #41).
//!
//! The reference model is `proofs/foundation/NobleFoundation/BatchRows.lean`: a row is one wrapping
//! `I64` operation over two explicit operands, and a batch is the *ordered map* of independent
//! single-row runs, so partial progress is the ordered map over the prefix and a refused row can
//! never change another row's result.
//!
//! This module is the tested implementation of that shape, not a proof of it. Rows delegate to the
//! same `sem::i64_*` reference operations the single-row paths use, so a row's result is the
//! documented K-NUM-01 wrapping result rather than a second evaluator. The model's literal-range
//! refusals cannot occur in `i64`, which is why `evaluate` is total; the refusal rows of the model
//! are therefore **not** exercised here and no correspondence claim covers them.
//!
//! [`Prepared`] is the graph-backed entry point: one checked `I64` graph compiled once, then run
//! over ordered input rows through the existing packed interpreter
//! ([`crate::compiled::Compiled::evaluate_in`]) with one reused register workspace. Each row's
//! report is the report of an independent single-row run, and limits are checked before any row
//! runs. It adds no arithmetic of its own, and its rows are compared against independent runs by
//! tests, not proved against `BatchRows.lean`.
//!
//! **Parameters do not apply to `I64` graphs.** The checker refuses a param slot in any profile
//! other than `ngraph-f32/0` (see `check.rs`: `F32 slot in ngraph-i64/0`), so an `I64` graph has no
//! parameter slots, [`Prepared::run`] takes no snapshot and a batch has no parameter state to bind.
//! A graph outside the `I64` whitelist is refused outright by [`Prepared::new`].
//!
//! **Interface shapes are metadata, not row layout.** A row is the graph's *flattened* input-slot
//! values in slot order. The graph's `input_shape`/`output_shape` trees record how interface items
//! nest pairs; they change neither the row width, nor the result association, nor the order the
//! checker already assigns to slots and outputs.

/// One batch row: one wrapping `I64` operation over two explicit operands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// The operation, applied to both operands in the documented order.
    pub op: RowOp,
    /// The deeper operand (the left operand of the machine's stack order).
    pub deeper: i64,
    /// The top operand.
    pub top: i64,
}

/// The wrapping operations a batch row admits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowOp {
    /// Wrapping `a + b`.
    Add,
    /// Wrapping `a - b`.
    Sub,
    /// Wrapping `a * b`.
    Mul,
}

impl Row {
    /// One independent row run.
    #[must_use]
    pub fn run(self) -> i64 {
        match self.op {
            RowOp::Add => crate::sem::i64_add(self.deeper, self.top),
            RowOp::Sub => crate::sem::i64_sub(self.deeper, self.top),
            RowOp::Mul => crate::sem::i64_mul(self.deeper, self.top),
        }
    }
}

/// The batch semantics: the ordered map of independent row runs.
#[must_use]
pub fn evaluate(rows: &[Row]) -> alloc::vec::Vec<i64> {
    rows.iter().copied().map(Row::run).collect()
}

/// The batch semantics with a caller-supplied output buffer, so a repeated batch does not allocate
/// per call. The buffer keeps its capacity and its previous contents are overwritten in order.
#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; evaluate_into clears, resizes, and reuses caller-owned Vec storage; a slice cannot preserve the variable-length output and retained-capacity contract."
)]
pub fn evaluate_into(rows: &[Row], out: &mut alloc::vec::Vec<i64>) {
    out.clear();
    out.extend(rows.iter().copied().map(Row::run));
}

/// The declared node work of one batch: one operation per row, counted separately from the
/// staging/output writes, which are also one per row. Both counters are exact functions of the row
/// count, so a cost claim can name them instead of elapsed time.
#[must_use]
pub fn declared_row_work(row_count: usize) -> (usize, usize) {
    (row_count, row_count)
}

/// Caller limits for one [`Prepared::run`]. The row and output limits are checked before any row
/// runs and before any report storage is allocated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Most rows one run may hold.
    pub max_rows: usize,
    /// Most output values one run may produce: its rows times the graph's outputs.
    pub max_output_values: usize,
    /// Node evaluations one row may use. Running out is that row's report, and later rows still
    /// run.
    pub row_fuel: Option<u64>,
    /// Node evaluations the whole run may use, counted as the compiled evaluator counts them.
    /// Running out stops the run inside the row that needed more. That evaluator counts a row's
    /// constant and slot loads before it charges the row's first layer, so the last row can count
    /// up to its constants and slots past this fuel.
    pub run_fuel: Option<u64>,
}

/// Why a graph was not prepared, or why a run was refused before any row ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The graph is not an `I64` sub-profile graph. Only `I64` arithmetic and its `Bool`
    /// comparisons are admitted for batching.
    Profile,
    /// The inputs are not `rows` rows of `width` values each.
    Shape {
        /// Declared rows.
        rows: usize,
        /// Input values per row: the graph's input slots.
        width: usize,
        /// Input values supplied.
        values: usize,
    },
    /// More rows than [`Limits::max_rows`].
    Rows {
        /// Declared rows.
        rows: usize,
        /// The row limit.
        max_rows: usize,
    },
    /// More output values than [`Limits::max_output_values`], including a count `usize` cannot
    /// hold.
    Outputs {
        /// Declared rows.
        rows: usize,
        /// Output values per row.
        per_row: usize,
        /// The output limit.
        max_output_values: usize,
    },
}

/// Why the rows after the reported ones never ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// The run was refused before any row ran.
    Refused(Refusal),
    /// The row checkpoint cancelled before row `next` started.
    Cancelled {
        /// The first row that never ran.
        next: usize,
    },
    /// The run fuel ran out inside row `row`, whose report is the last one.
    Exhausted {
        /// The row that ran out.
        row: usize,
    },
}

/// One run: the reports of the rows that started, in row order, and why the rest did not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// One report per started row, for rows `0..reports.len()`.
    pub reports: alloc::vec::Vec<crate::eval::Report>,
    /// `Some` exactly when the rows from `reports.len()` onward never ran.
    pub stop: Option<Stop>,
}

/// One checked `I64` graph prepared for repeated batch runs: its compiled form, its input width
/// and one register workspace that every row of every run reuses.
pub struct Prepared<'c> {
    compiled: &'c crate::compiled::Compiled,
    workspace: crate::compiled::Workspace,
    width: usize,
    outputs: usize,
}

impl<'c> Prepared<'c> {
    /// Prepares `compiled` for batch runs.
    ///
    /// # Errors
    /// [`Refusal::Profile`] unless the graph is an `I64` sub-profile graph.
    pub fn new(compiled: &'c crate::compiled::Compiled) -> Result<Self, Refusal> {
        let graph = compiled.checked().graph();
        if graph.profile != crate::ir::SubProfile::I64 {
            return Err(Refusal::Profile);
        }
        let width = graph
            .slots
            .iter()
            .filter(|slot| slot.kind == crate::ir::SlotKind::Input)
            .count();
        Ok(Self {
            compiled,
            workspace: compiled.workspace(),
            width,
            outputs: graph.outputs.len(),
        })
    }

    /// Runs `rows` rows read in order from `inputs`, each the graph's input width.
    ///
    /// Each row's report is the report of an independent single-row run on the same inputs under
    /// the row fuel; the one exception is the row in which the run fuel runs out, which is the
    /// last report. A refused row, a failed guard and a row that exhausts its row fuel are that
    /// row's report, and later rows still run. The run is refused before any row runs, and before
    /// any report storage is allocated, when the input shape does not match or a row or output
    /// limit is exceeded. `cancel_row` is asked before each row starts, so a row is never cut
    /// short by it. Every row reuses one register workspace: no instruction writes a constant
    /// register, and every other register is written before it is read, so no value from an
    /// earlier row reaches a later one.
    #[must_use]
    pub fn run(
        &mut self,
        rows: usize,
        inputs: &[crate::sem::Value],
        limits: Limits,
        cancel_row: Option<&dyn Fn(usize) -> bool>,
    ) -> Outcome {
        if let Err(refusal) = self.admit(rows, inputs.len(), limits) {
            return Outcome {
                reports: alloc::vec::Vec::new(),
                stop: Some(Stop::Refused(refusal)),
            };
        }
        let mut reports = alloc::vec::Vec::with_capacity(rows);
        let mut remaining = limits.run_fuel;
        let mut rest = inputs;
        for row in 0..rows {
            if cancel_row.is_some_and(|cancel| cancel(row)) {
                return Outcome {
                    reports,
                    stop: Some(Stop::Cancelled { next: row }),
                };
            }
            let (row_inputs, tail) = rest.split_at(self.width);
            rest = tail;
            let (fuel, is_run_bound) = row_budget(limits.row_fuel, remaining);
            let mut control = crate::eval::Control {
                fuel,
                cancel: None,
                scratch: None,
                forward_scratch: None,
                evaluated_nodes: None,
            };
            let report =
                self.compiled
                    .evaluate_in(&mut self.workspace, row_inputs, None, &mut control);
            remaining = remaining.map(|left| left.saturating_sub(report.evaluated_nodes));
            let is_exhausted =
                report.obs == crate::eval::Obs::Exhausted(crate::diag::BudgetKind::Fuel);
            reports.push(report);
            if is_exhausted && is_run_bound {
                return Outcome {
                    reports,
                    stop: Some(Stop::Exhausted { row }),
                };
            }
        }
        Outcome {
            reports,
            stop: None,
        }
    }

    /// The checks a run passes before any row runs or any report storage is allocated.
    fn admit(&self, rows: usize, values: usize, limits: Limits) -> Result<(), Refusal> {
        if rows.checked_mul(self.width) != Some(values) {
            return Err(Refusal::Shape {
                rows,
                width: self.width,
                values,
            });
        }
        if rows > limits.max_rows {
            return Err(Refusal::Rows {
                rows,
                max_rows: limits.max_rows,
            });
        }
        if rows
            .checked_mul(self.outputs)
            .is_none_or(|total| total > limits.max_output_values)
        {
            return Err(Refusal::Outputs {
                rows,
                per_row: self.outputs,
                max_output_values: limits.max_output_values,
            });
        }
        Ok(())
    }
}

/// One row's fuel and whether the run's remaining fuel, rather than the row fuel, is the binding
/// bound. A tie goes to the row fuel, so a row that exhausts it reports its own exhaustion.
fn row_budget(row_fuel: Option<u64>, remaining: Option<u64>) -> (Option<u64>, bool) {
    match (row_fuel, remaining) {
        (row_fuel, None) => (row_fuel, false),
        (None, Some(left)) => (Some(left), true),
        (Some(row_fuel), Some(left)) if left < row_fuel => (Some(left), true),
        (Some(row_fuel), Some(_left)) => (Some(row_fuel), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_rows_overwrite_the_reused_output() {
        let rows = [
            Row {
                op: RowOp::Add,
                deeper: 2,
                top: 3,
            },
            Row {
                op: RowOp::Sub,
                deeper: 2,
                top: 3,
            },
            Row {
                op: RowOp::Mul,
                deeper: -4,
                top: 5,
            },
        ];
        assert_eq!(evaluate(&rows), alloc::vec![5, -1, -20]);
        // A warm output buffer is overwritten in order and does not grow on the same row count,
        // which is the reuse the batch entry point exists for.
        let mut out = alloc::vec::Vec::with_capacity(rows.len());
        evaluate_into(&rows, &mut out);
        assert_eq!(out, alloc::vec![5, -1, -20]);
        #[expect(
            tigerstyle::numeric_units,
            reason = "Owner: noble-maintainers; capacity_rows is Vec capacity in rows, not bytes or time; its explicit row unit is outside this lint's suffix vocabulary."
        )]
        let capacity_rows = out.capacity();
        evaluate_into(&rows, &mut out);
        assert_eq!(out, alloc::vec![5, -1, -20]);
        assert_eq!(out.capacity(), capacity_rows);
    }

    #[test]
    fn wrapping_boundaries_are_the_documented_results() {
        let rows = [
            Row {
                op: RowOp::Add,
                deeper: i64::MAX,
                top: 1,
            },
            Row {
                op: RowOp::Sub,
                deeper: i64::MIN,
                top: 1,
            },
            Row {
                op: RowOp::Mul,
                deeper: i64::MAX,
                top: 2,
            },
        ];
        assert_eq!(evaluate(&rows), alloc::vec![i64::MIN, i64::MAX, -2]);
    }

    #[test]
    fn empty_input_and_prefixes_preserve_row_order() {
        assert!(evaluate(&[]).is_empty());
        let rows = [
            Row {
                op: RowOp::Mul,
                deeper: 6,
                top: 7,
            },
            Row {
                op: RowOp::Add,
                deeper: 1,
                top: 2,
            },
        ];
        assert_eq!(&evaluate(&rows)[..1], &evaluate(&rows[..1])[..]);
    }
}
