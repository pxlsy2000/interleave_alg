use crate::{
    cli::ValidateArgs,
    input::{InputLoadError, parse_yaml},
    io::input::InputIdentity,
    mapping::{MappingClassification, decode_mapping, validate_mapping},
    report::{Report, ReportCommand, TextReportStyle},
};

use super::{
    error::ExecutionError,
    input::acquire,
    output::{OutputOptions, ReportOutput},
};

pub(super) fn execute(arguments: &ValidateArgs) -> Result<u8, ExecutionError> {
    let (snapshot, input_name) = acquire(&arguments.spec)?;
    let identities: Vec<InputIdentity> = snapshot.identity().into_iter().collect();
    let route = ReportOutput::new(
        arguments.format,
        OutputOptions::new(arguments.output.as_deref(), arguments.force),
        if arguments.verbose {
            TextReportStyle::Verbose
        } else {
            TextReportStyle::Standard
        },
    );
    route.preflight(&identities)?;

    let document = match parse_yaml(&snapshot) {
        Ok(document) => document,
        Err(error) => {
            return render_schema_failure(error, route, &identities);
        }
    };
    let mapping = match decode_mapping(&document) {
        Ok(mapping) => mapping,
        Err(error) => {
            return render_issues_failure(error.issues().to_vec(), route, &identities);
        }
    };
    let validation = validate_mapping(&mapping);
    let exit = match validation.classification() {
        MappingClassification::ValidNatural | MappingClassification::ValidNonNatural => 0,
        MappingClassification::InvalidTargetUnreachable
        | MappingClassification::InvalidNonBijective => 2,
    };
    let report = Report::validate(&mapping, &input_name, &validation);
    route.write(&report, &identities, exit)
}

fn render_schema_failure(
    error: InputLoadError,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
) -> Result<u8, ExecutionError> {
    match error {
        InputLoadError::Read(error) => Err(ExecutionError::Input(error)),
        InputLoadError::Yaml { issue, .. } | InputLoadError::Schema { issue, .. } => {
            render_issues_failure(vec![issue], route, identities)
        }
    }
}

fn render_issues_failure(
    issues: Vec<crate::issue::Issue>,
    route: ReportOutput<'_>,
    identities: &[InputIdentity],
) -> Result<u8, ExecutionError> {
    let report = Report::failure(ReportCommand::Validate, issues)?;
    route.write(&report, identities, 2)
}
