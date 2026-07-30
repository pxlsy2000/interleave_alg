use super::model_kind::{MultiStreamScenario, StrideScenario, SweepScenario};

/// A structurally valid access count.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AccessCount(u64);

impl AccessCount {
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the number of accesses.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A structurally valid short-term window size.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WindowSize(u64);

impl WindowSize {
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the window size in accesses.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A validated Scenario case name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CaseName(String);

impl CaseName {
    pub(super) const fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns the decoded case name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated stream name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StreamName(String);

impl StreamName {
    pub(super) const fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns the decoded stream name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Scenario-wide values inherited by eligible cases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioDefaults {
    pub(super) accesses: AccessCount,
    pub(super) window_sizes: Vec<WindowSize>,
}

impl ScenarioDefaults {
    /// Returns the default access count.
    pub const fn accesses(&self) -> AccessCount {
        self.accesses
    }

    /// Returns default windows in declaration order.
    pub fn window_sizes(&self) -> &[WindowSize] {
        &self.window_sizes
    }
}

/// The sole v1 stream merge schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Schedule {
    /// Take one access per active stream in declaration order.
    RoundRobin,
}

/// A structurally valid kind-specific case payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScenarioCaseKind {
    /// One fixed linear sequence.
    Stride(StrideScenario),
    /// A declaration-ordered Cartesian product.
    Sweep(SweepScenario),
    /// One declaration-ordered stream merge.
    MultiStream(MultiStreamScenario),
}

/// One structurally valid declared case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioCase {
    pub(super) name: CaseName,
    pub(super) enabled: bool,
    pub(super) window_sizes: Option<Vec<WindowSize>>,
    pub(super) kind: ScenarioCaseKind,
}

impl ScenarioCase {
    /// Returns the case name.
    pub const fn name(&self) -> &CaseName {
        &self.name
    }

    /// Returns whether default selection includes this case.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the declared window override, if present.
    pub fn window_sizes(&self) -> Option<&[WindowSize]> {
        self.window_sizes.as_deref()
    }

    /// Returns the kind-specific payload.
    pub const fn kind(&self) -> &ScenarioCaseKind {
        &self.kind
    }
}

/// A whole-document structurally valid Scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioModel {
    pub(super) defaults: ScenarioDefaults,
    pub(super) cases: Vec<ScenarioCase>,
}

impl ScenarioModel {
    /// Returns inherited defaults.
    pub const fn defaults(&self) -> &ScenarioDefaults {
        &self.defaults
    }

    /// Returns cases in declaration order.
    pub fn cases(&self) -> &[ScenarioCase] {
        &self.cases
    }
}
