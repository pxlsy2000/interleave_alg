use super::MetricsError;

/// The earliest longest consecutive run of one Target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LongestRun {
    length: u64,
    target: u16,
    start_index: u64,
}

impl LongestRun {
    /// Returns the run length in accesses.
    pub const fn length(self) -> u64 {
        self.length
    }

    /// Returns the run's Target ID.
    pub const fn target(self) -> u16 {
        self.target
    }

    /// Returns the zero-based run start.
    pub const fn start_index(self) -> u64 {
        self.start_index
    }
}

pub(super) fn analyze_runs(targets: &[u16]) -> Result<LongestRun, MetricsError> {
    let mut entries = targets.iter().copied().enumerate();
    let (_, first_target) = entries.next().ok_or(MetricsError::AnalysisFailed)?;
    let mut current = LongestRun {
        length: 1,
        target: first_target,
        start_index: 0,
    };
    let mut best = current;
    for (index, target) in entries {
        if target == current.target {
            current.length = current
                .length
                .checked_add(1)
                .ok_or(MetricsError::AnalysisFailed)?;
        } else {
            if current.length > best.length {
                best = current;
            }
            current = LongestRun {
                length: 1,
                target,
                start_index: u64::try_from(index).map_err(|_| MetricsError::AnalysisFailed)?,
            };
        }
    }
    if current.length > best.length {
        best = current;
    }
    Ok(best)
}
