use crate::input::scalar::Address;

use super::model::{AccessCount, StreamName, WindowSize};

/// One checked linear address sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinearAccess {
    base_bytes: Address,
    stride_bytes: Option<Address>,
    accesses: AccessCount,
}

impl LinearAccess {
    pub(super) const fn new(
        base_bytes: Address,
        stride_bytes: Option<Address>,
        accesses: AccessCount,
    ) -> Self {
        Self {
            base_bytes,
            stride_bytes,
            accesses,
        }
    }

    /// Returns the first byte address.
    pub const fn base_bytes(self) -> Address {
        self.base_bytes
    }

    /// Returns the checked stride, or `None` when one access leaves it unused.
    pub const fn stride_bytes(self) -> Option<Address> {
        self.stride_bytes
    }

    /// Returns the checked access count.
    pub const fn accesses(self) -> AccessCount {
        self.accesses
    }
}

/// One checked stream within a round-robin stimulus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamAccess {
    name: StreamName,
    linear: LinearAccess,
}

impl StreamAccess {
    pub(super) const fn new(name: StreamName, linear: LinearAccess) -> Self {
        Self { name, linear }
    }

    /// Returns the stream name.
    pub const fn name(&self) -> &StreamName {
        &self.name
    }

    /// Returns the stream's checked linear sequence.
    pub const fn linear(&self) -> LinearAccess {
        self.linear
    }
}

/// The checked stimulus shape for one concrete test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConcreteStimulus {
    /// One linear sequence.
    Linear(LinearAccess),
    /// Declaration-ordered active-stream round robin.
    RoundRobin(Vec<StreamAccess>),
}

/// One fully checked concrete-test descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcreteTestDescriptor {
    pub(super) case_id: String,
    pub(super) source_case: String,
    pub(super) accesses: AccessCount,
    pub(super) window_sizes: Vec<WindowSize>,
    pub(super) stimulus: ConcreteStimulus,
}

impl ConcreteTestDescriptor {
    /// Returns the canonical concrete-test ID.
    pub fn case_id(&self) -> &str {
        &self.case_id
    }

    /// Returns the declared source case name.
    pub fn source_case(&self) -> &str {
        &self.source_case
    }

    /// Returns the final concrete-test access count.
    pub const fn accesses(&self) -> u64 {
        self.accesses.get()
    }

    /// Returns effective windows in declaration order.
    pub fn window_sizes(&self) -> &[WindowSize] {
        &self.window_sizes
    }

    /// Returns the checked stimulus without generated addresses or Targets.
    pub const fn stimulus(&self) -> &ConcreteStimulus {
        &self.stimulus
    }
}

/// A complete run plan approved by all Scenario preflight gates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioPreflightPlan {
    tests: Vec<ConcreteTestDescriptor>,
}

impl ScenarioPreflightPlan {
    pub(super) const fn new(tests: Vec<ConcreteTestDescriptor>) -> Self {
        Self { tests }
    }

    /// Returns concrete tests in deterministic execution order.
    pub fn tests(&self) -> &[ConcreteTestDescriptor] {
        &self.tests
    }
}
