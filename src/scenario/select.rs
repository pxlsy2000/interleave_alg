use std::collections::BTreeSet;

use thiserror::Error;

use crate::{
    error::ExitClass,
    issue::{Issue, IssueCode, IssuePath, canonical_json_string},
};

use super::model::{ScenarioCase, ScenarioDefaults, ScenarioModel};

/// One selected case paired with its Scenario declaration index.
#[derive(Clone, Copy, Debug)]
pub struct SelectedCase<'scenario> {
    source_index: usize,
    case: &'scenario ScenarioCase,
}

impl<'scenario> SelectedCase<'scenario> {
    /// Returns the zero-based Scenario declaration index.
    pub const fn source_index(self) -> usize {
        self.source_index
    }

    /// Returns the selected case.
    pub const fn case(self) -> &'scenario ScenarioCase {
        self.case
    }
}

/// A successful, declaration-ordered Scenario selection.
#[derive(Debug)]
pub struct SelectedCases<'scenario> {
    defaults: &'scenario ScenarioDefaults,
    cases: Vec<SelectedCase<'scenario>>,
}

impl<'scenario> SelectedCases<'scenario> {
    /// Returns inherited Scenario defaults.
    pub const fn defaults(&self) -> &'scenario ScenarioDefaults {
        self.defaults
    }

    /// Returns selected cases in Scenario declaration order.
    pub fn cases(&self) -> &[SelectedCase<'scenario>] {
        &self.cases
    }
}

/// A deterministic case-selection failure.
#[derive(Debug, Error)]
#[error("Scenario case selection failed")]
pub struct ScenarioSelectionError {
    issues: Vec<Issue>,
}

impl ScenarioSelectionError {
    /// Returns issues in normative command-line occurrence order.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Returns Scenario failure exit class 3.
    pub const fn exit_class(&self) -> ExitClass {
        ExitClass::ScenarioOrAddress
    }
}

/// Selects default-enabled or explicitly named cases with deterministic ordering.
pub fn select_cases<'scenario>(
    scenario: &'scenario ScenarioModel,
    requested: &[&str],
) -> Result<SelectedCases<'scenario>, ScenarioSelectionError> {
    if requested.is_empty() {
        return selected_or_empty_error(
            scenario,
            scenario
                .cases()
                .iter()
                .enumerate()
                .filter(|(_, case)| case.enabled())
                .map(|(source_index, case)| SelectedCase { source_index, case })
                .collect(),
        );
    }

    let declared = scenario
        .cases()
        .iter()
        .map(|case| case.name().as_str())
        .collect::<BTreeSet<_>>();
    let mut encountered = BTreeSet::new();
    let missing = requested
        .iter()
        .copied()
        .filter(|name| encountered.insert(*name) && !declared.contains(name))
        .map(|name| {
            Issue::new(
                IssueCode::ScenarioCaseNotFound,
                IssuePath::root(),
                format!("case {} was not found", canonical_json_string(name)),
            )
        })
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(ScenarioSelectionError { issues: missing });
    }

    let requested = requested.iter().copied().collect::<BTreeSet<_>>();
    let selected = scenario
        .cases()
        .iter()
        .enumerate()
        .filter(|(_, case)| requested.contains(case.name().as_str()))
        .map(|(source_index, case)| SelectedCase { source_index, case })
        .collect();
    selected_or_empty_error(scenario, selected)
}

fn selected_or_empty_error<'scenario>(
    scenario: &'scenario ScenarioModel,
    cases: Vec<SelectedCase<'scenario>>,
) -> Result<SelectedCases<'scenario>, ScenarioSelectionError> {
    if cases.is_empty() {
        Err(ScenarioSelectionError {
            issues: vec![Issue::new(
                IssueCode::ScenarioNoCaseSelected,
                IssuePath::root(),
                "no scenario case was selected",
            )],
        })
    } else {
        Ok(SelectedCases {
            defaults: scenario.defaults(),
            cases,
        })
    }
}
