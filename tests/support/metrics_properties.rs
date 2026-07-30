use proptest::prelude::*;

fn assert_matches_oracle(
    targets: &[u16],
    target_count: u32,
    window_values: &[u64],
) -> MetricsTestResult {
    let metrics = analyze_fixture(targets, target_count, window_values)?;
    let counts = oracle_counts(targets, target_count);
    assert_eq!(
        metrics.targets().iter().map(|row| row.count()).collect::<Vec<_>>(),
        counts
    );
    assert_eq!(
        metrics.targets().iter().map(|row| row.count()).sum::<u64>(),
        u64::try_from(targets.len()).map_err(|error| error.to_string())?
    );
    let maximum_count = counts.iter().copied().max().unwrap_or_default();
    let maximum_target = counts
        .iter()
        .position(|count| *count == maximum_count)
        .ok_or_else(|| "oracle count vector must not be empty".to_owned())?;
    assert_eq!(
        (
            usize::from(metrics.max_load().target()),
            metrics.max_load().count(),
            ratio_tuple(metrics.max_load().ratio()),
        ),
        (
            maximum_target,
            maximum_count,
            (
                u128::from(target_count) * u128::from(maximum_count),
                u128::try_from(targets.len()).map_err(|error| error.to_string())?,
            ),
        )
    );
    let expected_windows = window_values
        .iter()
        .map(|size| oracle_window(targets, target_count, *size))
        .collect::<Vec<_>>();
    let actual_windows = metrics
        .windows()
        .iter()
        .map(|row| OracleWindow {
            size: row.size(),
            target: row.target(),
            start: row.start_index(),
            count: row.count(),
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_windows, expected_windows);
    for (actual, expected) in metrics.windows().iter().zip(&expected_windows) {
        assert_eq!(
            ratio_tuple(actual.ratio()),
            (
                u128::from(target_count) * u128::from(expected.count),
                u128::from(expected.size),
            )
        );
    }
    assert_eq!(
        OracleRun {
            length: metrics.longest_run().length(),
            target: metrics.longest_run().target(),
            start: metrics.longest_run().start_index(),
        },
        oracle_run(targets)
    );
    assert_eq!(
        metrics.window_target_visits(),
        u128::try_from(targets.len()).map_err(|error| error.to_string())?
            * u128::try_from(window_values.len()).map_err(|error| error.to_string())?
    );
    Ok(())
}

#[test]
fn exhaustive_small_sequences_match_simple_oracle() -> MetricsTestResult {
    // Given
    let target_count = 2_u32;

    for q in 1_u32..=6 {
        let sequence_count = target_count.pow(q);
        for encoded in 0..sequence_count {
            let mut value = encoded;
            let targets = (0..q)
                .map(|_| {
                    let target = u16::try_from(value % target_count).unwrap_or(u16::MAX);
                    value /= target_count;
                    target
                })
                .collect::<Vec<_>>();
            let windows = (1..=u64::from(q)).collect::<Vec<_>>();

            // When / Then
            assert_matches_oracle(&targets, target_count, &windows)?;
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn arbitrary_small_sequences_match_simple_oracle(
        target_count_log2 in 1_u32..=3,
        raw_targets in prop::collection::vec(0_u16..8, 1..16),
    ) {
        // Given
        let target_count = 1_u32 << target_count_log2;
        let targets = raw_targets
            .into_iter()
            .map(|target| target % u16::try_from(target_count).unwrap_or(u16::MAX))
            .collect::<Vec<_>>();
        let q = u64::try_from(targets.len()).unwrap_or(u64::MAX);
        let mut windows = Vec::new();
        for size in [1, q.div_ceil(2), q] {
            if !windows.contains(&size) {
                windows.push(size);
            }
        }

        // When / Then
        prop_assert!(assert_matches_oracle(&targets, target_count, &windows).is_ok());
    }
}
