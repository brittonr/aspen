# Deterministic Simulation Testing (DST) Coverage Matrix Best Practices

Based on research of FoundationDB, RisingWave, TigerBeetle, and industry standards.

## Key Dimensions to Track

### 1. **Cluster Topology Dimensions**

- **Node Count**: 1, 3, 5, 7+ nodes (odd numbers for quorum)
- **Node Roles**: Leader, follower, learner, candidate
- **Data Distribution**: Single region, multi-region, cross-datacenter
- **Client Count**: 1, 10, 100, 1000+ concurrent clients

### 2. **Failure Mode Dimensions**

- **Network Faults**:
  - Partition types: Complete isolation, asymmetric partitions, partial connectivity
  - Packet loss: 0%, 1%, 5%, 10%, 50%
  - Latency injection: Normal, high (100ms+), variable jitter
  - Bandwidth constraints: Unlimited, limited, degraded
  - Connection failures: TCP resets, timeouts, refused connections

- **Process Failures**:
  - Clean shutdown vs crash
  - Single node vs cascading failures
  - Leader failure vs follower failure
  - Recovery timing: Immediate, delayed, never

- **Storage Faults**:
  - Disk I/O errors: Read failures, write failures, corruption
  - Disk performance: Normal, degraded, stalled
  - Space exhaustion scenarios
  - WAL corruption, snapshot corruption

- **Time-based Faults**:
  - Clock skew between nodes
  - Timer precision issues
  - Message reordering
  - Delayed message delivery

### 3. **Workload Dimensions**

- **Operation Types**: Read, write, scan, delete, batch operations
- **Operation Size**: Small (< 1KB), medium (1KB-1MB), large (> 1MB)
- **Operation Pattern**: Sequential, random, hot spots
- **Consistency Level**: Eventual, linearizable, serializable
- **Transaction Complexity**: Single-key, multi-key, cross-shard

### 4. **Protocol/Algorithm Coverage**

- **Consensus States**: Election, replication, reconfiguration, snapshot
- **Membership Changes**: Adding nodes, removing nodes, replacing nodes
- **Recovery Scenarios**: Cold start, warm restart, data migration
- **Edge Cases**: Split-brain, zombie leaders, conflicting terms

### 5. **Performance/Scale Dimensions**

- **Duration**: Short (seconds), medium (minutes), long (hours/days simulated)
- **Event Rate**: Low, normal, stress, overload
- **State Size**: Empty, small, large, near-limits
- **Concurrent Operations**: 1, 10, 100, 1000, max_allowed

## Best Format for Coverage Matrix

### Recommended: Hybrid JSON + Markdown Approach

#### 1. **Machine-Readable JSON Format** (`coverage-matrix.json`)

```json
{
  "version": "1.0.0",
  "last_updated": "2025-12-14T00:00:00Z",
  "total_scenarios": 256,
  "coverage_percentage": 78.5,
  "dimensions": {
    "cluster": {
      "node_counts": [1, 3, 5, 7],
      "coverage": "4/4"
    },
    "failures": {
      "network": {
        "partition": ["complete", "asymmetric", "partial"],
        "packet_loss": [0, 1, 5, 10, 50],
        "coverage": "12/15"
      },
      "process": {
        "crash_types": ["leader", "follower", "multiple"],
        "coverage": "3/3"
      }
    }
  },
  "test_scenarios": [
    {
      "id": "DST-001",
      "name": "3-node-partition-recovery",
      "dimensions": {
        "nodes": 3,
        "failure": "network_partition",
        "workload": "mixed_rw",
        "duration_simulated": "1h"
      },
      "status": "passed",
      "last_run": "2025-12-14T10:00:00Z",
      "seed": "0xDEADBEEF",
      "coverage_contribution": 2.5
    }
  ],
  "gaps": [
    {
      "dimension": "network.packet_loss",
      "missing": "25% packet loss scenario",
      "priority": "medium"
    }
  ]
}
```

#### 2. **Human-Readable Markdown Dashboard** (`COVERAGE_MATRIX.md`)

