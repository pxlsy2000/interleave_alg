#[test]
fn one_stream_round_robin_matches_its_linear_sequence() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - name: solo\n    kind: multi_stream\n    schedule: round_robin\n    window_sizes: [1]\n    streams:\n      - { name: A, base_bytes: 2, stride_bytes: 1, accesses: 3 }\n",
    )?;

    // When
    let targets = generated_at(&mapping, &plan, 0)?;

    // Then
    assert_eq!(targets, [2, 3, 0]);
    Ok(())
}

#[test]
fn unequal_streams_use_active_declaration_order_each_round() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - name: documented\n    kind: multi_stream\n    schedule: round_robin\n    window_sizes: [1]\n    streams:\n      - { name: A, base_bytes: 0, stride_bytes: 1, accesses: 3 }\n      - { name: B, base_bytes: 4, stride_bytes: 1, accesses: 2 }\n",
    )?;

    // When
    let targets = generated_at(&mapping, &plan, 0)?;

    // Then
    assert_eq!(targets, [0, 0, 1, 1, 2]);
    Ok(())
}

#[test]
fn aligned_and_staggered_streams_keep_their_phase() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - name: phase\n    kind: multi_stream\n    schedule: round_robin\n    window_sizes: [1]\n    streams:\n      - { name: aligned, base_bytes: 0, stride_bytes: 4, accesses: 3 }\n      - { name: staggered, base_bytes: 1, stride_bytes: 4, accesses: 3 }\n",
    )?;

    // When
    let targets = generated_at(&mapping, &plan, 0)?;

    // Then
    assert_eq!(targets, [0, 1, 0, 1, 0, 1]);
    Ok(())
}

#[test]
fn aligned_and_staggered_multistream_generation_produces_hand_calculated_metrics(
) -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - name: aligned\n    kind: multi_stream\n    schedule: round_robin\n    window_sizes: [2]\n    streams:\n      - { name: A, base_bytes: 0, stride_bytes: 4, accesses: 2 }\n      - { name: B, base_bytes: 0, stride_bytes: 4, accesses: 2 }\n  - name: staggered\n    kind: multi_stream\n    schedule: round_robin\n    window_sizes: [2]\n    streams:\n      - { name: A, base_bytes: 0, stride_bytes: 4, accesses: 2 }\n      - { name: B, base_bytes: 1, stride_bytes: 4, accesses: 2 }\n",
    )?;

    // When
    let generated = [generated_at(&mapping, &plan, 0)?, generated_at(&mapping, &plan, 1)?];
    let metrics = generated
        .iter()
        .zip(plan.tests())
        .map(|(targets, descriptor)| {
            analyze_targets(
                targets,
                mapping.target_count(),
                descriptor.window_sizes(),
            )
            .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Then
    assert_eq!(generated, [vec![0, 0, 0, 0], vec![0, 1, 0, 1]]);
    assert_eq!(
        metrics
            .iter()
            .map(|metric| (
                (
                    metric.max_load().ratio().numerator(),
                    metric.max_load().ratio().denominator(),
                ),
                metric.windows().first().map(|window| (
                    window.count(),
                    window.ratio().numerator(),
                    window.ratio().denominator(),
                )),
                metric.longest_run().length(),
            ))
            .collect::<Vec<_>>(),
        [
            ((16, 4), Some((2, 8, 2)), 4),
            ((8, 4), Some((1, 4, 2)), 1),
        ]
    );
    Ok(())
}

#[test]
fn generated_length_equals_preflight_total_q() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - name: total\n    kind: multi_stream\n    schedule: round_robin\n    window_sizes: [1]\n    streams:\n      - { name: A, base_bytes: 0, stride_bytes: 1, accesses: 1 }\n      - { name: B, base_bytes: 0, stride_bytes: 1, accesses: 3 }\n      - { name: C, base_bytes: 0, stride_bytes: 1, accesses: 2 }\n",
    )?;
    let descriptor = plan
        .tests()
        .first()
        .ok_or_else(|| "missing total descriptor".to_owned())?;

    // When
    let targets = generated_at(&mapping, &plan, 0)?;

    // Then
    assert_eq!(targets.len(), usize::try_from(descriptor.accesses()).map_err(|e| e.to_string())?);
    assert_eq!(targets.len(), 6);
    Ok(())
}
