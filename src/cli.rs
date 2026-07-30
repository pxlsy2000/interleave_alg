//! Typed command-line grammar.

use std::{convert::Infallible, ffi::OsString, path::PathBuf, str::FromStr};

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
    pub addresses: Vec<AddressOperand>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
/// One address operand preserved exactly for domain parsing.
pub struct AddressOperand(String);

impl AddressOperand {
    /// Returns the original command-line lexeme.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for AddressOperand {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for AddressOperand {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let original = value.strip_prefix('\0').map_or(value, |original| original);
        Ok(Self(original.to_owned()))
    }
}

impl Cli {
    /// Parses the native process arguments without exiting the process.
    pub fn try_parse() -> Result<Self, clap::Error> {
        Self::try_parse_from(std::env::args_os())
    }

    /// Parses an explicit argument iterator through the process-equivalent wrapper.
    pub fn try_parse_from<I, T>(arguments: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        <Self as Parser>::try_parse_from(normalize_map_arguments(arguments))
    }
}

fn normalize_map_arguments<I, T>(arguments: I) -> Vec<OsString>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut normalized: Vec<OsString> = arguments.into_iter().map(Into::into).collect();
    let Some(map_index) = normalized.iter().position(|argument| argument == "map") else {
        return normalized;
    };
    let mut option_value = false;
    for argument in normalized.iter_mut().skip(map_index + 1) {
        if option_value {
            option_value = false;
            continue;
        }
        let Some(value) = argument.to_str() else {
            continue;
        };
        if value == "--" {
            break;
        }
        if matches!(value, "--spec" | "--format" | "--output") {
            option_value = true;
            continue;
        }
        if value.starts_with("-0x") || value.starts_with("-0X") {
            let mut escaped = String::with_capacity(value.len() + 1);
            escaped.push('\0');
            escaped.push_str(value);
            *argument = OsString::from(escaped);
        }
    }
    normalized
}

/// Builds the parser from the typed grammar and authoritative package metadata.
#[must_use]
pub fn command() -> Command {
    Cli::command()
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::{Cli, CliCommand, command};

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

    #[test]
    fn map_option_values_that_look_signed_are_not_normalized_as_operands() {
        // Given
        let arguments = ["interleave", "map", "--spec=-0x1", "0", "--output=-0x2"];

        // When
        let parsed = Cli::try_parse_from(arguments);

        // Then
        let Ok(Cli {
            command: CliCommand::Map(map),
        }) = parsed
        else {
            panic!("map arguments must parse");
        };
        assert_eq!(map.spec.to_string_lossy(), "-0x1");
        assert_eq!(
            map.output.as_deref().map(|path| path.to_string_lossy()),
            Some("-0x2".into())
        );
        assert_eq!(
            map.addresses.first().map(super::AddressOperand::as_str),
            Some("0")
        );
    }
}
