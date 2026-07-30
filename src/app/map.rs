use crate::{
    cli::MapArgs,
    input::{InputLoadError, parse_yaml, preflight_query_addresses},
    io::input::InputIdentity,
    issue::{Issue, IssueCode},
    mapping::{AddressMapper, MappingClassification, decode_mapping, validate_mapping},
    report::{Report, ReportCommand, TextReportStyle},
};

use super::{
    error::ExecutionError,
    input::acquire,
    output::{OutputOptions, ReportOutput},
};

pub(super) fn execute(arguments: &MapArgs) -> Result<u8, ExecutionError> {
    let (snapshot, input_name) = acquire(&arguments.spec)?;
    let identities: Vec<InputIdentity> = snapshot.identity().into_iter().collect();
    let route = ReportOutput::new(
        arguments.format,
        OutputOptions::new(arguments.output.as_deref(), arguments.force),
        TextReportStyle::Standard,
    );
    route.preflight(&identities)?;

    let document = match parse_yaml(&snapshot) {
        Ok(document) => document,
        Err(error) => return render_schema_failure(error, route, &identities),
    };
    let mapping = match decode_mapping(&document) {
        Ok(mapping) => mapping,
        Err(error) => return render_mapping_failure(error.issues().to_vec(), route, &identities),
    };
    let validation = validate_mapping(&mapping);
    match validation.classification() {
        MappingClassification::InvalidTargetUnreachable
        | MappingClassification::InvalidNonBijective => {
            return render_mapping_failure(validation.issues().to_vec(), route, &identities);
        }
        MappingClassification::ValidNatural | MappingClassification::ValidNonNatural => {}
    }

    let addresses = match preflight_query_addresses(&arguments.addresses, mapping.address_width()) {
        Ok(addresses) => addresses,
        Err(issues) => {
            let exit = query_exit(&issues);
            let report = Report::failure(ReportCommand::Map, issues)?;
            return route.write(&report, &identities, exit);
        }
    };
    let Ok(mapper) = AddressMapper::try_new(&mapping) else {
        let report = Report::failure(
            ReportCommand::Map,
            vec![Issue::new(
                IssueCode::AnalysisFailed,
                crate::issue::IssuePath::root(),
                "analysis could not be completed",
            )],
        )?;
        return route.write(&report, &identities, 4);
    };
    let rows = match mapper.map_addresses(&addresses) {
        Ok(rows) => rows,
        Err(error) => {
            let issue = error.issue();
            let exit = query_exit(std::slice::from_ref(&issue));
            let report = Report::failure(ReportCommand::Map, vec![issue])?;
            return route.write(&report, &identities, exit);
        }
    };
    let report = Report::map_with_input(&mapping, Some(&input_name), mapper.validation(), &rows)?;
    route.write(&report, &identities, 0)
}

fn render_schema_failure(
    error: InputLoadError,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
) -> Result<u8, ExecutionError> {
    match error {
        InputLoadError::Read(error) => Err(ExecutionError::Input(error)),
        InputLoadError::Yaml { issue, .. } | InputLoadError::Schema { issue, .. } => {
            render_mapping_failure(vec![issue], route, identities)
        }
    }
}

fn render_mapping_failure(
    issues: Vec<Issue>,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
) -> Result<u8, ExecutionError> {
    let report = Report::failure(ReportCommand::Map, issues)?;
    route.write(&report, identities, 2)
}

fn query_exit(issues: &[Issue]) -> u8 {
    if issues
        .iter()
        .any(|issue| issue.code() == IssueCode::AnalysisFailed)
    {
        4
    } else {
        3
    }
}
