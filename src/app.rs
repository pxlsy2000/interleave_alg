//! Process-boundary orchestration for implemented commands.

mod error;
mod input;
mod map;
mod output;
mod validate;

use std::{
    io::{self, Write as _},
    process::ExitCode,
};

use clap::error::ErrorKind;

use crate::{
    cli::{self, Cli, CliCommand, TemplateCommand},
    io::atomic_output::{OutputRequest, write_report},
    templates,
};

use self::{
    error::{ExecutionError, UsageError},
    output::{OutputOptions, write_execution_error, write_usage_error},
};

/// Runs the native command-line process and returns its exact exit status.
#[must_use]
pub fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return route_clap_error(&error),
    };
    if let Err(error) = validate_usage(&cli) {
        return write_usage_error(error);
    }

    let result = match cli.command {
        CliCommand::Template(arguments) => execute_template(arguments.kind),
        CliCommand::Validate(arguments) => validate::execute(&arguments),
        CliCommand::Map(arguments) => map::execute(&arguments),
    };
    match result {
        Ok(exit) => ExitCode::from(exit),
        Err(error) => {
            write_execution_error(&error);
            ExitCode::from(error.exit_code())
        }
    }
}

fn validate_usage(cli: &Cli) -> Result<(), UsageError> {
    match &cli.command {
        CliCommand::Template(arguments) => {
            let output = match &arguments.kind {
                TemplateCommand::Mapping(options) | TemplateCommand::Scenario(options) => options,
            };
            OutputOptions::new(Some(&output.output), output.force).validate()
        }
        CliCommand::Validate(arguments) => {
            if arguments.verbose && arguments.format == cli::OutputFormat::Json {
                return Err(UsageError::VerboseJson);
            }
            OutputOptions::new(arguments.output.as_deref(), arguments.force).validate()
        }
        CliCommand::Map(arguments) => {
            OutputOptions::new(arguments.output.as_deref(), arguments.force).validate()
        }
    }
}

fn execute_template(command: TemplateCommand) -> Result<u8, ExecutionError> {
    let (template, options) = match command {
        TemplateCommand::Mapping(options) => (templates::mapping(), options),
        TemplateCommand::Scenario(options) => (templates::scenario(), options),
    };
    let output = OutputOptions::new(Some(&options.output), options.force);
    let destination = output.destination();
    let request = OutputRequest::new(destination, options.force, &[]);
    write_report(&request, template.as_bytes(), &mut io::stdout().lock())?;
    Ok(0)
}

fn route_clap_error(error: &clap::Error) -> ExitCode {
    if error.kind() == ErrorKind::DisplayVersion {
        let version = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"), "\n");
        return match io::stdout().lock().write_all(version.as_bytes()) {
            Ok(()) => ExitCode::from(0),
            Err(_) => ExitCode::from(1),
        };
    }
    let exit = u8::from(error.kind() != ErrorKind::DisplayHelp);
    if error.print().is_err() {
        return ExitCode::from(1);
    }
    ExitCode::from(exit)
}
