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
        write!(output, "{label} [{}]", issue.code)?;
        if !issue.path.is_empty() {
            output.push(' ');
            write_terminal_safe(output, &issue.path)?;
        }
        output.push_str(": ");
        write_terminal_safe(output, &issue.message)?;
        output.push('\n');
    }
    Ok(())
}

fn write_terminal_safe(output: &mut String, value: &str) -> fmt::Result {
    for character in value.chars() {
        match character {
            '\u{0008}' => output.push_str("\\b"),
            '\u{0009}' => output.push_str("\\t"),
            '\u{000a}' => output.push_str("\\n"),
            '\u{000c}' => output.push_str("\\f"),
            '\u{000d}' => output.push_str("\\r"),
            '\u{0000}'..='\u{001f}' | '\u{007f}'..='\u{009f}' | '\u{2028}' | '\u{2029}' => {
                write!(output, "\\u{:04x}", u32::from(character))?;
            }
            _ => output.push(character),
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
