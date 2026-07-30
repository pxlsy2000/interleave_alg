use std::fmt::{self, Write};

use super::{
    super::{MapResult, ReportIssue},
    format::{input_name, render_issues, safe_field},
};

pub(super) fn render(
    output: &mut impl Write,
    result: &MapResult,
    warnings: &[ReportIssue],
    errors: &[ReportIssue],
) -> fmt::Result {
    writeln!(output, "Mapping: {}", safe_field(&result.mapping_name))?;
    writeln!(output, "Input: {}", input_name(result.input.as_deref()))?;
    writeln!(output, "Result: {}", result.mapping_classification.as_str())?;
    render_issues(output, "WARNING", warnings)?;
    render_issues(output, "ERROR", errors)?;
    writeln!(output)?;
    writeln!(
        output,
        "{:<9}{:<14}{:<8}{:<8}{:<9}LA byte",
        "Address", "Line address", "Offset", "Target", "LA line"
    )?;
    for row in &result.addresses {
        writeln!(
            output,
            "{:<9}{:<14}{:<8}{:<8}{:<9}{}",
            row.input_address,
            row.line_address,
            row.byte_offset,
            row.target,
            row.local_line_address,
            row.local_byte_address
        )?;
    }
    Ok(())
}
