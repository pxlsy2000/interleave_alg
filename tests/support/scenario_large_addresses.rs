#[test]
fn selected_unbounded_plain_base_reaches_generated_address_preflight() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(8, 4)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [1] }\ncases:\n\
         \x20 - name: huge\n    kind: stride\n\
         \x20   base_bytes: 0x100000000000000000000000000000000\n\
         \x20   stride_bytes: 0\n",
    )?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error = preflight_scenarios(&mapping, &selected)
        .expect_err("the generated base is outside the Mapping width");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["address.out_of_range|cases[0]|address 0x100000000000000000000000000000000 is outside the 8-bit range"]
    );
    Ok(())
}

#[test]
fn selected_unbounded_decimal_base_uses_full_hexadecimal_diagnostic() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(8, 4)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [1] }\ncases:\n\
         \x20 - { name: decimal, kind: stride, base_bytes: 340_282_366_920_938_463_463_374_607_431_768_211_457, stride_bytes: 0 }\n",
    )?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error = preflight_scenarios(&mapping, &selected)
        .expect_err("the decimal generated base is outside the Mapping width");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["address.out_of_range|cases[0]|address 0x100000000000000000000000000000001 is outside the 8-bit range"]
    );
    Ok(())
}

#[test]
fn whole_document_decode_retains_unbounded_addresses_for_every_case_kind() -> ScenarioExpandResult {
    // Given
    let source = "schema_version: 1\ndefaults: { accesses: 2, window_sizes: [1] }\ncases:\n\
        \x20 - { name: linear, kind: stride, base_bytes: 0x100000000000000000000000000000000, stride_bytes: 340282366920938463463374607431768211457 }\n\
        \x20 - { name: cart, kind: sweep, base_bytes: [0x100000000000000000000000000000002], stride_bytes: [340282366920938463463374607431768211459] }\n\
        \x20 - name: streams\n    kind: multi_stream\n    schedule: round_robin\n    streams:\n\
        \x20     - { name: s, base_bytes: 0x100000000000000000000000000000004, stride_bytes: 340282366920938463463374607431768211461, accesses: 1 }\n";

    // When
    let model = decode_expand_scenario(source)?;

    // Then
    let canonicals = model
        .cases()
        .iter()
        .flat_map(|case| match case.kind() {
            interleave::scenario::ScenarioCaseKind::Stride(value) => vec![
                value.base_bytes().canonical(),
                value.stride_bytes().canonical(),
            ],
            interleave::scenario::ScenarioCaseKind::Sweep(value) => vec![
                value.base_bytes().first().map(AddressMagnitude::canonical),
                value.stride_bytes().first().map(AddressMagnitude::canonical),
            ]
            .into_iter()
            .flatten()
            .collect(),
            interleave::scenario::ScenarioCaseKind::MultiStream(value) => value
                .streams()
                .first()
                .map(|stream| {
                    vec![
                        stream.base_bytes().canonical(),
                        stream.stride_bytes().canonical(),
                    ]
                })
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        canonicals,
        [
            "0x100000000000000000000000000000000",
            "0x100000000000000000000000000000001",
            "0x100000000000000000000000000000002",
            "0x100000000000000000000000000000003",
            "0x100000000000000000000000000000004",
            "0x100000000000000000000000000000005",
        ]
    );
    Ok(())
}

#[test]
fn sweep_duplicate_gate_compares_unbounded_mathematical_values() -> ScenarioExpandResult {
    // Given
    let source = "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [1] }\ncases:\n\
        \x20 - name: duplicate\n    kind: sweep\n\
        \x20   base_bytes: [340_282_366_920_938_463_463_374_607_431_768_211_456, 0x100000000000000000000000000000000]\n\
         \x20   stride_bytes: [340_282_366_920_938_463_463_374_607_431_768_211_457, 0x100000000000000000000000000000001]\n";
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;

    // When
    let error = interleave::scenario::decode_scenario(&document)
        .expect_err("equivalent decimal and hexadecimal magnitudes must duplicate");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        [
            "scenario.invalid|cases[0].base_bytes|expected unique values, observed sequence",
            "scenario.invalid|cases[0].stride_bytes|expected unique values, observed sequence",
        ]
    );
    Ok(())
}

#[test]
fn unused_unbounded_stride_is_truthfully_absent_but_q_two_fails_exactly(
) -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(8, 4)?;
    let q_one = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [1] }\ncases:\n\
         \x20 - { name: once, kind: sweep, base_bytes: [1], stride_bytes: [0x100000000000000000000000000000000] }\n",
    )?;
    let q_two = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 2, window_sizes: [1] }\ncases:\n\
         \x20 - { name: twice, kind: stride, base_bytes: 1, stride_bytes: 0x100000000000000000000000000000000 }\n",
    )?;

    // When
    let one_selected = select_cases(&q_one, &[]).map_err(|error| error.to_string())?;
    let one = preflight_scenarios(&mapping, &one_selected).map_err(|error| error.to_string())?;
    let two_selected = select_cases(&q_two, &[]).map_err(|error| error.to_string())?;
    let two = preflight_scenarios(&mapping, &two_selected)
        .expect_err("the second generated address must fail");

    // Then
    let ConcreteStimulus::Linear(linear) = concrete_test(&one, 0)?.stimulus() else {
        return Err("stride case changed stimulus kind".to_owned());
    };
    assert_eq!(
        concrete_test(&one, 0)?.case_id(),
        "once[base=0x1,stride=0x100000000000000000000000000000000]"
    );
    assert_eq!(linear.base_bytes().get(), 1);
    assert_eq!(linear.stride_bytes(), None);
    assert_eq!(
        rendered_issues(two.issues()),
        ["address.out_of_range|cases[0]|address 0x100000000000000000000000000000001 is outside the 8-bit range"]
    );
    Ok(())
}