```markdown
# DST Coverage Matrix

Last Updated: 2025-12-14 | Coverage: **78.5%** | Scenarios: **200/256**

## Coverage Summary

| Dimension | Coverage | Status |
|-----------|----------|--------|
| Node Counts (1,3,5,7) | 4/4 | ✅ |
| Network Partitions | 12/15 | ⚠️ |
| Process Failures | 3/3 | ✅ |
| Storage Faults | 8/10 | ⚠️ |
| Workload Types | 5/5 | ✅ |

## Detailed Coverage Grid

### Network Fault Coverage
| Fault Type | 0% | 1% | 5% | 10% | 50% |
|------------|----|----|----|----|-----|
| Packet Loss | ✅ | ✅ | ✅ | ✅ | ❌ |
| Latency | ✅ | ✅ | ⚠️ | ❌ | ❌ |
| Partition | ✅ | N/A | N/A | N/A | ✅ |

Legend: ✅ Tested | ⚠️ Partial | ❌ Not Tested | N/A Not Applicable

## High-Priority Gaps
1. **25% packet loss scenarios** - Missing medium degradation testing
2. **Asymmetric partitions with 5+ nodes** - Complex topology untested
3. **Leader failure during reconfiguration** - Edge case not covered
```

#### 3. **Test Execution Manifest** (`test-manifest.toml`)

```toml
[coverage]
version = "1.0.0"
total_dimensions = 5
total_scenarios = 256

[[scenarios]]
id = "DST-001"
name = "3-node-partition-recovery"
tags = ["network", "partition", "recovery"]
weight = 2.5  # Coverage contribution

[scenarios.config]
nodes = 3
failure_mode = "network_partition"
duration = "1h"
seed = "0xDEADBEEF"

[scenarios.assertions]
safety = ["linearizability", "no_data_loss"]
liveness = ["recovery_time < 30s"]
```

## How to Identify Coverage Gaps

### 1. **Automated Gap Detection**

```rust
// Example gap detection algorithm
fn identify_gaps(matrix: &CoverageMatrix) -> Vec<Gap> {
    let mut gaps = Vec::new();

    // Check dimension combinations
    for nodes in [1, 3, 5, 7] {
        for failure in all_failure_modes() {
            for workload in all_workloads() {
                if !matrix.has_test(nodes, failure, workload) {
                    gaps.push(Gap {
                        dimension: format!("{}-node-{}-{}", nodes, failure, workload),
                        priority: calculate_priority(nodes, failure, workload),
                    });
                }
            }
        }
    }

    // Check edge cases
    if !matrix.has_test("leader_failure_during_reconfiguration") {
        gaps.push(Gap::high_priority("Edge case: leader failure during reconfig"));
    }

    gaps.sort_by_key(|g| g.priority);
    gaps
}
```

### 2. **Priority Scoring for Gaps**

- **Critical (P0)**: Safety violations possible, data loss scenarios
- **High (P1)**: Liveness issues, common production scenarios
- **Medium (P2)**: Performance degradation, less common scenarios
- **Low (P3)**: Edge cases, theoretical scenarios

### 3. **Coverage Metrics**

- **Dimension Coverage**: % of each dimension tested
- **Combination Coverage**: % of valid dimension combinations tested
- **Weighted Coverage**: Based on scenario importance/frequency
- **Time Coverage**: Simulated time vs real-world equivalence

## CI Integration Patterns

### 1. **GitHub Actions Workflow**

