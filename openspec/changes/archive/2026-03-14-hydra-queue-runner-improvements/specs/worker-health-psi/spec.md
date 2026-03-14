## ADDED Requirements

### Requirement: Workers report PSI pressure metrics

Workers SHALL collect Linux PSI (Pressure Stall Information) metrics from `/proc/pressure/{cpu,memory,io}` and include them in heartbeat updates. Each metric SHALL report the `avg10` value (10-second moving average).

#### Scenario: Worker on Linux with PSI support

- **WHEN** a worker sends a heartbeat on a Linux kernel ≥ 4.20
- **THEN** the heartbeat SHALL include `cpu_pressure_avg10`, `memory_pressure_avg10`, and `io_pressure_avg10` values parsed from `/proc/pressure/`

#### Scenario: Worker on system without PSI support

- **WHEN** a worker sends a heartbeat on a system where `/proc/pressure/` does not exist
- **THEN** all PSI values SHALL default to `0.0`
- **AND** the scheduler SHALL fall back to load-based capacity checks

### Requirement: Workers report disk free percentages

Workers SHALL report free disk space as a percentage for both the build working directory and the nix store directory in their heartbeat updates.

#### Scenario: Worker reports disk space

- **WHEN** a worker sends a heartbeat
- **THEN** the heartbeat SHALL include `disk_free_build_pct` and `disk_free_store_pct` as f64 values between 0.0 and 100.0

#### Scenario: Disk path does not exist

- **WHEN** a worker cannot stat the build directory or store directory
- **THEN** the corresponding disk free value SHALL default to `0.0`
- **AND** the worker SHALL log a warning

### Requirement: Scheduler rejects workers under pressure

The scheduler SHALL check PSI and disk-free metrics against configurable thresholds before dispatching a job to a worker. A worker that exceeds any threshold SHALL be treated as unavailable for scheduling, regardless of its job slot count.

#### Scenario: Worker exceeds CPU pressure threshold

- **WHEN** a worker's `cpu_pressure_avg10` exceeds the configured CPU threshold (default: 75.0)
- **THEN** the scheduler SHALL NOT dispatch new jobs to that worker
- **AND** existing jobs on the worker SHALL NOT be affected

#### Scenario: Worker exceeds memory pressure threshold

- **WHEN** a worker's `memory_pressure_avg10` exceeds the configured memory threshold (default: 50.0)
- **THEN** the scheduler SHALL NOT dispatch new jobs to that worker

#### Scenario: Worker exceeds I/O pressure threshold

- **WHEN** a worker's `io_pressure_avg10` exceeds the configured I/O threshold (default: 80.0)
- **THEN** the scheduler SHALL NOT dispatch new jobs to that worker

#### Scenario: Worker below disk free threshold

- **WHEN** a worker's `disk_free_build_pct` or `disk_free_store_pct` falls below the configured threshold (default: 5.0%)
- **THEN** the scheduler SHALL NOT dispatch new jobs to that worker

#### Scenario: All thresholds within limits

- **WHEN** a worker's PSI values and disk free percentages are all within configured thresholds
- **AND** the worker has available job slots
- **THEN** the scheduler SHALL consider the worker eligible for dispatch

#### Scenario: Pressure check as verified function

- **WHEN** the scheduler evaluates worker capacity
- **THEN** the pressure threshold check SHALL be implemented as a pure function in `src/verified/` with no I/O or async dependencies
