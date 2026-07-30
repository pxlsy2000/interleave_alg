//! Typed command-line grammar.

use std::path::PathBuf;

use clap::{ArgAction, Args, Command, CommandFactory as _, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    propagate_version = true,
    arg_required_else_help = true,
    disable_help_subcommand = true,
    color = clap::ColorChoice::Never
)]
/// Parsed root command.
pub struct Cli {
    /// Selected operation.
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
/// Implemented root operations.
pub enum CliCommand {
    /// Generate an exact v1 YAML template.
    Template(TemplateArgs),
    /// Validate one Mapping specification.
    Validate(ValidateArgs),
    /// Map byte addresses through one Mapping specification.
    Map(MapArgs),
}

#[derive(Debug, Args)]
/// Template-kind selection.
pub struct TemplateArgs {
    /// Selected exact template.
    #[command(subcommand)]
    pub kind: TemplateCommand,
}

#[derive(Debug, Subcommand)]
/// Exact template kinds.
pub enum TemplateCommand {
    /// Generate a Mapping template.
    Mapping(TemplateOutputArgs),
    /// Generate a Scenario template.
    Scenario(TemplateOutputArgs),
}

#[derive(Debug, Args)]
/// Common template destination options.
pub struct TemplateOutputArgs {
    /// Report destination.
    #[arg(long, required = true, action = ArgAction::Set)]
    pub output: PathBuf,
    /// Replace an existing regular-file destination.
    #[arg(long, action = ArgAction::SetTrue)]
    pub force: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
/// Supported report encodings.
pub enum OutputFormat {
    /// Human-readable report.
    Text,
    /// Stable machine-readable envelope.
    Json,
}

#[derive(Debug, Args)]
/// Mapping validation options.
pub struct ValidateArgs {
    /// Mapping YAML source path, or - for standard input.
    #[arg(long, required = true, action = ArgAction::Set)]
    pub spec: PathBuf,
    /// Report encoding.
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        action = ArgAction::Set
    )]
    pub format: OutputFormat,
    /// Report destination path, or - for standard output.
    #[arg(long, action = ArgAction::Set)]
    pub output: Option<PathBuf>,
    /// Replace an existing regular-file destination.
    #[arg(long, action = ArgAction::SetTrue)]
    pub force: bool,
    /// Include complete Mapping matrices in text output.
    #[arg(long, action = ArgAction::SetTrue)]
    pub verbose: bool,
}

#[derive(Debug, Args)]
/// Address Mapping options.
pub struct MapArgs {
    /// Mapping YAML source path, or - for standard input.
    #[arg(long, required = true, action = ArgAction::Set)]
    pub spec: PathBuf,
    /// Byte addresses in exact command-line order.
    #[arg(required = true, num_args = 1.., allow_negative_numbers = true)]
    pub addresses: Vec<String>,
    /// Report encoding.
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        action = ArgAction::Set
    )]
    pub format: OutputFormat,
    /// Report destination path, or - for standard output.
    #[arg(long, action = ArgAction::Set)]
    pub output: Option<PathBuf>,
    /// Replace an existing regular-file destination.
    #[arg(long, action = ArgAction::SetTrue)]
    pub force: bool,
}

impl Cli {
    /// Parses the native process arguments without exiting the process.
    pub fn try_parse() -> Result<Self, clap::Error> {
        <Self as Parser>::try_parse()
    }
}

/// Builds the parser from the typed grammar and authoritative package metadata.
#[must_use]
pub fn command() -> Command {
    Cli::command()
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::command;

    #[test]
    fn version_is_reported_from_package_metadata() {
        // Given
        let arguments = ["interleave", "--version"];

        // When
        let result = command().try_get_matches_from(arguments);

        // Then
        assert!(matches!(
            result.as_ref(),
            Err(error) if error.kind() == ErrorKind::DisplayVersion
        ));
        assert_eq!(
            result.err().map(|error| error.to_string()).as_deref(),
            Some("interleave 0.1.0\n")
        );
    }

    #[test]
    fn empty_invocation_is_rejected() {
        // Given
        let arguments = ["interleave"];

        // When
        let result = command().try_get_matches_from(arguments);

        // Then
        assert!(matches!(
            result.as_ref(),
            Err(error) if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        ));
    }
}
