use crate::{
    input::{
        limits::{MAX_ACCESSES_PER_TEST, MAX_STREAMS_PER_CASE, MAX_WINDOW_SIZES_PER_CASE},
        scalar::Address,
    },
    issue::{Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase},
};

use super::{
    model::{AccessCount, ScenarioCaseKind, WindowSize},
    model_kind::StreamScenario,
    select::{SelectedCase, SelectedCases},
};

pub(super) enum PreparedKind<'scenario> {
    Linear {
        base: Address,
        stride: Address,
    },
    Sweep {
        bases: &'scenario [Address],
        strides: &'scenario [Address],
    },
    MultiStream {
        streams: &'scenario [StreamScenario],
    },
}

pub(super) struct PreparedCase<'scenario> {
    pub(super) source_index: usize,
    pub(super) name: &'scenario str,
    pub(super) windows: &'scenario [WindowSize],
    pub(super) accesses: AccessCount,
    pub(super) test_count: u128,
    pub(super) kind: PreparedKind<'scenario>,
}

pub(super) fn prepare_selected<'scenario>(
    selected: &SelectedCases<'scenario>,
) -> Result<Vec<PreparedCase<'scenario>>, Vec<Issue>> {
    let mut prepared = Vec::with_capacity(selected.cases().len());
    let mut issues = Vec::new();
    for selected_case in selected.cases() {
        match prepare_case(*selected_case, selected.defaults()) {
            Ok(case) => prepared.push(case),
            Err(mut case_issues) => issues.append(&mut case_issues),
        }
    }
    if issues.is_empty() {
        Ok(prepared)
    } else {
        Err(issues)
    }
}

fn prepare_case<'scenario>(
    selected: SelectedCase<'scenario>,
    defaults: &'scenario super::ScenarioDefaults,
) -> Result<PreparedCase<'scenario>, Vec<Issue>> {
    let case = selected.case();
    let source_index = selected.source_index();
    let location = SemanticLocation { source_index };
    let (windows, window_path) = effective_windows(selected, defaults);
    let mut issues = location.window_count_issues(windows, &window_path);
    let accesses = match access_count(case.kind(), defaults.accesses()) {
        Ok(accesses) => Some(accesses),
        Err(observed) => {
            issues.push(location.issue(
                IssuePath::root().field("cases").index(source_index),
                format!("expected integer <= {MAX_ACCESSES_PER_TEST}, observed {observed}"),
                3_000,
            ));
            None
        }
    };
    if windows.len() <= MAX_WINDOW_SIZES_PER_CASE
        && let Some(accesses) = accesses
    {
        issues.extend(location.window_range_issues(windows, &window_path, accesses));
    }
    let kind = prepared_kind(case.kind(), source_index, &mut issues);
    let Some(accesses) = accesses else {
        return Err(issues);
    };
    if !issues.is_empty() {
        return Err(issues);
    }
    let test_count = match &kind {
        PreparedKind::Sweep { bases, strides } => u128::try_from(bases.len())
            .ok()
            .and_then(|base_count| {
                u128::try_from(strides.len())
                    .ok()
                    .and_then(|stride_count| base_count.checked_mul(stride_count))
            })
            .unwrap_or(10_001),
        PreparedKind::Linear { .. } | PreparedKind::MultiStream { .. } => 1,
    };
    Ok(PreparedCase {
        source_index,
        name: case.name().as_str(),
        windows,
        accesses,
        test_count,
        kind,
    })
}

fn effective_windows<'scenario>(
    selected: SelectedCase<'scenario>,
    defaults: &'scenario super::ScenarioDefaults,
) -> (&'scenario [WindowSize], IssuePath) {
    selected.case().window_sizes().map_or_else(
        || {
            (
                defaults.window_sizes(),
                IssuePath::root().field("defaults").field("window_sizes"),
            )
        },
        |windows| {
            (
                windows,
                IssuePath::root()
                    .field("cases")
                    .index(selected.source_index())
                    .field("window_sizes"),
            )
        },
    )
}

fn prepared_kind<'scenario>(
    kind: &'scenario ScenarioCaseKind,
    source_index: usize,
    issues: &mut Vec<Issue>,
) -> PreparedKind<'scenario> {
    match kind {
        ScenarioCaseKind::Stride(value) => PreparedKind::Linear {
            base: value.base_bytes(),
            stride: value.stride_bytes(),
        },
        ScenarioCaseKind::Sweep(value) => PreparedKind::Sweep {
            bases: value.base_bytes(),
            strides: value.stride_bytes(),
        },
        ScenarioCaseKind::MultiStream(value) => {
            if value.streams().len() > MAX_STREAMS_PER_CASE {
                issues.push(
                    SemanticLocation { source_index }.issue(
                        IssuePath::root()
                            .field("cases")
                            .index(source_index)
                            .field("streams"),
                        format!(
                            "expected integer <= {MAX_STREAMS_PER_CASE}, observed {}",
                            value.streams().len()
                        ),
                        2_000,
                    ),
                );
            }
            PreparedKind::MultiStream {
                streams: value.streams(),
            }
        }
    }
}

fn access_count(kind: &ScenarioCaseKind, default: AccessCount) -> Result<AccessCount, u128> {
    match kind {
        ScenarioCaseKind::Stride(value) => Ok(value.accesses().unwrap_or(default)),
        ScenarioCaseKind::Sweep(value) => Ok(value.accesses().unwrap_or(default)),
        ScenarioCaseKind::MultiStream(value) => {
            let sum = value.streams().iter().try_fold(0_u128, |total, stream| {
                total.checked_add(u128::from(stream.accesses().get()))
            });
            let Some(sum) = sum else {
                return Err(MAX_ACCESSES_PER_TEST.saturating_add(1));
            };
            if sum > MAX_ACCESSES_PER_TEST {
                Err(sum)
            } else {
                u64::try_from(sum)
                    .map(AccessCount::new)
                    .map_err(|_| MAX_ACCESSES_PER_TEST.saturating_add(1))
            }
        }
    }
}

#[derive(Clone, Copy)]
struct SemanticLocation {
    source_index: usize,
}

impl SemanticLocation {
    fn issue(self, path: IssuePath, message: String, field_order: usize) -> Issue {
        Issue::new(IssueCode::ScenarioInvalid, path, message).with_order(IssueOrderKey::new(
            IssuePhase::ScenarioSemantic,
            self.source_index,
            field_order,
        ))
    }

    fn window_count_issues(self, windows: &[WindowSize], path: &IssuePath) -> Vec<Issue> {
        if windows.len() > MAX_WINDOW_SIZES_PER_CASE {
            vec![self.issue(
                path.clone(),
                format!(
                    "expected integer <= {MAX_WINDOW_SIZES_PER_CASE}, observed {}",
                    windows.len()
                ),
                0,
            )]
        } else {
            Vec::new()
        }
    }

    fn window_range_issues(
        self,
        windows: &[WindowSize],
        path: &IssuePath,
        accesses: AccessCount,
    ) -> Vec<Issue> {
        windows
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, window)| window.get() > accesses.get())
            .map(|(window_index, window)| {
                self.issue(
                    path.clone().index(window_index),
                    format!(
                        "expected integer <= {}, observed {}",
                        accesses.get(),
                        window.get()
                    ),
                    window_index.saturating_add(1),
                )
            })
            .collect()
    }
}