#[test]
fn sweep_and_stream_failures_keep_exact_source_order_and_paths() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(8, 4)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 2, window_sizes: [1] }\ncases:\n\
         \x20 - name: cart\n    kind: sweep\n\
         \x20   base_bytes: [0x100000000000000000000000000000000, 0x100000000000000000000000000000001]\n\
         \x20   stride_bytes: [0, 1]\n\
         \x20 - name: streams\n    kind: multi_stream\n    schedule: round_robin\n    streams:\n\
         \x20     - { name: huge-base, base_bytes: 0x100000000000000000000000000000002, stride_bytes: 0, accesses: 1 }\n\
         \x20     - { name: unused, base_bytes: 0, stride_bytes: 0x100000000000000000000000000000003, accesses: 1 }\n\
         \x20     - { name: used, base_bytes: 0, stride_bytes: 0x100000000000000000000000000000004, accesses: 2 }\n",
    )?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error = preflight_scenarios(&mapping, &selected)
        .expect_err("selected generated addresses must fail atomically");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        [
            "address.out_of_range|cases[0]|address 0x100000000000000000000000000000000 is outside the 8-bit range",
            "address.out_of_range|cases[0]|address 0x100000000000000000000000000000001 is outside the 8-bit range",
            "address.out_of_range|cases[0]|address 0x100000000000000000000000000000001 is outside the 8-bit range",
            "address.out_of_range|cases[0]|address 0x100000000000000000000000000000002 is outside the 8-bit range",
            "address.out_of_range|cases[1].streams[0]|address 0x100000000000000000000000000000002 is outside the 8-bit range",
            "address.out_of_range|cases[1].streams[2]|address 0x100000000000000000000000000000004 is outside the 8-bit range",
        ]
    );
    Ok(())
}

#[test]
fn disabled_unbounded_cases_are_inert_until_explicitly_selected() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(8, 4)?;
    let scenario = decode_expand_scenario(
        "schema_version: 1\ndefaults: { accesses: 1, window_sizes: [1] }\ncases:\n\
         \x20 - { name: good, kind: stride, base_bytes: 0, stride_bytes: 0 }\n\
         \x20 - { name: hidden-linear, enabled: false, kind: stride, base_bytes: 0x100000000000000000000000000000000, stride_bytes: 0 }\n\
         \x20 - { name: hidden-sweep, enabled: false, kind: sweep, base_bytes: [0x100000000000000000000000000000001], stride_bytes: [0] }\n\
         \x20 - name: hidden-stream\n    enabled: false\n    kind: multi_stream\n    schedule: round_robin\n    streams:\n\
         \x20     - { name: s, base_bytes: 0x100000000000000000000000000000002, stride_bytes: 0, accesses: 1 }\n",
    )?;

    // When
    let defaults = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;
    let default_plan = preflight_scenarios(&mapping, &defaults).map_err(|error| error.to_string())?;
    let explicit = select_cases(&scenario, &["hidden-linear"])
        .map_err(|error| error.to_string())?;
    let error = preflight_scenarios(&mapping, &explicit)
        .expect_err("explicit selection must activate the hidden range failure");
    let missing = select_cases(&scenario, &["absent"])
        .expect_err("selection failure must suppress address preflight");

    // Then
    assert_eq!(default_plan.tests().len(), 1);
    assert_eq!(
        missing
            .issues()
            .first()
            .map(interleave::issue::Issue::code),
        Some(IssueCode::ScenarioCaseNotFound)
    );
    assert_eq!(
        rendered_issues(error.issues()),
        ["address.out_of_range|cases[1]|address 0x100000000000000000000000000000000 is outside the 8-bit range"]
    );
    Ok(())
}

#[test]
fn first_global_resource_gate_suppresses_unbounded_address_failures() -> ScenarioExpandResult {
    // Given
    let mapping = decode_expand_mapping(64, 4)?;
    let bases = format!(
        "0x100000000000000000000000000000000, {}",
        integer_list(10_000)
    );
    let source = format!(
        "schema_version: 1\ndefaults: {{ accesses: 1, window_sizes: [1] }}\ncases:\n\
         \x20 - name: capped\n    kind: sweep\n    base_bytes: [{bases}]\n    stride_bytes: [0]\n"
    );
    let scenario = decode_expand_scenario(&source)?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;

    // When
    let error = preflight_scenarios(&mapping, &selected)
        .expect_err("the first global gate must suppress address preflight");

    // Then
    assert_eq!(
        rendered_issues(error.issues()),
        ["scenario.invalid|cases|expected integer <= 10000, observed 10001"]
    );
    Ok(())
}
use interleave::input::scalar::AddressMagnitude;
