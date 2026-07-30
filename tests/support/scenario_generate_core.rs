use interleave::{
    input::load_yaml_bytes,
    mapping::{AddressMapper, MappingModel, decode_mapping},
    scenario::{
        ScenarioModel, ScenarioPreflightPlan, decode_scenario, generate_target_sequence,
        preflight_scenarios, select_cases,
    },
};

type ScenarioGenerateResult<T = ()> = Result<T, String>;

fn generation_mapping() -> ScenarioGenerateResult<MappingModel> {
    let document = load_yaml_bytes(
        b"schema_version: 1
name: identity-target
address: { width_bits: 8, granule_bytes: 1 }
targets: { count: 4 }
mapping:
  m: { rows: [[0], [1]] }
  l: { mode: preserve_high }
",
    )
    .map_err(|error| error.to_string())?;
    decode_mapping(&document).map_err(|error| error.to_string())
}

fn generation_plan(
    mapping: &MappingModel,
    cases: &str,
) -> ScenarioGenerateResult<ScenarioPreflightPlan> {
    let source = format!(
        "schema_version: 1\ndefaults: {{ accesses: 4, window_sizes: [1] }}\ncases:\n{cases}"
    );
    let document =
        load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    let scenario: ScenarioModel =
        decode_scenario(&document).map_err(|error| error.to_string())?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;
    preflight_scenarios(mapping, &selected).map_err(|error| error.to_string())
}

fn generated_at(
    mapping: &MappingModel,
    plan: &ScenarioPreflightPlan,
    index: usize,
) -> ScenarioGenerateResult<Vec<u16>> {
    let mapper = AddressMapper::try_new(mapping).map_err(|error| error.to_string())?;
    let descriptor = plan
        .tests()
        .get(index)
        .ok_or_else(|| format!("missing descriptor at {index}"))?;
    generate_target_sequence(&mapper, descriptor).map_err(|error| error.to_string())
}

#[test]
fn stride_zero_repeats_the_mapped_base_target() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - { name: fixed, kind: stride, base_bytes: 3, stride_bytes: 0 }\n",
    )?;

    // When
    let targets = generated_at(&mapping, &plan, 0)?;

    // Then
    assert_eq!(targets, [3, 3, 3, 3]);
    Ok(())
}

#[test]
fn nonzero_stride_preserves_address_order() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - { name: walking, kind: stride, base_bytes: 1, stride_bytes: 2 }\n",
    )?;

    // When
    let targets = generated_at(&mapping, &plan, 0)?;

    // Then
    assert_eq!(targets, [1, 3, 1, 3]);
    Ok(())
}

#[test]
fn sweep_combinations_remain_independent_and_base_major() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - name: phases\n    kind: sweep\n    base_bytes: [0, 1]\n    stride_bytes: [1, 2]\n    accesses: 3\n",
    )?;

    // When
    let generated = (0..plan.tests().len())
        .map(|index| generated_at(&mapping, &plan, index))
        .collect::<Result<Vec<_>, _>>()?;

    // Then
    assert_eq!(
        plan.tests()
            .iter()
            .map(interleave::scenario::ConcreteTestDescriptor::case_id)
            .collect::<Vec<_>>(),
        [
            "phases[base=0x0,stride=0x1]",
            "phases[base=0x0,stride=0x2]",
            "phases[base=0x1,stride=0x1]",
            "phases[base=0x1,stride=0x2]",
        ]
    );
    assert_eq!(
        generated,
        [vec![0, 1, 2], vec![0, 2, 0], vec![1, 2, 3], vec![1, 3, 1]]
    );
    Ok(())
}

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

#[test]
fn repeated_generation_is_byte_identical() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - { name: repeat, kind: stride, base_bytes: 1, stride_bytes: 3, accesses: 8 }\n",
    )?;

    // When
    let first = generated_at(&mapping, &plan, 0)?;
    let second = generated_at(&mapping, &plan, 0)?;

    // Then
    assert_eq!(first, second);
    assert_eq!(first, [1, 0, 3, 2, 1, 0, 3, 2]);
    Ok(())
}

#[test]
fn small_linear_domain_matches_direct_identity_oracle() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;

    for base in 0_u16..8 {
        for stride in 0_u16..5 {
            for accesses in 1_u16..7 {
                let plan = generation_plan(
                    &mapping,
                    &format!(
                        "  - {{ name: oracle, kind: stride, base_bytes: {base}, stride_bytes: {stride}, accesses: {accesses} }}\n"
                    ),
                )?;

                // When
                let targets = generated_at(&mapping, &plan, 0)?;

                // Then
                let expected = (0..accesses)
                    .map(|index| (base + index * stride) % 4)
                    .collect::<Vec<_>>();
                assert_eq!(targets, expected);
            }
        }
    }
    Ok(())
}
