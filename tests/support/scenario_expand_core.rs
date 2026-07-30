use interleave::{
    input::load_yaml_bytes,
    issue::IssueCode,
    mapping::{MappingModel, decode_mapping},
    scenario::{
        ConcreteStimulus, ScenarioModel, preflight_scenarios, select_cases,
    },
};

type ScenarioExpandResult<T = ()> = Result<T, String>;

fn decode_expand_scenario(source: &str) -> ScenarioExpandResult<ScenarioModel> {
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    interleave::scenario::decode_scenario(&document).map_err(|error| error.to_string())
}

fn decode_expand_mapping(width: u8, targets: u32) -> ScenarioExpandResult<MappingModel> {
    let target_bits = targets.ilog2();
    let rows = (0..target_bits)
        .map(|bit| format!("[{bit}]"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!(
        "schema_version: 1\nname: test\naddress:\n  width_bits: {width}\n  granule_bytes: 1\n\
         targets:\n  count: {targets}\nmapping:\n  m:\n    rows: [{rows}]\n  l:\n    mode: preserve_high\n"
    );
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    decode_mapping(&document).map_err(|error| error.to_string())
}

fn simple_scenario(cases: &str) -> String {
    format!(
        "schema_version: 1\ndefaults:\n  accesses: 4\n  window_sizes: [1, 4]\ncases:\n{cases}"
    )
}

fn rendered_issues(issues: &[interleave::issue::Issue]) -> Vec<String> {
    issues
        .iter()
        .map(|issue| {
            format!(
                "{}|{}|{}",
                issue.code().as_str(),
                issue.path().as_str(),
                issue.message()
            )
        })
        .collect()
}

fn concrete_test(
    plan: &interleave::scenario::ScenarioPreflightPlan,
    index: usize,
) -> ScenarioExpandResult<&interleave::scenario::ConcreteTestDescriptor> {
    plan.tests()
        .get(index)
        .ok_or_else(|| format!("missing concrete test at {index}"))
}

#[test]
fn selection_uses_enabled_only_without_explicit_names() -> ScenarioExpandResult {
    // Given
    let scenario = decode_expand_scenario(&simple_scenario(
        "  - { name: first, enabled: false, kind: stride, base_bytes: 0, stride_bytes: 1 }\n\
         \x20 - { name: second, kind: stride, base_bytes: 0, stride_bytes: 1 }\n",
    ))?;

    // When
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(
        selected
            .cases()
            .iter()
            .map(|selected| selected.case().name().as_str())
            .collect::<Vec<_>>(),
        ["second"]
    );
    Ok(())
}

#[test]
fn explicit_selection_deduplicates_and_returns_declaration_order() -> ScenarioExpandResult {
    // Given
    let scenario = decode_expand_scenario(&simple_scenario(
        "  - { name: first, enabled: false, kind: stride, base_bytes: 0, stride_bytes: 1 }\n\
         \x20 - { name: second, kind: stride, base_bytes: 0, stride_bytes: 1 }\n\
         \x20 - { name: third, kind: stride, base_bytes: 0, stride_bytes: 1 }\n",
    ))?;

    // When
    let selected = select_cases(&scenario, &["third", "first", "third"])
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(
        selected
            .cases()
            .iter()
            .map(|selected| selected.case().name().as_str())
            .collect::<Vec<_>>(),
        ["first", "third"]
    );
    Ok(())
}

#[test]
fn missing_names_are_deduplicated_in_cli_order_without_no_selection() -> ScenarioExpandResult {
    // Given
    let scenario = decode_expand_scenario(&simple_scenario(
        "  - { name: present, kind: stride, base_bytes: 0, stride_bytes: 1 }\n",
    ))?;

    // When
    let error = select_cases(&scenario, &["z", "missing", "z"])
        .expect_err("missing cases must fail");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        [
            "scenario.case_not_found||case \"z\" was not found",
            "scenario.case_not_found||case \"missing\" was not found",
        ]
    );
    assert_eq!(error.exit_class().code(), 3);
    Ok(())
}

#[test]
fn empty_default_selection_emits_one_selection_issue() -> ScenarioExpandResult {
    // Given
    let scenario = decode_expand_scenario(&simple_scenario(
        "  - { name: off, enabled: false, kind: stride, base_bytes: 0, stride_bytes: 1 }\n",
    ))?;

    // When
    let error = select_cases(&scenario, &[]).expect_err("all-disabled selection must fail");

    // Then
    assert_eq!(error.issues().len(), 1);
    assert_eq!(
        error
            .issues()
            .first()
            .ok_or_else(|| "missing no-selection issue".to_owned())?
            .code(),
        IssueCode::ScenarioNoCaseSelected
    );
    Ok(())
}

#[test]
fn preflight_expands_sweep_base_major_with_canonical_ids() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(16, 4)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 2, window_sizes: [1, 2] }\ncases:\n\
         \x20 - name: sweep-it\n    kind: sweep\n    base_bytes: [64, 0]\n\
         \x20   stride_bytes: [256, 16]\n",
    )?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

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
            "sweep-it[base=0x40,stride=0x100]",
            "sweep-it[base=0x40,stride=0x10]",
            "sweep-it[base=0x0,stride=0x100]",
            "sweep-it[base=0x0,stride=0x10]",
        ]
    );
    assert!(plan
        .tests()
        .iter()
        .all(|test| test.source_case() == "sweep-it" && test.accesses() == 2));
    assert!(plan
        .tests()
        .iter()
        .all(|test| matches!(test.stimulus(), ConcreteStimulus::Linear(_))));
    Ok(())
}

#[test]
fn selected_window_override_wins_and_unselected_w_greater_than_q_is_ignored(
) -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(16, 4)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [2] }\ncases:\n\
         \x20 - { name: ignored, enabled: false, kind: stride, base_bytes: 0, stride_bytes: 0 }\n\
         \x20 - name: chosen\n    kind: stride\n    base_bytes: 0\n    stride_bytes: 0\n\
         \x20   accesses: 4\n    window_sizes: [4]\n",
    )?;
    let selected = select_cases(&scenario, &["chosen"]).map_err(|error| error.to_string())?;

    // When
    let plan =
        preflight_scenarios(&mapping, &selected).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(plan.tests().len(), 1);
    assert_eq!(
        concrete_test(&plan, 0)?
            .window_sizes()
            .iter()
            .map(|window| window.get())
            .collect::<Vec<_>>(),
        [4]
    );
    Ok(())
}

#[test]
fn selected_window_larger_than_q_uses_its_source_path() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(16, 4)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [2] }\ncases:\n\
         \x20 - { name: bad, kind: stride, base_bytes: 0, stride_bytes: 0 }\n",
    )?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error = preflight_scenarios(&mapping, &selected).expect_err("W > Q must fail");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|defaults.window_sizes[0]|expected integer <= 1, observed 2"]
    );
    Ok(())
}

#[test]
fn last_address_is_checked_before_any_analysis() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(8, 4)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 3, window_sizes: [1] }\ncases:\n\
         \x20 - { name: bad, kind: stride, base_bytes: 250, stride_bytes: 3 }\n",
    )?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error = preflight_scenarios(&mapping, &selected)
        .expect_err("last address outside A bits must fail");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["address.out_of_range|cases[0]|address 0x100 is outside the 8-bit range"]
    );
    Ok(())
}
