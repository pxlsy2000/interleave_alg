use std::{cmp::Reverse, collections::BTreeSet};

use crate::{input::scalar::Ratio, mapping::TargetCount, scenario::WindowSize};

use super::{MetricsError, exact_ratio};

/// The exact worst-load result for one declared window size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowMetrics {
    size: u64,
    target: u16,
    start_index: u64,
    count: u64,
    ratio: Ratio,
}

impl WindowMetrics {
    /// Returns the window size in accesses.
    pub const fn size(self) -> u64 {
        self.size
    }

    /// Returns the representative Target ID.
    pub const fn target(self) -> u16 {
        self.target
    }

    /// Returns the zero-based representative window start.
    pub const fn start_index(self) -> u64 {
        self.start_index
    }

    /// Returns the representative Target count within the window.
    pub const fn count(self) -> u64 {
        self.count
    }

    /// Returns the exact unreduced `N * count / W` ratio.
    pub const fn ratio(self) -> Ratio {
        self.ratio
    }
}

pub(super) struct WindowAnalysis {
    pub(super) rows: Vec<WindowMetrics>,
    pub(super) target_visits: u128,
}

pub(super) fn analyze_windows(
    targets: &[u16],
    target_count: TargetCount,
    window_sizes: &[WindowSize],
) -> Result<WindowAnalysis, MetricsError> {
    if targets.is_empty() || window_sizes.is_empty() {
        return Err(MetricsError::AnalysisFailed);
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(window_sizes.len())
        .map_err(|_| MetricsError::AnalysisFailed)?;
    let mut target_visits = 0_u128;
    let mut analyzer = WindowAnalyzer::new(target_count)?;
    for window in window_sizes {
        let (row, visits) = analyzer.analyze(targets, *window)?;
        rows.push(row);
        target_visits = target_visits
            .checked_add(visits)
            .ok_or(MetricsError::AnalysisFailed)?;
    }
    Ok(WindowAnalysis {
        rows,
        target_visits,
    })
}

struct WindowAnalyzer {
    target_count: TargetCount,
    tracker: CountTracker,
}

impl WindowAnalyzer {
    fn new(target_count: TargetCount) -> Result<Self, MetricsError> {
        Ok(Self {
            target_count,
            tracker: CountTracker::new(target_count)?,
        })
    }

    fn analyze(
        &mut self,
        targets: &[u16],
        window: WindowSize,
    ) -> Result<(WindowMetrics, u128), MetricsError> {
        let size = usize::try_from(window.get()).map_err(|_| MetricsError::AnalysisFailed)?;
        if size == 0 || size > targets.len() {
            return Err(MetricsError::AnalysisFailed);
        }
        let mut target_visits = 0_u128;
        for target in targets.iter().copied().take(size) {
            self.tracker.include(target)?;
            target_visits = target_visits
                .checked_add(1)
                .ok_or(MetricsError::AnalysisFailed)?;
        }
        let (best_count, best_target) = self.tracker.maximum()?;
        let mut best = WindowBest {
            count: best_count,
            target: best_target,
            start: 0,
        };
        for (offset, (outgoing, incoming)) in targets
            .iter()
            .copied()
            .zip(targets.iter().copied().skip(size))
            .enumerate()
        {
            self.tracker.exclude(outgoing)?;
            self.tracker.include(incoming)?;
            target_visits = target_visits
                .checked_add(1)
                .ok_or(MetricsError::AnalysisFailed)?;
            let (count, target) = self.tracker.maximum()?;
            if count > best.count {
                best = WindowBest {
                    count,
                    target,
                    start: u64::try_from(offset)
                        .map_err(|_| MetricsError::AnalysisFailed)?
                        .checked_add(1)
                        .ok_or(MetricsError::AnalysisFailed)?,
                };
            }
        }
        for target in targets.iter().copied().skip(targets.len() - size) {
            self.tracker.exclude(target)?;
        }
        let numerator = u128::from(self.target_count.get())
            .checked_mul(u128::from(best.count))
            .ok_or(MetricsError::AnalysisFailed)?;
        Ok((
            WindowMetrics {
                size: window.get(),
                target: best.target,
                start_index: best.start,
                count: best.count,
                ratio: exact_ratio(numerator, u128::from(window.get()))?,
            },
            target_visits,
        ))
    }
}

struct WindowBest {
    count: u64,
    target: u16,
    start: u64,
}

struct CountTracker {
    counts: Vec<u64>,
    order: BTreeSet<(Reverse<u64>, u16)>,
}

impl CountTracker {
    fn new(target_count: TargetCount) -> Result<Self, MetricsError> {
        let capacity =
            usize::try_from(target_count.get()).map_err(|_| MetricsError::AnalysisFailed)?;
        let mut counts = Vec::new();
        counts
            .try_reserve_exact(capacity)
            .map_err(|_| MetricsError::AnalysisFailed)?;
        counts.resize(capacity, 0);
        Ok(Self {
            counts,
            order: BTreeSet::new(),
        })
    }

    fn include(&mut self, target: u16) -> Result<(), MetricsError> {
        self.change(target, CountChange::Increment)
    }

    fn exclude(&mut self, target: u16) -> Result<(), MetricsError> {
        self.change(target, CountChange::Decrement)
    }

    fn change(&mut self, target: u16, change: CountChange) -> Result<(), MetricsError> {
        let count = self
            .counts
            .get_mut(usize::from(target))
            .ok_or(MetricsError::AnalysisFailed)?;
        let previous = *count;
        if previous > 0 && !self.order.remove(&(Reverse(previous), target)) {
            return Err(MetricsError::AnalysisFailed);
        }
        *count = match change {
            CountChange::Increment => previous.checked_add(1),
            CountChange::Decrement => previous.checked_sub(1),
        }
        .ok_or(MetricsError::AnalysisFailed)?;
        if *count > 0 && !self.order.insert((Reverse(*count), target)) {
            return Err(MetricsError::AnalysisFailed);
        }
        Ok(())
    }

    fn maximum(&self) -> Result<(u64, u16), MetricsError> {
        self.order
            .first()
            .map(|(Reverse(count), target)| (*count, *target))
            .ok_or(MetricsError::AnalysisFailed)
    }
}

#[derive(Clone, Copy)]
enum CountChange {
    Increment,
    Decrement,
}
