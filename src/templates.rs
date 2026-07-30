//! Exact v1 YAML template assets.

const MAPPING: &str = include_str!("../assets/templates/mapping.yaml");
const SCENARIO: &str = include_str!("../assets/templates/scenario.yaml");

/// Returns the exact v1 Mapping YAML template.
pub const fn mapping() -> &'static str {
    MAPPING
}

/// Returns the exact v1 Scenario YAML template.
pub const fn scenario() -> &'static str {
    SCENARIO
}