```yaml
name: DST Coverage Matrix

on:
  push:
    branches: [main]
  pull_request:
  schedule:
    - cron: '0 0 * * *'  # Nightly full matrix run

jobs:
  coverage-matrix:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        include:
          - scenario: "3-node-partition"
            nodes: 3
            failure: "partition"
          - scenario: "5-node-cascade"
            nodes: 5
            failure: "cascade"

    steps:
      - uses: actions/checkout@v3

      - name: Run DST Scenario
        run: |
          cargo test --test madsim_matrix \
            --scenario ${{ matrix.scenario }} \
            -- --seed ${{ github.run_number }}

      - name: Update Coverage Matrix
        run: |
          cargo run --bin update-coverage-matrix \
            --scenario ${{ matrix.scenario }} \
            --result ${{ steps.test.outcome }}

      - name: Generate Coverage Report
        run: |
          cargo run --bin coverage-report \
            --format json > coverage.json
          cargo run --bin coverage-report \
            --format markdown > COVERAGE_MATRIX.md

      - name: Comment PR with Coverage
        if: github.event_name == 'pull_request'
        uses: actions/github-script@v6
        with:
          script: |
            const coverage = require('./coverage.json');
            const comment = `## DST Coverage Impact

            Overall Coverage: **${coverage.percentage}%** (${coverage.delta > 0 ? '+' : ''}${coverage.delta}%)
            New Scenarios: ${coverage.new_scenarios}
            Gaps Closed: ${coverage.gaps_closed}`;

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: comment
            });
```

### 2. **Continuous Coverage Tracking**

```rust
// src/testing/coverage_tracker.rs
pub struct CoverageTracker {
    matrix: CoverageMatrix,
    history: Vec<CoverageSnapshot>,
}

impl CoverageTracker {
    pub fn record_test_run(&mut self, scenario: TestScenario, result: TestResult) {
        self.matrix.mark_tested(scenario.dimensions());

        if result.failed() {
            self.matrix.mark_failure_found(scenario.dimensions(), result.seed);
        }

        self.update_metrics();
        self.save_to_disk();
    }

    pub fn generate_report(&self, format: ReportFormat) -> String {
        match format {
            ReportFormat::Json => self.to_json(),
            ReportFormat::Markdown => self.to_markdown(),
            ReportFormat::Html => self.to_html_dashboard(),
        }
    }

    pub fn identify_next_scenarios(&self, count: usize) -> Vec<TestScenario> {
        // Smart selection of next scenarios to maximize coverage
        self.matrix.get_gaps()
            .into_iter()
            .take(count)
            .map(|gap| gap.to_scenario())
            .collect()
    }
}
```

### 3. **Dashboard Integration**

- **Grafana**: Export metrics to Prometheus for time-series coverage tracking
- **GitHub Pages**: Auto-deploy HTML coverage dashboard on main branch
- **Slack/Discord**: Alert on coverage regression or new failures found
- **Badge Generation**: Create coverage badges for README

### 4. **Nightly/Weekly Deep Testing**

```yaml
# .github/workflows/deep-coverage.yml
on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday

jobs:
  deep-matrix:
    strategy:
      matrix:
        nodes: [3, 5, 7, 9]
        failure_rate: [0, 1, 5, 10, 25, 50]
        duration: ['1h', '6h', '24h']  # Simulated time

    runs-on: ubuntu-latest
    timeout-minutes: 180  # 3 hours real time

    steps:
      - name: Run Extended DST
        run: |
          cargo test --test madsim_deep \
            --nodes ${{ matrix.nodes }} \
            --failure-rate ${{ matrix.failure_rate }} \
            --duration ${{ matrix.duration }} \
            --features extended-coverage
```

## Best Practices Summary

1. **Track Multiple Formats**: Use JSON for machines, Markdown for humans, TOML for configuration
2. **Automate Gap Detection**: Don't rely on manual tracking - use code to find gaps
3. **Prioritize by Risk**: Focus on scenarios that could cause data loss or extended downtime
4. **Version Your Matrix**: Track how coverage evolves over time
5. **Integrate with CI**: Make coverage visible on every PR
6. **Use Deterministic Seeds**: Enable reproduction of any failure
7. **Time-Dilate Strategically**: Balance thorough testing with CI time limits
8. **Document Failures**: When DST finds bugs, add them to regression suite
9. **Cross-Reference Real Incidents**: Map production issues back to coverage gaps
10. **Regular Reviews**: Weekly/monthly coverage review meetings

## Tools and Libraries

- **madsim**: Deterministic simulation for Rust async code
- **proptest**: Property-based testing to generate scenarios
- **criterion**: Benchmark and track performance metrics
- **tracing**: Structured logging for failure analysis
- **serde_json**: Coverage matrix serialization
- **comrak**: Markdown generation for reports
- **prometheus**: Metrics export for dashboards
