//! Verus Sync Verification and Coverage Metrics
//!
//! This crate provides tools for validating that production verified functions
//! (`src/verified/*.rs`) match their Verus specifications (`verus/*.rs`).
//!
//! # Features
//!
//! - AST-based parsing via `syn` for accurate function extraction
//! - Configurable normalization with per-crate mapping files
//! - Auto-discovery of critical functions from Verus specs
//! - Structured output (JSON/terminal/GitHub Actions) for CI integration
//! - Coverage tracking and metrics
//! - Watch mode for continuous validation
//! - Severity-based filtering for flexible CI policies

pub mod comparison;
pub mod output;
pub mod parser;
pub mod watch;

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

/// A parsed function from either production or Verus code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFunction {
    /// Function name
    pub name: String,
    /// Source file path
    pub file_path: PathBuf,
    /// Line number where function starts
    pub line_number: u32,
    /// Function signature (parameters and return type)
    pub signature: FunctionSignature,
    /// Normalized function body (if available)
    pub body: Option<String>,
    /// Raw function body before normalization
    pub raw_body: Option<String>,
    /// Function kind (exec, spec, proof, or regular)
    pub kind: FunctionKind,
    /// Whether body comparison should be skipped (e.g., #[verifier(external_body)])
    pub skip_body: bool,
}

/// Function signature information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionSignature {
    /// Parameter names and types
    pub params: Vec<FunctionParam>,
    /// Return type (if any)
    pub return_type: Option<String>,
    /// Whether function is const
    pub is_const: bool,
    /// Whether function is async
    pub is_async: bool,
    /// Generic parameters
    pub generics: Vec<String>,
}

/// A function parameter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionParam {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub ty: String,
}

/// The kind of function (Verus-specific or regular Rust).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FunctionKind {
    /// Regular Rust function
    Regular,
    /// Verus exec function (can be called at runtime)
    Exec,
    /// Verus spec function (specification only)
    Spec,
    /// Verus proof function (proof only)
    Proof,
}

impl FunctionKind {
    /// Returns true if this function can have a production counterpart.
    pub fn is_executable(&self) -> bool {
        matches!(self, Self::Regular | Self::Exec)
    }
}

/// Severity level for comparison results.
///
/// Used to classify issues by their impact and filter output accordingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational - no action required (e.g., MissingVerus)
    Info,
    /// Warning - may indicate an issue but not blocking (e.g., whitespace-only differences)
    Warning,
    /// Error - blocking issue that must be resolved (e.g., BodyDrift, SignatureDrift,
    /// MissingProduction)
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" | "i" => Ok(Self::Info),
            "warning" | "warn" | "w" => Ok(Self::Warning),
            "error" | "err" | "e" => Ok(Self::Error),
            _ => Err(format!("Unknown severity: '{}'. Valid values: info, warning, error", s)),
        }
    }
}

/// Result of comparing two functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonResult {
    /// Functions match
    Match,
    /// Signatures differ
    SignatureDrift {
        production: FunctionSignature,
        verus: FunctionSignature,
    },
    /// Bodies differ
    BodyDrift {
        production_body: String,
        verus_body: String,
        diff: String,
    },
    /// Missing in production
    MissingProduction {
        verus_function: String,
        verus_file: PathBuf,
    },
    /// Missing in Verus
    MissingVerus {
        production_function: String,
        production_file: PathBuf,
    },
    /// Body comparison skipped (external_body)
    SkippedExternalBody,
}

impl ComparisonResult {
    /// Returns true if this is a match or skipped comparison.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Match | Self::SkippedExternalBody)
    }

    /// Returns true if this represents drift.
    pub fn is_drift(&self) -> bool {
        !self.is_ok()
    }

    /// Returns the severity level of this comparison result.
    pub fn severity(&self) -> Severity {
        match self {
            Self::Match | Self::SkippedExternalBody => Severity::Info,
            Self::MissingVerus { .. } => Severity::Info,
            Self::SignatureDrift { .. } | Self::BodyDrift { .. } | Self::MissingProduction { .. } => Severity::Error,
        }
    }

    /// Returns a short description of this result type.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::SkippedExternalBody => "skipped",
            Self::SignatureDrift { .. } => "signature_drift",
            Self::BodyDrift { .. } => "body_drift",
            Self::MissingProduction { .. } => "missing_production",
            Self::MissingVerus { .. } => "missing_verus",
        }
    }
}

