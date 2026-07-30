use std::{ffi::OsStr, io, path::Path};

use crate::{
    cli::{OutputFormat, ValidateArgs},
    input::{InputLoadError, parse_yaml, read_named, read_stdin},
    io::{
        atomic_output::{OutputRequest, preflight_report_output, write_report},
        input::{InputIdentity, InputSnapshot},
    },
    mapping::{MappingClassification, decode_mapping, validate_mapping},
    report::{Report, ReportCommand, TextReportStyle, render_json, render_text},
};

use super::{
    error::ExecutionError,
    output::{OutputOptions, write_business_text},
};

pub(super) fn execute(arguments: &ValidateArgs) -> Result<u8, ExecutionError> {
    let (snapshot, input_name) = acquire(&arguments.spec)?;
    let identities: Vec<InputIdentity> = snapshot.identity().into_iter().collect();
    let output = OutputOptions::new(arguments.output.as_deref(), arguments.force);
    let preflight = OutputRequest::new(output.destination(), arguments.force, &identities);
    preflight_report_output(&preflight)?;

    let document = match parse_yaml(&snapshot) {
        Ok(document) => document,
        Err(error) => {
            return render_schema_failure(error, arguments, output, &identities);
        }
    };
    let mapping = match decode_mapping(&document) {
        Ok(mapping) => mapping,
        Err(error) => {
            return render_issues_failure(error.issues().to_vec(), arguments, output, &identities);
        }
    };
    let validation = validate_mapping(&mapping);
    let exit = match validation.classification() {
        MappingClassification::ValidNatural | MappingClassification::ValidNonNatural => 0,
        MappingClassification::InvalidTargetUnreachable
        | MappingClassification::InvalidNonBijective => 2,
    };
    let report = Report::validate(&mapping, &input_name, &validation);
    render_report(&report, arguments, output, &identities, exit)
}

fn acquire(path: &Path) -> Result<(InputSnapshot, String), ExecutionError> {
    if path.as_os_str() == OsStr::new("-") {
        return Ok((read_stdin()?, "-".to_owned()));
    }
    let snapshot = read_named(path)?;
    Ok((snapshot, path.to_string_lossy().into_owned()))
}

fn render_schema_failure(
    error: InputLoadError,
    arguments: &ValidateArgs,
    output: OutputOptions<'_>,
    identities: &[InputIdentity],
) -> Result<u8, ExecutionError> {
    match error {
        InputLoadError::Read(error) => Err(ExecutionError::Input(error)),
        InputLoadError::Yaml { issue, .. } | InputLoadError::Schema { issue, .. } => {
            render_issues_failure(vec![issue], arguments, output, identities)
        }
    }
}

fn render_issues_failure(
    issues: Vec<crate::issue::Issue>,
    arguments: &ValidateArgs,
    output: OutputOptions<'_>,
    identities: &[InputIdentity],
) -> Result<u8, ExecutionError> {
    let report = Report::failure(ReportCommand::Validate, issues)?;
    render_report(&report, arguments, output, identities, 2)
}

fn render_report(
    report: &Report,
    arguments: &ValidateArgs,
    output: OutputOptions<'_>,
    identities: &[InputIdentity],
    exit: u8,
) -> Result<u8, ExecutionError> {
    let rendered = match arguments.format {
        OutputFormat::Text => render_text(
            report,
            if arguments.verbose {
                TextReportStyle::Verbose
            } else {
                TextReportStyle::Standard
            },
        )?,
        OutputFormat::Json => render_json(report)?,
    };

    if arguments.format == OutputFormat::Text && exit != 0 {
        write_business_text(&rendered)?;
        return Ok(exit);
    }
    let request = OutputRequest::new(output.destination(), arguments.force, identities);
    write_report(&request, &rendered, &mut io::stdout().lock())?;
    Ok(exit)
}
