use std::fmt::{self, Write};

use super::{
    super::{ReportIssue, RunCaseResult, RunResult},
    format::{input_name, render_issues, safe_field},
};

pub(super) fn render(
    output: &mut String,
    result: &RunResult,
    warnings: &[ReportIssue],
) -> fmt::Result {
    writeln!(output, "Mapping: {}", safe_field(&result.mapping_name))?;
    writeln!(output, "Input: {}", input_name(result.input.as_deref()))?;
    writeln!(output, "Result: {}", result.mapping_classification.as_str())?;
    for case in &result.cases {
        writeln!(output)?;
        render_case(output, case, warnings)?;
    }
    Ok(())
}

fn render_case(output: &mut String, case: &RunCaseResult, warnings: &[ReportIssue]) -> fmt::Result {
    writeln!(output, "Case: {}", safe_field(&case.case_id))?;
    writeln!(output, "Source case: {}", safe_field(&case.source_case))?;
    writeln!(output, "Accesses: {}", case.accesses.get())?;
    writeln!(output)?;
    writeln!(output, "Targets")?;
    writeln!(output, "{:<8}{:<7}Share", "Target", "Count")?;
    for row in &case.targets {
        writeln!(
            output,
            "{:<8}{:<7}{}",
            row.target,
            row.count.get(),
            row.share.decimal
        )?;
    }
    writeln!(output)?;
    writeln!(output, "Max load")?;
    writeln!(output, "{:<8}{:<7}Ratio", "Target", "Count")?;
    writeln!(
        output,
        "{:<8}{:<7}{}",
        case.max_load.target,
        case.max_load.count.get(),
        case.max_load.ratio.decimal
    )?;
    writeln!(output)?;
    writeln!(output, "Short-term windows")?;
    writeln!(
        output,
        "{:<6}{:<8}{:<13}{:<7}Ratio",
        "Size", "Target", "Start index", "Count"
    )?;
    for row in &case.windows {
        writeln!(
            output,
            "{:<6}{:<8}{:<13}{:<7}{}",
            row.size.get(),
            row.target,
            row.start_index.get(),
            row.count.get(),
            row.ratio.decimal
        )?;
    }
    writeln!(output)?;
    writeln!(output, "Longest run")?;
    writeln!(output, "{:<8}{:<8}Start index", "Length", "Target")?;
    writeln!(
        output,
        "{:<8}{:<8}{}",
        case.longest_run.length.get(),
        case.longest_run.target,
        case.longest_run.start_index.get()
    )?;
    writeln!(output)?;
    if warnings.is_empty() {
        writeln!(output, "Warnings: none")
    } else {
        writeln!(output, "Warnings:")?;
        render_issues(output, "WARNING", warnings)
    }
}