/// Configuration for per-crate normalization mappings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrateConfig {
    /// Expression mappings (pattern -> replacement)
    #[serde(default)]
    pub expression_mappings: Vec<ExpressionMapping>,
    /// Semantic equivalences
    #[serde(default)]
    pub equivalences: Vec<Equivalence>,
    /// Functions to always skip
    #[serde(default)]
    pub skip_functions: Vec<String>,
    /// Critical functions that must be present
    #[serde(default)]
    pub critical_functions: Vec<String>,
}

/// An expression mapping for normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionMapping {
    /// Pattern to match
    pub pattern: String,
    /// Replacement string
    pub replacement: String,
}

/// A semantic equivalence rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equivalence {
    /// Pattern to match
    pub pattern: String,
    /// Canonical form
    pub canonical: String,
}

/// Verification report for a single crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateReport {
    /// Crate name
    pub name: String,
    /// Path to the crate
    pub path: PathBuf,
    /// Individual function comparisons
    pub comparisons: Vec<FunctionComparison>,
    /// Number of functions that match
    pub matches: u32,
    /// Number of functions with drift
    pub drifts: u32,
    /// Number of functions missing from production
    pub missing_production: u32,
    /// Number of functions missing from Verus
    pub missing_verus: u32,
    /// Number of skipped comparisons
    pub skipped: u32,
}

impl CrateReport {
    /// Calculate the coverage percentage.
    pub fn coverage_percent(&self) -> f64 {
        let total = self.matches + self.drifts + self.missing_production;
        if total == 0 {
            100.0
        } else {
            (self.matches as f64 / total as f64) * 100.0
        }
    }

    /// Returns true if there are any drift issues.
    pub fn has_drift(&self) -> bool {
        self.drifts > 0 || self.missing_production > 0
    }

    /// Returns true if there are any issues at or above the given severity level.
    pub fn has_issues_at_or_above(&self, threshold: Severity) -> bool {
        self.comparisons.iter().any(|c| c.result.severity() >= threshold)
    }
}

/// A single function comparison result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComparison {
    /// Function name
    pub function_name: String,
    /// Production file (if found)
    pub production_file: Option<PathBuf>,
    /// Verus file (if found)
    pub verus_file: Option<PathBuf>,
    /// Comparison result
    pub result: ComparisonResult,
}

/// Full verification report across all crates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Individual crate reports
    pub crates: Vec<CrateReport>,
    /// Summary statistics
    pub summary: ReportSummary,
}

/// Summary statistics for the report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    /// Total crates checked
    pub crates_checked: u32,
    /// Total functions compared
    pub functions_compared: u32,
    /// Total matches
    pub matches: u32,
    /// Total drifts
    pub drifts: u32,
    /// Total missing from production
    pub missing_production: u32,
    /// Total missing from Verus
    pub missing_verus: u32,
    /// Total skipped
    pub skipped: u32,
    /// Overall coverage percentage
    pub coverage_percent: f64,
}

impl VerificationReport {
    /// Create a new report from crate reports.
    pub fn new(crates: Vec<CrateReport>) -> Self {
        let mut summary = ReportSummary {
            crates_checked: crates.len() as u32,
            functions_compared: 0,
            matches: 0,
            drifts: 0,
            missing_production: 0,
            missing_verus: 0,
            skipped: 0,
            coverage_percent: 0.0,
        };

        for report in &crates {
            summary.functions_compared += report.matches + report.drifts + report.missing_production + report.skipped;
            summary.matches += report.matches;
            summary.drifts += report.drifts;
            summary.missing_production += report.missing_production;
            summary.missing_verus += report.missing_verus;
            summary.skipped += report.skipped;
        }

        let total = summary.matches + summary.drifts + summary.missing_production;
        summary.coverage_percent = if total == 0 {
            100.0
        } else {
            (summary.matches as f64 / total as f64) * 100.0
        };

        Self { crates, summary }
    }

    /// Returns true if there are any drift issues.
    pub fn has_drift(&self) -> bool {
        self.summary.drifts > 0 || self.summary.missing_production > 0
    }

