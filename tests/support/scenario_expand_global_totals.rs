fn integer_list(end_exclusive: u32) -> String {
    (0..end_exclusive)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn sweep_source(bases: u32, accesses: u64, windows: &str) -> String {
    let bases = integer_list(bases);
    format!(
        "schema_version: 1\ndefaults: {{ accesses: {accesses}, window_sizes: [{windows}] }}\n\
         cases:\n  - name: total\n    kind: sweep\n    base_bytes: [{bases}]\n\
         \x20   stride_bytes: [0]\n"
    )
}

#[test]
fn concrete_test_count_at_limit_passes() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(16, 1)?;
    let scenario = decode_expand_scenario(&sweep_source(10_000, 1, "1"))?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let plan =
        preflight_scenarios(&mapping, &selected).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(plan.tests().len(), 10_000);
    Ok(())
}

#[test]
fn total_accesses_precedes_later_global_failures() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(32, 2)?;
    let scenario = decode_expand_scenario(&sweep_source(11, 10_000_000, "1"))?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error =
        preflight_scenarios(&mapping, &selected).expect_err("total Q must fail first");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases|expected integer <= 100000000, observed 110000000"]
    );
    Ok(())
}

#[test]
fn target_rows_precede_window_rows_and_work() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(32, 65_536)?;
    let scenario = decode_expand_scenario(&sweep_source(16, 1, "1"))?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error =
        preflight_scenarios(&mapping, &selected).expect_err("Target rows must fail first");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases|expected integer <= 1000000, observed 1048576"]
    );
    Ok(())
}

#[test]
fn largest_feasible_target_row_value_passes() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(32, 65_536)?;
    let scenario = decode_expand_scenario(&sweep_source(15, 1, "1"))?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let plan =
        preflight_scenarios(&mapping, &selected).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(plan.tests().len(), 15);
    Ok(())
}

#[test]
fn window_rows_precede_window_work() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(32, 1)?;
    let windows = (1..=1_000_u32)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let scenario = decode_expand_scenario(&sweep_source(1_001, 1_000, &windows))?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error =
        preflight_scenarios(&mapping, &selected).expect_err("window rows must fail first");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases|expected integer <= 1000000, observed 1001000"]
    );
    Ok(())
}

#[test]
fn window_rows_at_exact_limit_pass() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(32, 1)?;
    let windows = (1..=100_u32)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let scenario = decode_expand_scenario(&sweep_source(10_000, 100, &windows))?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let plan =
        preflight_scenarios(&mapping, &selected).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(plan.tests().len(), 10_000);
    Ok(())
}

#[test]
fn exact_window_work_is_the_fifth_global_gate() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(32, 1)?;
    let windows = (1..=1_000_u32)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let exact = decode_expand_scenario(&sweep_source(100, 1_000, &windows))?;
    let over = decode_expand_scenario(&sweep_source(101, 1_000, &windows))?;
    let exact_selected = select_cases(&exact, &[]).map_err(|error| error.to_string())?;
    let over_selected = select_cases(&over, &[]).map_err(|error| error.to_string())?;

    // When
    let plan =
        preflight_scenarios(&mapping, &exact_selected).map_err(|error| error.to_string())?;
    let error =
        preflight_scenarios(&mapping, &over_selected).expect_err("window work + 1 must fail");

    // Then
    assert_eq!(plan.tests().len(), 100);
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases|expected sum(Q*K) <= 100000000, observed 101000000"]
    );
    Ok(())
}

#[test]
fn single_maximum_access_descriptor_with_1024_windows_fails_exact_work_sum(
) -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(32, 1)?;
    let windows = (1..=1_024_u32)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!(
        "schema_version: 1\ndefaults: {{ accesses: 10000000, window_sizes: [{windows}] }}\n\
         cases:\n  - {{ name: maximum-work, kind: stride, base_bytes: 0, stride_bytes: 0 }}\n"
    );
    let scenario = decode_expand_scenario(&source)?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error =
        preflight_scenarios(&mapping, &selected).expect_err("exact window work must fail");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases|expected sum(Q*K) <= 100000000, observed 10240000000"]
    );
    Ok(())
}

#[test]
fn effective_window_count_at_limit_passes_and_plus_one_fails() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(32, 1)?;
    let exact_windows = (1..=1_024_u32)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let over_windows = format!("{exact_windows}, 1025");
    let exact = decode_expand_scenario(&sweep_source(1, 1_024, &exact_windows))?;
    let over = decode_expand_scenario(&sweep_source(1, 1_025, &over_windows))?;
    let exact_selected = select_cases(&exact, &[]).map_err(|error| error.to_string())?;
    let over_selected = select_cases(&over, &[]).map_err(|error| error.to_string())?;

    // When
    let plan =
        preflight_scenarios(&mapping, &exact_selected).map_err(|error| error.to_string())?;
    let error =
        preflight_scenarios(&mapping, &over_selected).expect_err("window count + 1 must fail");

    // Then
    assert_eq!(concrete_test(&plan, 0)?.window_sizes().len(), 1_024);
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|defaults.window_sizes|expected integer <= 1024, observed 1025"]
    );
    Ok(())
}

#[test]
fn checked_last_address_above_u128_reports_exact_magnitude() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(64, 1)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 2, window_sizes: [1] }\ncases:\n\
         \x20 - name: overflow\n    kind: multi_stream\n    schedule: round_robin\n\
         \x20   streams:\n      - { name: s, base_bytes: 0xffffffffffffffffffffffffffffffff, \
         stride_bytes: 0xffffffffffffffffffffffffffffffff, accesses: 2 }\n",
    )?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error =
        preflight_scenarios(&mapping, &selected).expect_err("checked u128 overflow must fail");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["address.out_of_range|cases[0].streams[0]|address 0x1fffffffffffffffffffffffffffffffe is outside the 64-bit range"]
    );
    Ok(())
}

#[test]
fn documented_template_expands_to_fourteen_tests_in_declaration_order(
) -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(20, 4)?;
    let scenario = decode_expand_scenario(include_str!("../../assets/templates/scenario.yaml"))?;
    let selected = select_cases(
        &scenario,
        &[
            "two-master",
            "stride-and-phase-sweep",
            "sequential",
            "two-master",
        ],
    )
    .map_err(|error| error.to_string())?;

    // When
    let plan =
        preflight_scenarios(&mapping, &selected).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(
        plan.tests()
            .iter()
            .map(interleave::scenario::ConcreteTestDescriptor::case_id)
            .collect::<Vec<_>>(),
        [
            "sequential",
            "stride-and-phase-sweep[base=0x0,stride=0x40]",
            "stride-and-phase-sweep[base=0x0,stride=0x80]",
            "stride-and-phase-sweep[base=0x0,stride=0x100]",
            "stride-and-phase-sweep[base=0x40,stride=0x40]",
            "stride-and-phase-sweep[base=0x40,stride=0x80]",
            "stride-and-phase-sweep[base=0x40,stride=0x100]",
            "stride-and-phase-sweep[base=0x80,stride=0x40]",
            "stride-and-phase-sweep[base=0x80,stride=0x80]",
            "stride-and-phase-sweep[base=0x80,stride=0x100]",
            "stride-and-phase-sweep[base=0xc0,stride=0x40]",
            "stride-and-phase-sweep[base=0xc0,stride=0x80]",
            "stride-and-phase-sweep[base=0xc0,stride=0x100]",
            "two-master",
        ]
    );
    Ok(())
}
