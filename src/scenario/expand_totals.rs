use crate::{
    input::limits::{
        MAX_CONCRETE_TESTS, MAX_REPORT_TARGET_ROWS, MAX_REPORT_WINDOW_ROWS,
        MAX_TOTAL_GENERATED_ACCESSES, MAX_WINDOW_WORK,
    },
    issue::{Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase},
    mapping::TargetCount,
};

use super::expand_prepare::PreparedCase;

pub(super) fn validate_global_totals(
    cases: &[PreparedCase<'_>],
    target_count: TargetCount,
) -> Result<(), Issue> {
    let concrete_tests = checked_sum(cases.iter().map(|case| Some(case.test_count)));
    ensure_limit(
        concrete_tests,
        GlobalLimit {
            maximum: u128::try_from(MAX_CONCRETE_TESTS).unwrap_or(u128::MAX),
            overflow_observed: 10_001,
            constraint: "integer <= 10000",
            field_order: 0,
        },
    )?;

    let total_accesses = checked_sum(
        cases
            .iter()
            .map(|case| case.test_count.checked_mul(u128::from(case.accesses.get()))),
    );
    ensure_limit(
        total_accesses,
        GlobalLimit {
            maximum: MAX_TOTAL_GENERATED_ACCESSES,
            overflow_observed: 100_000_001,
            constraint: "integer <= 100000000",
            field_order: 1,
        },
    )?;

    let target_rows =
        concrete_tests.and_then(|tests| tests.checked_mul(u128::from(target_count.get())));
    ensure_limit(
        target_rows,
        GlobalLimit {
            maximum: MAX_REPORT_TARGET_ROWS,
            overflow_observed: 1_000_001,
            constraint: "integer <= 1000000",
            field_order: 2,
        },
    )?;

    let window_rows = checked_sum(cases.iter().map(|case| {
        u128::try_from(case.windows.len())
            .ok()
            .and_then(|windows| case.test_count.checked_mul(windows))
    }));
    ensure_limit(
        window_rows,
        GlobalLimit {
            maximum: MAX_REPORT_WINDOW_ROWS,
            overflow_observed: 1_000_001,
            constraint: "integer <= 1000000",
            field_order: 3,
        },
    )?;

    let window_work = checked_sum(cases.iter().map(|case| {
        u128::try_from(case.windows.len()).ok().and_then(|windows| {
            case.test_count
                .checked_mul(u128::from(case.accesses.get()))
                .and_then(|accesses| accesses.checked_mul(windows))
        })
    }));
    ensure_limit(
        window_work,
        GlobalLimit {
            maximum: MAX_WINDOW_WORK,
            overflow_observed: 100_000_001,
            constraint: "sum(Q*K) <= 100000000",
            field_order: 4,
        },
    )
}

fn checked_sum(values: impl IntoIterator<Item = Option<u128>>) -> Option<u128> {
    values
        .into_iter()
        .try_fold(0_u128, |total, value| total.checked_add(value?))
}

#[derive(Clone, Copy)]
struct GlobalLimit {
    maximum: u128,
    overflow_observed: u128,
    constraint: &'static str,
    field_order: usize,
}

fn ensure_limit(observed: Option<u128>, limit: GlobalLimit) -> Result<(), Issue> {
    let observed = observed.unwrap_or(limit.overflow_observed);
    if observed <= limit.maximum {
        Ok(())
    } else {
        Err(Issue::new(
            IssueCode::ScenarioInvalid,
            IssuePath::root().field("cases"),
            format!("expected {}, observed {observed}", limit.constraint),
        )
        .with_order(IssueOrderKey::new(
            IssuePhase::Expansion,
            0,
            limit.field_order,
        )))
    }
}
