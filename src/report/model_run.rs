use serde::Serialize;

use crate::{
    input::scalar::Ratio,
    mapping::{MappingModel, MappingValidation},
    metrics::TestMetrics,
    scenario::ConcreteTestDescriptor,
};

use super::ReportModelError;

const MAX_SAFE_INTEGER: u128 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(super) struct SafeInteger(u64);

impl SafeInteger {
    fn new(value: u128) -> Result<Self, ReportModelError> {
        if value > MAX_SAFE_INTEGER {
            return Err(ReportModelError::UnsafeInteger);
        }
        u64::try_from(value)
            .map(Self)
            .map_err(|_| ReportModelError::UnsafeInteger)
    }

    pub(super) const fn get(self) -> u64 {
        self.0
    }
}

/// Exact unreduced ratio and its deterministic six-place display value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExactRatio {
    pub(super) numerator: SafeInteger,
    pub(super) denominator: SafeInteger,
    pub(super) decimal: String,
}

impl ExactRatio {
    fn new(ratio: Ratio) -> Result<Self, ReportModelError> {
        Ok(Self {
            numerator: SafeInteger::new(ratio.numerator())?,
            denominator: SafeInteger::new(ratio.denominator())?,
            decimal: ratio
                .decimal_six()
                .map_err(|_| ReportModelError::InvalidRatio)?,
        })
    }
}

/// One Target's count and exact traffic share.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetResult {
    pub(super) target: u16,
    pub(super) count: SafeInteger,
    pub(super) share: ExactRatio,
}

/// The representative maximum long-term load.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MaximumLoadResult {
    pub(super) target: u16,
    pub(super) count: SafeInteger,
    pub(super) ratio: ExactRatio,
}

/// Worst short-term load for one declared window size.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WindowResult {
    pub(super) size: SafeInteger,
    pub(super) target: u16,
    pub(super) start_index: SafeInteger,
    pub(super) count: SafeInteger,
    pub(super) ratio: ExactRatio,
}

/// Earliest longest run of one Target.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct LongestRunResult {
    pub(super) length: SafeInteger,
    pub(super) target: u16,
    pub(super) start_index: SafeInteger,
}

/// Complete metrics for one concrete Scenario test.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RunCaseResult {
    pub(super) case_id: String,
    pub(super) source_case: String,
    pub(super) accesses: SafeInteger,
    pub(super) targets: Vec<TargetResult>,
    pub(super) max_load: MaximumLoadResult,
    pub(super) windows: Vec<WindowResult>,
    pub(super) longest_run: LongestRunResult,
}

impl RunCaseResult {
    /// Converts one checked descriptor and its exact metrics into report rows.
    ///
    /// # Errors
    /// Returns an error if a value exceeds the JSON safe-integer contract or a
    /// ratio cannot be formatted exactly.
    pub fn from_metrics(
        descriptor: &ConcreteTestDescriptor,
        metrics: &TestMetrics,
    ) -> Result<Self, ReportModelError> {
        let targets = metrics
            .targets()
            .iter()
            .map(|row| {
                Ok(TargetResult {
                    target: row.target(),
                    count: SafeInteger::new(u128::from(row.count()))?,
                    share: ExactRatio::new(row.share())?,
                })
            })
            .collect::<Result<Vec<_>, ReportModelError>>()?;
        let max_load = metrics.max_load();
        let windows = metrics
            .windows()
            .iter()
            .map(|row| {
                Ok(WindowResult {
                    size: SafeInteger::new(u128::from(row.size()))?,
                    target: row.target(),
                    start_index: SafeInteger::new(u128::from(row.start_index()))?,
                    count: SafeInteger::new(u128::from(row.count()))?,
                    ratio: ExactRatio::new(row.ratio())?,
                })
            })
            .collect::<Result<Vec<_>, ReportModelError>>()?;
        let longest = metrics.longest_run();
        Ok(Self {
            case_id: descriptor.case_id().to_owned(),
            source_case: descriptor.source_case().to_owned(),
            accesses: SafeInteger::new(u128::from(descriptor.accesses()))?,
            targets,
            max_load: MaximumLoadResult {
                target: max_load.target(),
                count: SafeInteger::new(u128::from(max_load.count()))?,
                ratio: ExactRatio::new(max_load.ratio())?,
            },
            windows,
            longest_run: LongestRunResult {
                length: SafeInteger::new(u128::from(longest.length()))?,
                target: longest.target(),
                start_index: SafeInteger::new(u128::from(longest.start_index()))?,
            },
        })
    }
}

/// Complete run-command result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RunResult {
    pub(super) mapping_name: String,
    pub(super) mapping_classification: crate::mapping::MappingClassification,
    pub(super) cases: Vec<RunCaseResult>,
    #[serde(skip)]
    pub(super) input: Option<String>,
}

impl RunResult {
    pub(super) fn new(
        mapping: &MappingModel,
        validation: &MappingValidation,
        cases: Vec<RunCaseResult>,
        input: Option<&str>,
    ) -> Self {
        Self {
            mapping_name: mapping.name().as_str().to_owned(),
            mapping_classification: validation.classification(),
            cases,
            input: input.map(str::to_owned),
        }
    }
}
