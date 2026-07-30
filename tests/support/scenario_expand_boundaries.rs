#[test]
fn global_test_count_is_the_first_total_failure() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(16, 128)?;
    let bases = (0..10_001_u32)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!(
        "schema_version: 1\ndefaults: {{ accesses: 10000000, window_sizes: [1] }}\n\
         cases:\n  - name: too-many\n    kind: sweep\n    base_bytes: [{bases}]\n\
         \x20   stride_bytes: [0]\n"
    );
    let scenario = decode_expand_scenario(&source)?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error = preflight_scenarios(&mapping, &selected)
        .expect_err("first global cap must stop later totals");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases|expected integer <= 10000, observed 10001"]
    );
    Ok(())
}

#[test]
fn stream_sum_and_stream_count_use_selected_case_paths() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(64, 4)?;
    let mut streams = String::new();
    for index in 0..4_097_u32 {
        writeln!(
            streams,
            "      - {{ name: s{index}, base_bytes: 0, stride_bytes: 0, accesses: 1 }}"
        )
        .map_err(|error| error.to_string())?;
    }
    let source = format!(
        "schema_version: 1\ndefaults: {{ accesses: 1, window_sizes: [1] }}\ncases:\n\
         \x20 - name: many\n    kind: multi_stream\n    schedule: round_robin\n    streams:\n{streams}"
    );
    let scenario = decode_expand_scenario(&source)?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error = preflight_scenarios(&mapping, &selected)
        .expect_err("stream count over cap must fail");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases[0].streams|expected integer <= 4096, observed 4097"]
    );
    Ok(())
}

#[test]
fn multi_stream_sum_at_limit_passes_and_plus_one_fails() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(64, 4)?;
    let at_limit = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [1] }\ncases:\n\
         \x20 - name: exact\n    kind: multi_stream\n    schedule: round_robin\n    streams:\n\
         \x20     - { name: a, base_bytes: 0, stride_bytes: 0, accesses: 5000000 }\n\
         \x20     - { name: b, base_bytes: 0, stride_bytes: 0, accesses: 5000000 }\n",
    )?;
    let over = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [1] }\ncases:\n\
         \x20 - name: over\n    kind: multi_stream\n    schedule: round_robin\n    streams:\n\
         \x20     - { name: a, base_bytes: 0, stride_bytes: 0, accesses: 5000000 }\n\
         \x20     - { name: b, base_bytes: 0, stride_bytes: 0, accesses: 5000001 }\n",
    )?;
    let at_limit_selected =
        select_cases(&at_limit, &[]).map_err(|error| error.to_string())?;
    let over_selected = select_cases(&over, &[]).map_err(|error| error.to_string())?;

    // When
    let plan = preflight_scenarios(&mapping, &at_limit_selected)
        .map_err(|error| error.to_string())?;
    let error =
        preflight_scenarios(&mapping, &over_selected).expect_err("Q cap + 1 must fail");

    // Then
    assert_eq!(concrete_test(&plan, 0)?.accesses(), 10_000_000);
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases[0]|expected integer <= 10000000, observed 10000001"]
    );
    Ok(())
}
use std::fmt::Write as _;
