use crate::{
    cli::RunArgs,
    input::{InputLoadError, parse_yaml},
    io::input::{InputIdentity, InputSnapshot},
    issue::{Issue, Severity},
    mapping::{
        AddressMapper, MappingClassification, MappingValidation, decode_mapping, validate_mapping,
    },
    report::{Report, ReportCommand, TextReportStyle},
    scenario::{decode_scenario, preflight_scenarios, select_cases},
};

use super::{
    error::ExecutionError,
    input::acquire,
    output::{OutputOptions, ReportOutput},
    run_analysis::{AnalysisFailure, analyze_plan},
};

const ANALYSIS_EXIT: u8 = 4;

pub(super) fn execute(arguments: &RunArgs) -> Result<u8, ExecutionError> {
    let (mapping_snapshot, mapping_name) = acquire(&arguments.spec)?;
    let (scenario_snapshot, _) =
        acquire(&arguments.scenario).map_err(classify_scenario_read_error)?;
    let identities = input_identities(&mapping_snapshot, &scenario_snapshot);
    let route = ReportOutput::new(
        arguments.format,
        OutputOptions::new(arguments.output.as_deref(), arguments.force),
        TextReportStyle::Standard,
    );
    route.preflight(&identities)?;

    let mapping_document = match parse_yaml(&mapping_snapshot) {
        Ok(document) => document,
        Err(error) => return render_input_failure(error, route, &identities, 2),
    };
    let mapping = match decode_mapping(&mapping_document) {
        Ok(mapping) => mapping,
        Err(error) => return render_failure(error.issues().to_vec(), route, &identities, 2),
    };
    let validation = validate_mapping(&mapping);
    match validation.classification() {
        MappingClassification::InvalidTargetUnreachable
        | MappingClassification::InvalidNonBijective => {
            return render_failure(validation.issues().to_vec(), route, &identities, 2);
        }
        MappingClassification::ValidNatural | MappingClassification::ValidNonNatural => {}
    }

    let scenario_document = match parse_yaml(&scenario_snapshot) {
        Ok(document) => document,
        Err(error) => {
            return render_scenario_input_failure(error, &validation, route, &identities);
        }
    };
    let scenario = match decode_scenario(&scenario_document) {
        Ok(scenario) => scenario,
        Err(error) => {
            return render_after_mapping_failure(
                &validation,
                error.issues().to_vec(),
                route,
                &identities,
                3,
            );
        }
    };
    let requested = arguments
        .cases
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let selected = match select_cases(&scenario, &requested) {
        Ok(selected) => selected,
        Err(error) => {
            return render_after_mapping_failure(
                &validation,
                error.issues().to_vec(),
                route,
                &identities,
                3,
            );
        }
    };
    let plan = match preflight_scenarios(&mapping, &selected) {
        Ok(plan) => plan,
        Err(error) => {
            return render_after_mapping_failure(
                &validation,
                error.issues().to_vec(),
                route,
                &identities,
                3,
            );
        }
    };
    let Ok(mapper) = AddressMapper::try_new(&mapping) else {
        return render_analysis_failure(&validation, route, &identities);
    };
    let Ok(cases) = analyze_plan(&mapping, &mapper, &plan) else {
        return render_analysis_failure(&validation, route, &identities);
    };
    let Ok(report) =
        Report::run_with_input(&mapping, Some(&mapping_name), mapper.validation(), cases)
    else {
        return render_analysis_failure(&validation, route, &identities);
    };
    route.write(&report, &identities, 0)
}

fn input_identities(mapping: &InputSnapshot, scenario: &InputSnapshot) -> Vec<InputIdentity> {
    mapping
        .identity()
        .into_iter()
        .chain(scenario.identity())
        .collect()
}

