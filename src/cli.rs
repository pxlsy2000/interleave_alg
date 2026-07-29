use clap::Command;

/// Builds the command-line parser using package metadata as its sole version source.
#[must_use]
pub fn command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg_required_else_help(true)
        .disable_help_subcommand(true)
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
        let Err(error) = result else {
            panic!("the version flag must short-circuit parsing");
        };
        assert_eq!(error.kind(), ErrorKind::DisplayVersion);
        assert_eq!(error.to_string(), "interleave 0.1.0\n");
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