    /// Returns true if there are any issues at or above the given severity level.
    pub fn has_issues_at_or_above(&self, threshold: Severity) -> bool {
        self.crates.iter().any(|c| c.has_issues_at_or_above(threshold))
    }

    /// Get all comparisons filtered by minimum severity.
    pub fn comparisons_at_or_above(
        &self,
        threshold: Severity,
    ) -> impl Iterator<Item = (&CrateReport, &FunctionComparison)> {
        self.crates.iter().flat_map(move |c| {
            c.comparisons
                .iter()
                .filter(move |comp| comp.result.severity() >= threshold)
                .map(move |comp| (c, comp))
        })
    }
}

/// Main verification engine.
pub struct VerificationEngine {
    /// Root directory of the project
    root_dir: PathBuf,
    /// Crates to verify
    crate_names: Vec<String>,
    /// Verbose output
    verbose: bool,
}

impl VerificationEngine {
    /// Create a new verification engine.
    pub fn new(root_dir: PathBuf, crate_names: Vec<String>, verbose: bool) -> Self {
        Self {
            root_dir,
            crate_names,
            verbose,
        }
    }

    /// Run verification and return the report.
    pub fn verify(&self) -> Result<VerificationReport> {
        let mut crate_reports = Vec::new();

        for crate_name in &self.crate_names {
            if let Some(report) = self.verify_crate(crate_name)? {
                crate_reports.push(report);
            }
        }

        Ok(VerificationReport::new(crate_reports))
    }

    /// Verify a single crate.
    fn verify_crate(&self, crate_name: &str) -> Result<Option<CrateReport>> {
        let crate_dir = self.root_dir.join("crates").join(crate_name);
        let verified_dir = crate_dir.join("src/verified");
        let verus_dir = crate_dir.join("verus");

        // Skip if directories don't exist
        if !verified_dir.exists() || !verus_dir.exists() {
            if self.verbose {
                eprintln!("Skipping {}: missing src/verified/ or verus/", crate_name);
            }
            return Ok(None);
        }

        // Load crate config if exists
        let config = self.load_crate_config(&crate_dir)?;

        // Parse production functions
        let production_fns = parser::production::parse_verified_dir(&verified_dir)?;

        // Parse Verus functions
        let verus_fns = parser::verus::parse_verus_dir(&verus_dir)?;

        // Compare functions
        let comparisons = comparison::matcher::compare_functions(&production_fns, &verus_fns, &config);

        // Build report
        let mut report = CrateReport {
            name: crate_name.to_string(),
            path: crate_dir,
            comparisons: Vec::new(),
            matches: 0,
            drifts: 0,
            missing_production: 0,
            missing_verus: 0,
            skipped: 0,
        };

        for (name, result) in comparisons {
            let prod_file = production_fns.iter().find(|f| f.name == name).map(|f| f.file_path.clone());
            let verus_file = verus_fns.iter().find(|f| f.name == name).map(|f| f.file_path.clone());

            match &result {
                ComparisonResult::Match => report.matches += 1,
                ComparisonResult::SignatureDrift { .. } | ComparisonResult::BodyDrift { .. } => report.drifts += 1,
                ComparisonResult::MissingProduction { .. } => report.missing_production += 1,
                ComparisonResult::MissingVerus { .. } => report.missing_verus += 1,
                ComparisonResult::SkippedExternalBody => report.skipped += 1,
            }

            report.comparisons.push(FunctionComparison {
                function_name: name,
                production_file: prod_file,
                verus_file,
                result,
            });
        }

        Ok(Some(report))
    }

    /// Load crate-specific configuration.
    fn load_crate_config(&self, crate_dir: &Path) -> Result<CrateConfig> {
        let config_path = crate_dir.join(".verus-sync-config.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?;
            toml::from_str(&content).with_context(|| format!("failed to parse {}", config_path.display()))
        } else {
            Ok(CrateConfig::default())
        }
    }
}

/// Default verified crates to check.
pub const DEFAULT_VERIFIED_CRATES: &[&str] = &[
    "aspen-coordination",
    "aspen-raft",
    "aspen-core",
    "aspen-transport",
    "aspen-cluster",
];
