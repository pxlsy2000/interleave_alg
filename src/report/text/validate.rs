use std::fmt::{self, Write};

use crate::{
    issue::CheckStatus,
    mapping::{MappingCheck, MappingCheckObservation},
};

use super::{
    super::{ReportIssue, ValidateResult, model_mapping::MappingMatrixReport},
    TextReportStyle,
    format::{render_issues, safe_field},
};

pub(super) fn render(
    output: &mut String,
    result: &ValidateResult,
    warnings: &[ReportIssue],
    errors: &[ReportIssue],
    style: TextReportStyle,
) -> fmt::Result {
    writeln!(output, "Mapping: {}", safe_field(&result.mapping_name))?;
    writeln!(output, "Input: {}", safe_field(&result.input))?;
    let derived = result.derived;
    writeln!(
        output,
        "Derived: A={}, G={}, g={}, n={}, N={}, r={}, s={}",
        derived.address_width_bits,
        derived.granule_bytes,
        derived.offset_bits,
        derived.line_bits,
        derived.target_count,
        derived.target_bits,
        derived.local_address_bits
    )?;
    writeln!(output)?;
    if style == TextReportStyle::Verbose {
        render_matrices(output, &result.matrices)?;
    }
    writeln!(output, "PASS  input structure")?;
    for check in &result.checks {
        render_check(output, check)?;
    }
    writeln!(output)?;
    writeln!(output, "Result: {}", result.classification.as_str())?;
    render_issues(output, "WARNING", warnings)?;
    render_issues(output, "ERROR", errors)
}

fn render_matrices(output: &mut String, matrices: &MappingMatrixReport) -> fmt::Result {
    render_labeled_matrix(output, "M", "t", &matrices.target, matrices.line_bits)?;
    render_labeled_matrix(output, "L", "l", &matrices.local, matrices.line_bits)?;
    let rows = combined_rows(matrices.target.len(), matrices.local.len());
    writeln!(
        output,
        "F ({} x {}; rows {rows}; {})",
        matrices.combined.len(),
        matrices.line_bits,
        columns(matrices.line_bits)
    )?;
    render_unlabeled_rows(output, &matrices.combined)?;
    writeln!(output)?;
    render_labeled_matrix(
        output,
        "Mp",
        "t",
        &matrices.target_low,
        matrices.target_bits,
    )
}

fn render_labeled_matrix(
    output: &mut String,
    name: &str,
    label: &str,
    rows: &[Vec<bool>],
    columns_count: usize,
) -> fmt::Result {
    writeln!(
        output,
        "{name} ({} x {columns_count}; {})",
        rows.len(),
        columns(columns_count)
    )?;
    for (index, row) in rows.iter().enumerate() {
        write!(output, "  {label}{index:<3} ")?;
        render_bits(output, row);
    }
    writeln!(output)
}

fn render_unlabeled_rows(output: &mut String, rows: &[Vec<bool>]) -> fmt::Result {
    for row in rows {
        write!(output, "  ")?;
        render_bits(output, row);
    }
    Ok(())
}

fn render_bits(output: &mut String, row: &[bool]) {
    for (index, bit) in row.iter().enumerate() {
        if index != 0 {
            output.push(' ');
        }
        output.push(if *bit { '1' } else { '0' });
    }
    output.push('\n');
}

fn columns(count: usize) -> String {
    count.checked_sub(1).map_or_else(
        || "columns none".to_owned(),
        |last| format!("columns x0..x{last}"),
    )
}

fn combined_rows(targets: usize, locals: usize) -> String {
    let mut labels: Vec<String> = (0..targets).map(|index| format!("t{index}")).collect();
    match locals {
        0 => {}
        1 => labels.push("l0".to_owned()),
        count => labels.push(format!("l0..l{}", count - 1)),
    }
    if labels.is_empty() {
        "none".to_owned()
    } else {
        labels.join(",")
    }
}

fn render_check(output: &mut String, check: &MappingCheck) -> fmt::Result {
    let prefix = match check.status() {
        CheckStatus::Pass => "PASS",
        CheckStatus::Warning => "WARN",
        CheckStatus::Fail => "FAIL",
    };
    match (check.observed(), check.expected()) {
        (
            MappingCheckObservation::TargetReachable { rank_m: actual },
            MappingCheckObservation::TargetReachable { rank_m: expected },
        ) => writeln!(
            output,
            "{prefix}  target reachable: rank(M)={actual}, expected {expected}"
        ),
        (
            MappingCheckObservation::Bijective { rank_f: actual },
            MappingCheckObservation::Bijective { rank_f: expected },
        ) => writeln!(
            output,
            "{prefix}  bijective: rank(F)={actual}, expected {expected}"
        ),
        (
            MappingCheckObservation::NaturalLocalAddress {
                rank_m_low: actual,
                l_matches_preserve_high,
            },
            MappingCheckObservation::NaturalLocalAddress {
                rank_m_low: expected,
                l_matches_preserve_high: _,
            },
        ) => render_natural_check(output, prefix, *actual, *expected, *l_matches_preserve_high),
        (
            MappingCheckObservation::TargetReachable { .. },
            MappingCheckObservation::Bijective { .. }
            | MappingCheckObservation::NaturalLocalAddress { .. },
        )
        | (
            MappingCheckObservation::Bijective { .. },
            MappingCheckObservation::TargetReachable { .. }
            | MappingCheckObservation::NaturalLocalAddress { .. },
        )
        | (
            MappingCheckObservation::NaturalLocalAddress { .. },
            MappingCheckObservation::TargetReachable { .. }
            | MappingCheckObservation::Bijective { .. },
        ) => Err(fmt::Error),
    }
}

fn render_natural_check(
    output: &mut String,
    prefix: &str,
    actual: u8,
    expected: u8,
    l_matches: bool,
) -> fmt::Result {
    match (actual == expected, l_matches) {
        (true, true) => writeln!(
            output,
            "{prefix}  natural LA: rank(Mp)={actual} and L=[0 I]"
        ),
        (true, false) => writeln!(
            output,
            "{prefix}  natural LA: rank(Mp)={actual}, but L != [0 I]"
        ),
        (false, true) => writeln!(
            output,
            "{prefix}  natural LA: rank(Mp)={actual}, expected {expected}"
        ),
        (false, false) => writeln!(
            output,
            "{prefix}  natural LA: rank(Mp)={actual}, expected {expected}, and L != [0 I]"
        ),
    }
}
