mod counts;
mod runs;
mod windows;

use thiserror::Error;

use crate::{
    input::scalar::Ratio,
    issue::{Issue, IssueCode, IssuePath},
    mapping::TargetCount,
    scenario::WindowSize,
};

pub use counts::{MaximumLoad, TargetMetrics};
pub use runs::LongestRun;
pub use windows::WindowMetrics;

/// A failure in a metrics computation approved by Scenario preflight.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum MetricsError {
    /// An impossible target, allocation failure, or checked arithmetic failure occurred.
    #[error("analysis could not be completed")]
    AnalysisFailed,
}

impl MetricsError {
    /// Converts an internal metrics failure into the stable command diagnostic.
    pub fn issue(self) -> Issue {
        match self {
            Self::AnalysisFailed => Issue::new(
                IssueCode::AnalysisFailed,
                IssuePath::root(),
                "analysis could not be completed",
            ),
        }
    }
}

/// Exact metric rows for one concrete Target sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestMetrics {
    targets: Vec<TargetMetrics>,
    max_load: MaximumLoad,
    windows: Vec<WindowMetrics>,
    longest_run: LongestRun,
    window_target_visits: u128,
}

impl TestMetrics {
    /// Returns all Targets in ascending ID order, including zero-count Targets.
    pub fn targets(&self) -> &[TargetMetrics] {
        &self.targets
    }

    /// Returns the busiest Target, using the smallest Target ID on ties.
    pub const fn max_load(&self) -> &MaximumLoad {
        &self.max_load
    }

    /// Returns short-term metrics in declared window order.
    pub fn windows(&self) -> &[WindowMetrics] {
        &self.windows
    }

    /// Returns the earliest longest consecutive Target run.
    pub const fn longest_run(&self) -> &LongestRun {
        &self.longest_run
    }

    /// Returns exact stimulus entries visited across all window analyses.
    pub const fn window_target_visits(&self) -> u128 {
        self.window_target_visits
    }
}

/// Computes exact metrics over one borrowed, ordered Target sequence.
pub fn analyze_targets(
    targets: &[u16],
    target_count: TargetCount,
    window_sizes: &[WindowSize],
) -> Result<TestMetrics, MetricsError> {
    let (target_rows, max_load) = counts::analyze_counts(targets, target_count)?;
    let window_analysis = windows::analyze_windows(targets, target_count, window_sizes)?;
    let longest_run = runs::analyze_runs(targets)?;
    Ok(TestMetrics {
        targets: target_rows,
        max_load,
        windows: window_analysis.rows,
        longest_run,
        window_target_visits: window_analysis.target_visits,
    })
}

fn exact_ratio(numerator: u128, denominator: u128) -> Result<Ratio, MetricsError> {
    Ratio::new(numerator, denominator).map_err(|_| MetricsError::AnalysisFailed)
}
