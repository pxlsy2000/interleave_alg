use thiserror::Error;

use crate::{
    issue::{Issue, IssueCode, IssuePath},
    mapping::{AddressMapper, MappingModel},
    metrics::analyze_targets,
    report::RunCaseResult,
    scenario::{ConcreteTestDescriptor, ScenarioPreflightPlan, generate_target_sequence},
};

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("analysis could not be completed")]
pub(super) struct AnalysisFailure;

impl AnalysisFailure {
    pub(super) fn issue() -> Issue {
        Issue::new(
            IssueCode::AnalysisFailed,
            IssuePath::root(),
            "analysis could not be completed",
        )
    }
}

pub(super) fn analyze_plan(
    mapping: &MappingModel,
    mapper: &AddressMapper<'_>,
    plan: &ScenarioPreflightPlan,
) -> Result<Vec<RunCaseResult>, AnalysisFailure> {
    analyze_plan_with(mapping, mapper, plan, analyze_case)
}

fn analyze_plan_with<F>(
    mapping: &MappingModel,
    mapper: &AddressMapper<'_>,
    plan: &ScenarioPreflightPlan,
    mut analyze: F,
) -> Result<Vec<RunCaseResult>, AnalysisFailure>
where
    F: FnMut(
        &MappingModel,
        &AddressMapper<'_>,
        &ConcreteTestDescriptor,
    ) -> Result<RunCaseResult, AnalysisFailure>,
{
    let mut results = Vec::new();
    results
        .try_reserve_exact(plan.tests().len())
        .map_err(|_| AnalysisFailure)?;
    for descriptor in plan.tests() {
        results.push(analyze(mapping, mapper, descriptor)?);
    }
    Ok(results)
}

fn analyze_case(
    mapping: &MappingModel,
    mapper: &AddressMapper<'_>,
    descriptor: &ConcreteTestDescriptor,
) -> Result<RunCaseResult, AnalysisFailure> {
    let targets = generate_target_sequence(mapper, descriptor).map_err(|_| AnalysisFailure)?;
    let metrics = analyze_targets(&targets, mapping.target_count(), descriptor.window_sizes())
        .map_err(|_| AnalysisFailure)?;
    RunCaseResult::from_metrics(descriptor, &metrics).map_err(|_| AnalysisFailure)
}

#[cfg(test)]
mod tests {
    use crate::{
        input::load_yaml_bytes,
        mapping::{AddressMapper, decode_mapping},
        scenario::{decode_scenario, preflight_scenarios, select_cases},
    };

    use super::{AnalysisFailure, analyze_plan_with};

    #[test]
    fn injected_below_limit_analysis_failure_is_typed() -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let mapping_document = load_yaml_bytes(crate::templates::mapping().as_bytes())?;
        let mapping = decode_mapping(&mapping_document)?;
        let scenario_document = load_yaml_bytes(crate::templates::scenario().as_bytes())?;
        let scenario = decode_scenario(&scenario_document)?;
        let selected = select_cases(&scenario, &["sequential"])?;
        let plan = preflight_scenarios(&mapping, &selected)?;
        let mapper = AddressMapper::try_new(&mapping)?;

        // When
        let result = analyze_plan_with(&mapping, &mapper, &plan, |_, _, _| Err(AnalysisFailure));

        // Then
        assert_eq!(result, Err(AnalysisFailure));
        assert_eq!(AnalysisFailure::issue().code().as_str(), "analysis.failed");
        Ok(())
    }
}
