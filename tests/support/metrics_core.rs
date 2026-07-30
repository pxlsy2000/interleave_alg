#[test]
fn counts_and_max_load_use_exact_unreduced_formulas() -> MetricsTestResult {
    // Given
    let targets = [0, 0, 1, 2, 3];

    // When
    let metrics = analyze_fixture(&targets, 8, &[1])?;

    // Then
    assert_eq!(
        metrics
            .targets()
            .iter()
            .map(|row| (
                row.target(),
                row.count(),
                row.share().numerator(),
                row.share().denominator()
            ))
            .collect::<Vec<_>>(),
        [
            (0, 2, 2, 5),
            (1, 1, 1, 5),
            (2, 1, 1, 5),
            (3, 1, 1, 5),
            (4, 0, 0, 5),
            (5, 0, 0, 5),
            (6, 0, 0, 5),
            (7, 0, 0, 5),
        ]
    );
    assert_eq!(metrics.max_load().target(), 0);
    assert_eq!(metrics.max_load().count(), 2);
    assert_eq!(ratio_tuple(metrics.max_load().ratio()), (16, 5));
    Ok(())
}

#[test]
fn max_load_tie_uses_smallest_target() -> MetricsTestResult {
    // Given
    let targets = [1, 1, 0, 0];

    // When
    let metrics = analyze_fixture(&targets, 4, &[1])?;

    // Then
    assert_eq!(metrics.max_load().target(), 0);
    assert_eq!(metrics.max_load().count(), 2);
    Ok(())
}

#[test]
fn windows_use_earliest_start_before_smallest_target() -> MetricsTestResult {
    // Given
    let targets = [1, 1, 0, 0];

    // When
    let metrics = analyze_fixture(&targets, 4, &[2, 4])?;

    // Then
    let rows = metrics
        .windows()
        .iter()
        .map(|row| {
            (
                row.size(),
                row.target(),
                row.start_index(),
                row.count(),
                ratio_tuple(row.ratio()),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, [(2, 1, 0, 2, (8, 2)), (4, 0, 0, 2, (8, 4))]);
    Ok(())
}

#[test]
fn same_counts_in_different_order_change_windows_and_runs() -> MetricsTestResult {
    // Given
    let grouped_targets = [0, 0, 1, 1];
    let alternating_targets = [0, 1, 0, 1];

    // When
    let grouped = analyze_fixture(&grouped_targets, 2, &[2])?;
    let alternating = analyze_fixture(&alternating_targets, 2, &[2])?;

    // Then
    assert_eq!(grouped.targets(), alternating.targets());
    assert_eq!(grouped.windows().first().map(|row| row.count()), Some(2));
    assert_eq!(
        alternating.windows().first().map(|row| row.count()),
        Some(1)
    );
    assert_eq!(grouped.longest_run().length(), 2);
    assert_eq!(alternating.longest_run().length(), 1);
    Ok(())
}

#[test]
fn longest_run_tie_uses_smallest_start_not_smallest_target() -> MetricsTestResult {
    // Given
    let targets = [2, 2, 1, 1];

    // When
    let metrics = analyze_fixture(&targets, 4, &[1])?;

    // Then
    assert_eq!(
        (
            metrics.longest_run().length(),
            metrics.longest_run().target(),
            metrics.longest_run().start_index()
        ),
        (2, 2, 0)
    );
    Ok(())
}

#[test]
fn window_visits_equal_q_times_declared_window_count() -> MetricsTestResult {
    // Given
    let targets = [0, 1, 2, 3, 0, 1, 2];

    // When
    let metrics = analyze_fixture(&targets, 4, &[1, 3, 7])?;

    // Then
    assert_eq!(metrics.window_target_visits(), 21);
    Ok(())
}

#[test]
fn aligned_and_staggered_stream_metrics_match_hand_calculated_goldens() -> MetricsTestResult {
    // Given
    let aligned_targets = [0, 0, 0, 0];
    let staggered_targets = [0, 1, 0, 1];

    // When
    let aligned = analyze_fixture(&aligned_targets, 2, &[2])?;
    let staggered = analyze_fixture(&staggered_targets, 2, &[2])?;

    // Then
    assert_eq!(ratio_tuple(aligned.max_load().ratio()), (8, 4));
    assert_eq!(ratio_tuple(staggered.max_load().ratio()), (4, 4));
    assert_eq!(
        aligned.windows().first().map(|row| (row.count(), ratio_tuple(row.ratio()))),
        Some((2, (4, 2)))
    );
    assert_eq!(
        staggered.windows().first().map(|row| (row.count(), ratio_tuple(row.ratio()))),
        Some((1, (2, 2)))
    );
    assert_eq!(aligned.longest_run().length(), 4);
    assert_eq!(staggered.longest_run().length(), 1);
    Ok(())
}
