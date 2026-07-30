use crate::{input::scalar::Ratio, mapping::TargetCount};

use super::{MetricsError, exact_ratio};

/// One Target's exact long-term count and share.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetMetrics {
    target: u16,
    count: u64,
    share: Ratio,
}

impl TargetMetrics {
    /// Returns the Target ID.
    pub const fn target(self) -> u16 {
        self.target
    }

    /// Returns the number of accesses mapped to this Target.
    pub const fn count(self) -> u64 {
        self.count
    }

    /// Returns the exact unreduced `count / Q` share.
    pub const fn share(self) -> Ratio {
        self.share
    }
}

/// The maximum long-term Target load.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaximumLoad {
    target: u16,
    count: u64,
    ratio: Ratio,
}

impl MaximumLoad {
    /// Returns the representative Target ID.
    pub const fn target(self) -> u16 {
        self.target
    }

    /// Returns the representative Target count.
    pub const fn count(self) -> u64 {
        self.count
    }

    /// Returns the exact unreduced `N * max(count) / Q` ratio.
    pub const fn ratio(self) -> Ratio {
        self.ratio
    }
}

pub(super) fn analyze_counts(
    targets: &[u16],
    target_count: TargetCount,
) -> Result<(Vec<TargetMetrics>, MaximumLoad), MetricsError> {
    let q = u64::try_from(targets.len()).map_err(|_| MetricsError::AnalysisFailed)?;
    if q == 0 {
        return Err(MetricsError::AnalysisFailed);
    }
    let capacity = usize::try_from(target_count.get()).map_err(|_| MetricsError::AnalysisFailed)?;
    let mut counts = Vec::new();
    counts
        .try_reserve_exact(capacity)
        .map_err(|_| MetricsError::AnalysisFailed)?;
    counts.resize(capacity, 0_u64);
    for target in targets {
        let count = counts
            .get_mut(usize::from(*target))
            .ok_or(MetricsError::AnalysisFailed)?;
        *count = count.checked_add(1).ok_or(MetricsError::AnalysisFailed)?;
    }

    let (max_index, max_count) = counts
        .iter()
        .copied()
        .enumerate()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .ok_or(MetricsError::AnalysisFailed)?;
    let max_target = u16::try_from(max_index).map_err(|_| MetricsError::AnalysisFailed)?;
    let max_load = MaximumLoad {
        target: max_target,
        count: max_count,
        ratio: maximum_load_ratio(target_count.get(), max_count, q)?,
    };

    let mut rows = Vec::new();
    rows.try_reserve_exact(capacity)
        .map_err(|_| MetricsError::AnalysisFailed)?;
    for (target, count) in counts.into_iter().enumerate() {
        rows.push(TargetMetrics {
            target: u16::try_from(target).map_err(|_| MetricsError::AnalysisFailed)?,
            count,
            share: exact_ratio(u128::from(count), u128::from(q))?,
        });
    }
    Ok((rows, max_load))
}

fn maximum_load_ratio(
    target_count: u32,
    maximum_count: u64,
    accesses: u64,
) -> Result<Ratio, MetricsError> {
    let numerator = u128::from(target_count)
        .checked_mul(u128::from(maximum_count))
        .ok_or(MetricsError::AnalysisFailed)?;
    exact_ratio(numerator, u128::from(accesses))
}

#[cfg(test)]
#[path = "counts_tests.rs"]
mod tests;
