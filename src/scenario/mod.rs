mod decode;
mod decode_case;
mod decode_collection;
mod decode_kind;
mod decode_stream;
mod decode_support;
mod decode_values;
mod expand;
mod expand_model;
mod expand_prepare;
mod expand_totals;
mod generate;
mod model;
mod model_kind;
mod select;

pub use decode::{ScenarioDecodeError, decode_scenario};
pub use expand::{ScenarioPreflightError, preflight_scenarios};
pub use expand_model::{
    ConcreteStimulus, ConcreteTestDescriptor, LinearAccess, ScenarioPreflightPlan, StreamAccess,
};
pub use model::{
    AccessCount, CaseName, ScenarioCase, ScenarioCaseKind, ScenarioDefaults, ScenarioModel,
    Schedule, StreamName, WindowSize,
};
pub use model_kind::{MultiStreamScenario, StreamScenario, StrideScenario, SweepScenario};
pub use select::{ScenarioSelectionError, SelectedCase, SelectedCases, select_cases};
