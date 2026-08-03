use thiserror::Error;

use crate::{
    error::ExitClass,
    input::scalar::{AddressMagnitude, AddressWidth},
    issue::{Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase, sort_issues},
    mapping::MappingModel,
};

use super::{
    expand_descriptors::build_descriptors,
    expand_model::ScenarioPreflightPlan,
    expand_prepare::{PreparedCase, PreparedKind, prepare_selected},
    expand_totals::validate_global_totals,
    select::SelectedCases,
};

/// A deterministic selected-Scenario preflight failure.
#[derive(Debug, Error)]
#[error("Scenario preflight failed")]
pub struct ScenarioPreflightError {
    issues: Vec<Issue>,
}

impl ScenarioPreflightError {
    /// Returns issues in normative phase, declaration, and field order.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Returns analysis class 4 only for bounded-infrastructure failure.
    pub fn exit_class(&self) -> ExitClass {
        if self
            .issues
            .iter()
            .any(|issue| issue.code() == IssueCode::AnalysisFailed)
        {
            ExitClass::Analysis
        } else {
            ExitClass::ScenarioOrAddress
        }
    }
}

/// Resolves inheritance and creates a complete checked descriptor-only run plan.
pub fn preflight_scenarios(
    mapping: &MappingModel,
    selected: &SelectedCases<'_>,
) -> Result<ScenarioPreflightPlan, ScenarioPreflightError> {
    let prepared = prepare_selected(selected).map_err(preflight_error)?;
    validate_global_totals(&prepared, mapping.target_count())
        .map_err(|issue| preflight_error(vec![issue]))?;
    let address_issues = validate_addresses(&prepared, mapping.address_width())
        .map_err(|issue| preflight_error(vec![issue]))?;
    if !address_issues.is_empty() {
        return Err(preflight_error(address_issues));
    }
    let descriptors = build_descriptors(&prepared, mapping.address_width())
        .map_err(|issue| preflight_error(vec![issue]))?;
    Ok(ScenarioPreflightPlan::new(descriptors))
}

fn validate_addresses(
    cases: &[PreparedCase<'_>],
    width: AddressWidth,
) -> Result<Vec<Issue>, Issue> {
    let mut issues = Vec::new();
    for case in cases {
        let case_path = IssuePath::root().field("cases").index(case.source_index);
        match &case.kind {
            PreparedKind::Linear { base, stride } => {
                push_address_issue(
                    &mut issues,
                    AddressCheck {
                        base,
                        stride,
                        accesses: case.accesses,
                        width,
                        path: case_path,
                        source_order: case.source_index,
                        field_order: 0,
                    },
                )?;
            }
            PreparedKind::Sweep { bases, strides } => {
                for (base_index, base) in bases.iter().enumerate() {
                    for (stride_index, stride) in strides.iter().enumerate() {
                        let field_order = base_index
                            .checked_mul(strides.len())
                            .and_then(|offset| offset.checked_add(stride_index))
                            .unwrap_or(usize::MAX);
                        push_address_issue(
                            &mut issues,
                            AddressCheck {
                                base,
                                stride,
                                accesses: case.accesses,
                                width,
                                path: case_path.clone(),
                                source_order: case.source_index,
                                field_order,
                            },
                        )?;
                    }
                }
            }
            PreparedKind::MultiStream { streams } => {
                for (stream_index, stream) in streams.iter().enumerate() {
                    push_address_issue(
                        &mut issues,
                        AddressCheck {
                            base: stream.base_bytes(),
                            stride: stream.stride_bytes(),
                            accesses: stream.accesses(),
                            width,
                            path: case_path.clone().field("streams").index(stream_index),
                            source_order: case.source_index,
                            field_order: stream_index,
                        },
                    )?;
                }
            }
        }
    }
    sort_issues(&mut issues);
    Ok(issues)
}

struct AddressCheck<'magnitude> {
    base: &'magnitude AddressMagnitude,
    stride: &'magnitude AddressMagnitude,
    accesses: super::AccessCount,
    width: AddressWidth,
    path: IssuePath,
    source_order: usize,
    field_order: usize,
}

fn push_address_issue(issues: &mut Vec<Issue>, check: AddressCheck<'_>) -> Result<(), Issue> {
    let last = check
        .base
        .checked_last(check.stride, check.accesses.get())
        .map_err(|_| analysis_issue())?;
    if last.in_width(check.width).is_some() {
        return Ok(());
    }
    issues.push(
        Issue::new(
            IssueCode::AddressOutOfRange,
            check.path,
            format!(
                "address {} is outside the {}-bit range",
                last.canonical(),
                check.width.get()
            ),
        )
        .with_order(IssueOrderKey::new(
            IssuePhase::Address,
            check.source_order,
            check.field_order,
        )),
    );
    Ok(())
}

pub(super) fn analysis_issue() -> Issue {
    Issue::new(
        IssueCode::AnalysisFailed,
        IssuePath::root(),
        "analysis could not be completed",
    )
}

fn preflight_error(mut issues: Vec<Issue>) -> ScenarioPreflightError {
    sort_issues(&mut issues);
    ScenarioPreflightError { issues }
}
