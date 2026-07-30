use std::fmt::{self, Write};

use crate::issue::canonical_json_string;

use super::super::{Report, ReportCommand, ReportIssue};

pub(super) fn render_failure(output: &mut String, report: &Report) -> fmt::Result {
    match report.command {
        ReportCommand::Validate => writeln!(output, "FAIL  input structure")?,
        ReportCommand::Map => writeln!(output, "FAIL  address preflight")?,
        ReportCommand::Run => writeln!(output, "FAIL  scenario preflight")?,
    }
    render_issues(output, "WARNING", &report.warnings)?;
    render_issues(output, "ERROR", &report.errors)
}

pub(super) fn render_issues(
    output: &mut String,
    label: &str,
    issues: &[ReportIssue],
) -> fmt::Result {
    for issue in issues {
        if issue.path.is_empty() {
            writeln!(output, "{label} [{}]: {}", issue.code, issue.message)?;
        } else {
            writeln!(
                output,
                "{label} [{}] {}: {}",
                issue.code, issue.path, issue.message
            )?;
        }
    }
    Ok(())
}

pub(super) fn safe_field(value: &str) -> String {
    let quoted = canonical_json_string(value);
    let mut chars = quoted.chars();
    let _opening_quote = chars.next();
    let _closing_quote = chars.next_back();
    chars.collect()
}

pub(super) fn input_name(input: Option<&str>) -> String {
    input.map_or_else(|| "<unspecified>".to_owned(), safe_field)
}