fn classify_scenario_read_error(error: ExecutionError) -> ExecutionError {
    match error {
        ExecutionError::Input(input) | ExecutionError::ScenarioInput(input) => {
            ExecutionError::ScenarioInput(input)
        }
        ExecutionError::Output(output) => ExecutionError::Output(output),
        ExecutionError::JsonRender(render) => ExecutionError::JsonRender(render),
        ExecutionError::TextRender(render) => ExecutionError::TextRender(render),
        ExecutionError::ReportModel(model) => ExecutionError::ReportModel(model),
        ExecutionError::Stderr(stderr) => ExecutionError::Stderr(stderr),
    }
}

fn render_input_failure(
    error: InputLoadError,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
    exit: u8,
) -> Result<u8, ExecutionError> {
    match error {
        InputLoadError::Read(error) if exit == 3 => Err(ExecutionError::ScenarioInput(error)),
        InputLoadError::Read(error) => Err(ExecutionError::Input(error)),
        InputLoadError::Yaml { issue, .. } | InputLoadError::Schema { issue, .. } => {
            render_failure(vec![issue], route, identities, exit)
        }
    }
}

fn render_scenario_input_failure(
    error: InputLoadError,
    validation: &MappingValidation,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
) -> Result<u8, ExecutionError> {
    match error {
        InputLoadError::Read(error) => Err(ExecutionError::ScenarioInput(error)),
        InputLoadError::Yaml { issue, .. } | InputLoadError::Schema { issue, .. } => {
            render_after_mapping_failure(validation, vec![issue], route, identities, 3)
        }
    }
}

fn render_after_mapping_failure(
    validation: &MappingValidation,
    issues: Vec<Issue>,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
    exit: u8,
) -> Result<u8, ExecutionError> {
    let report = report_after_mapping_failure(validation, issues)?;
    route.write(&report, identities, exit)
}

fn report_after_mapping_failure(
    validation: &MappingValidation,
    mut issues: Vec<Issue>,
) -> Result<Report, crate::report::ReportModelError> {
    issues.extend(
        validation
            .issues()
            .iter()
            .filter(|issue| issue.severity() == Severity::Warning)
            .cloned(),
    );
    Report::failure(ReportCommand::Run, issues)
}

fn render_failure(
    issues: Vec<Issue>,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
    exit: u8,
) -> Result<u8, ExecutionError> {
    let report = Report::failure(ReportCommand::Run, issues)?;
    route.write(&report, identities, exit)
}

fn render_analysis_failure(
    validation: &MappingValidation,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
) -> Result<u8, ExecutionError> {
    render_after_mapping_failure(
        validation,
        vec![AnalysisFailure::issue()],
        route,
        identities,
        ANALYSIS_EXIT,
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        input::load_yaml_bytes,
        mapping::{decode_mapping, validate_mapping},
        report::{TextReportStyle, render_json, render_text},
    };

    use super::{ANALYSIS_EXIT, AnalysisFailure, report_after_mapping_failure};

    #[test]
    fn injected_analysis_failure_retains_mapping_warning_in_both_formats()
    -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let source = "schema_version: 1\nname: warning\naddress:\n  width_bits: 8\n\
                      \x20 granule_bytes: 64\ntargets:\n  count: 2\nmapping:\n  m:\n    rows: [[1]]\n\
                      \x20 l:\n    mode: explicit\n    rows: [[0]]\n";
        let document = load_yaml_bytes(source.as_bytes())?;
        let mapping = decode_mapping(&document)?;
        let validation = validate_mapping(&mapping);

        // When
        let report = report_after_mapping_failure(&validation, vec![AnalysisFailure::issue()])?;
        let json = String::from_utf8(render_json(&report)?)?;
        let text = String::from_utf8(render_text(&report, TextReportStyle::Standard)?)?;

        // Then
        assert_eq!(ANALYSIS_EXIT, 4);
        assert!(json.contains("\"code\": \"mapping.non_natural\""));
        assert!(json.contains("\"code\": \"analysis.failed\""));
        assert!(text.contains("WARNING [mapping.non_natural]"));
        assert!(text.contains("ERROR [analysis.failed]"));
        Ok(())
    }
}
