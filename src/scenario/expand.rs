use thiserror::Error;

use crate::{
    error::ExitClass,
    input::scalar::{Address, AddressWidth},
    issue::{Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase, sort_issues},
    mapping::MappingModel,
};

use super::{
    expand_model::{
        ConcreteStimulus, ConcreteTestDescriptor, LinearAccess, ScenarioPreflightPlan, StreamAccess,
    },
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

    /// Returns Scenario or generated-address failure exit class 3.
    pub const fn exit_class(&self) -> ExitClass {
        ExitClass::ScenarioOrAddress
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
    let address_issues = validate_addresses(&prepared, mapping.address_width());
    if !address_issues.is_empty() {
        return Err(preflight_error(address_issues));
    }
    Ok(ScenarioPreflightPlan::new(build_descriptors(&prepared)))
}

fn validate_addresses(cases: &[PreparedCase<'_>], width: AddressWidth) -> Vec<Issue> {
    let mut issues = Vec::new();
    for case in cases {
        let case_path = IssuePath::root().field("cases").index(case.source_index);
        match &case.kind {
            PreparedKind::Linear { base, stride } => {
                push_address_issue(
                    &mut issues,
                    AddressCheck {
                        base: *base,
                        stride: *stride,
                        accesses: case.accesses,
                        width,
                        path: case_path,
                        source_order: case.source_index,
                        field_order: 0,
                    },
                );
            }
            PreparedKind::Sweep { bases, strides } => {
                for (base_index, base) in bases.iter().copied().enumerate() {
                    for (stride_index, stride) in strides.iter().copied().enumerate() {
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
                        );
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
                    );
                }
            }
        }
    }
    sort_issues(&mut issues);
    issues
}

struct AddressCheck {
    base: Address,
    stride: Address,
    accesses: super::AccessCount,
    width: AddressWidth,
    path: IssuePath,
    source_order: usize,
    field_order: usize,
}

fn push_address_issue(issues: &mut Vec<Issue>, check: AddressCheck) {
    let last = u128::from(check.accesses.get())
        .checked_sub(1)
        .and_then(|index| check.stride.get().checked_mul(index))
        .and_then(|offset| check.base.get().checked_add(offset));
    let bound = check.width.exclusive_bound();
    let observed = match last {
        Some(last) if last < bound => return,
        Some(last) => last,
        None => bound,
    };
    issues.push(
        Issue::new(
            IssueCode::AddressOutOfRange,
            check.path,
            format!(
                "address 0x{observed:x} is outside the {}-bit range",
                check.width.get()
            ),
        )
        .with_order(IssueOrderKey::new(
            IssuePhase::Address,
            check.source_order,
            check.field_order,
        )),
    );
}

fn build_descriptors(cases: &[PreparedCase<'_>]) -> Vec<ConcreteTestDescriptor> {
    let mut tests = Vec::new();
    for case in cases {
        match &case.kind {
            PreparedKind::Linear { base, stride } => tests.push(ConcreteTestDescriptor {
                case_id: case.name.to_owned(),
                source_case: case.name.to_owned(),
                accesses: case.accesses,
                window_sizes: case.windows.to_vec(),
                stimulus: ConcreteStimulus::Linear(LinearAccess::new(
                    *base,
                    *stride,
                    case.accesses,
                )),
            }),
            PreparedKind::Sweep { bases, strides } => {
                for base in *bases {
                    for stride in *strides {
                        tests.push(ConcreteTestDescriptor {
                            case_id: format!(
                                "{}[base={},stride={}]",
                                case.name,
                                base.canonical(),
                                stride.canonical()
                            ),
                            source_case: case.name.to_owned(),
                            accesses: case.accesses,
                            window_sizes: case.windows.to_vec(),
                            stimulus: ConcreteStimulus::Linear(LinearAccess::new(
                                *base,
                                *stride,
                                case.accesses,
                            )),
                        });
                    }
                }
            }
            PreparedKind::MultiStream { streams } => {
                let stream_descriptors = streams
                    .iter()
                    .map(|stream| {
                        StreamAccess::new(
                            stream.name().clone(),
                            LinearAccess::new(
                                stream.base_bytes(),
                                stream.stride_bytes(),
                                stream.accesses(),
                            ),
                        )
                    })
                    .collect();
                tests.push(ConcreteTestDescriptor {
                    case_id: case.name.to_owned(),
                    source_case: case.name.to_owned(),
                    accesses: case.accesses,
                    window_sizes: case.windows.to_vec(),
                    stimulus: ConcreteStimulus::RoundRobin(stream_descriptors),
                });
            }
        }
    }
    tests
}

fn preflight_error(mut issues: Vec<Issue>) -> ScenarioPreflightError {
    sort_issues(&mut issues);
    ScenarioPreflightError { issues }
}
