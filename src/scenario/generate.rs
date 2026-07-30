use std::collections::VecDeque;

use thiserror::Error;

use crate::{
    input::scalar::Address,
    issue::{Issue, IssueCode, IssuePath},
    mapping::AddressMapper,
};

use super::{ConcreteStimulus, ConcreteTestDescriptor, LinearAccess, StreamAccess};

/// A failure in a computation already approved by Scenario preflight.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ScenarioGenerationError {
    /// Checked arithmetic, allocation, or Mapping unexpectedly failed.
    #[error("analysis could not be completed")]
    AnalysisFailed,
}

impl ScenarioGenerationError {
    /// Converts an internal computation failure into the stable command diagnostic.
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

/// Generates one concrete test's ordered Target sequence.
pub fn generate_target_sequence(
    mapper: &AddressMapper<'_>,
    descriptor: &ConcreteTestDescriptor,
) -> Result<Vec<u16>, ScenarioGenerationError> {
    let capacity = usize::try_from(descriptor.accesses())
        .map_err(|_| ScenarioGenerationError::AnalysisFailed)?;
    let mut targets = Vec::new();
    targets
        .try_reserve_exact(capacity)
        .map_err(|_| ScenarioGenerationError::AnalysisFailed)?;
    match descriptor.stimulus() {
        ConcreteStimulus::Linear(linear) => {
            for index in 0..linear.accesses().get() {
                push_mapped_target(mapper, *linear, index, &mut targets)?;
            }
        }
        ConcreteStimulus::RoundRobin(streams) => {
            let mut schedule = RoundRobinSchedule::new(streams)?;
            while let Some(access) = schedule.next_access()? {
                push_mapped_target(mapper, access.linear, access.index, &mut targets)?;
            }
        }
    }
    if targets.len() != capacity {
        return Err(ScenarioGenerationError::AnalysisFailed);
    }
    Ok(targets)
}

#[derive(Clone, Copy)]
struct ActiveStream {
    linear: LinearAccess,
    next_index: u64,
}

pub(super) struct ScheduledAccess {
    pub(super) linear: LinearAccess,
    pub(super) index: u64,
}

pub(super) struct RoundRobinSchedule {
    active: VecDeque<ActiveStream>,
}

impl RoundRobinSchedule {
    pub(super) fn new(streams: &[StreamAccess]) -> Result<Self, ScenarioGenerationError> {
        let mut active = VecDeque::new();
        active
            .try_reserve_exact(streams.len())
            .map_err(|_| ScenarioGenerationError::AnalysisFailed)?;
        active.extend(streams.iter().map(|stream| ActiveStream {
            linear: stream.linear(),
            next_index: 0,
        }));
        Ok(Self { active })
    }

    pub(super) fn next_access(
        &mut self,
    ) -> Result<Option<ScheduledAccess>, ScenarioGenerationError> {
        let Some(mut stream) = self.active.pop_front() else {
            return Ok(None);
        };
        let scheduled = ScheduledAccess {
            linear: stream.linear,
            index: stream.next_index,
        };
        stream.next_index = stream
            .next_index
            .checked_add(1)
            .ok_or(ScenarioGenerationError::AnalysisFailed)?;
        if stream.next_index < stream.linear.accesses().get() {
            self.active.push_back(stream);
        }
        Ok(Some(scheduled))
    }

    #[cfg(test)]
    pub(super) fn active_len(&self) -> usize {
        self.active.len()
    }
}

fn push_mapped_target(
    mapper: &AddressMapper<'_>,
    linear: LinearAccess,
    index: u64,
    targets: &mut Vec<u16>,
) -> Result<(), ScenarioGenerationError> {
    let address = linear
        .stride_bytes()
        .get()
        .checked_mul(u128::from(index))
        .and_then(|offset| linear.base_bytes().get().checked_add(offset))
        .ok_or(ScenarioGenerationError::AnalysisFailed)?;
    let result = mapper
        .map_address(Address::from_u128(address))
        .map_err(|_| ScenarioGenerationError::AnalysisFailed)?;
    targets.push(result.target());
    Ok(())
}
