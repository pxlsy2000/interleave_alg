use interleave::{
    input::{load_yaml_bytes, scalar::Address},
    mapping::{AddressMapper, MappingModel, decode_mapping},
    metrics::analyze_targets,
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
fn unaligned_base_and_stride_preserve_generated_address_byte_offsets() -> ScenarioGenerateResult {
    // Given
    let document = load_yaml_bytes(
        b"schema_version: 1
name: unaligned
address: { width_bits: 6, granule_bytes: 4 }
targets: { count: 4 }
mapping:
  m: { rows: [[0], [1]] }
  l: { mode: preserve_high }
",
    )
    .map_err(|error| error.to_string())?;
    let mapping = decode_mapping(&document).map_err(|error| error.to_string())?;
    let plan = generation_plan(
        &mapping,
        "  - { name: unaligned, kind: stride, base_bytes: 1, stride_bytes: 3, accesses: 5 }\n",
    )?;

    // When
    let targets = generated_at(&mapping, &plan, 0)?;
    let mapper = AddressMapper::try_new(&mapping).map_err(|error| error.to_string())?;
    let mapped_rows = [1_u128, 4, 7, 10, 13]
        .into_iter()
        .map(|value| {
            let text = value.to_string();
            let address = Address::parse(&text).map_err(|error| error.to_string())?;
            mapper
                .map_address(address)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Then
    assert_eq!(targets, [0, 1, 1, 2, 3]);
    assert_eq!(
        mapped_rows
            .iter()
            .map(interleave::mapping::MappedAddress::target)
            .collect::<Vec<_>>(),
        targets
    );
    assert_eq!(
        mapped_rows
            .iter()
            .map(interleave::mapping::MappedAddress::byte_offset)
            .collect::<Vec<_>>(),
        [1, 0, 3, 2, 1]
    );
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
fn sweep_combinations_keep_independent_metrics_in_base_major_order() -> ScenarioGenerateResult {
    // Given
    let mapping = generation_mapping()?;
    let plan = generation_plan(
        &mapping,
        "  - name: measured\n    kind: sweep\n    base_bytes: [0, 1]\n    stride_bytes: [0, 1]\n    accesses: 3\n    window_sizes: [2]\n",
    )?;

    // When
    let measured = plan
        .tests()
        .iter()
        .enumerate()
        .map(|(index, descriptor)| {
            let targets = generated_at(&mapping, &plan, index)?;
            let metrics = analyze_targets(
                &targets,
                mapping.target_count(),
                descriptor.window_sizes(),
            )
            .map_err(|error| error.to_string())?;
            Ok((
                descriptor.case_id().to_owned(),
                targets,
                (
                    metrics.max_load().target(),
                    metrics.max_load().count(),
                    metrics.max_load().ratio().numerator(),
                    metrics.max_load().ratio().denominator(),
                ),
                metrics.windows().first().map(|window| {
                    (
                        window.target(),
                        window.count(),
                        window.ratio().numerator(),
                        window.ratio().denominator(),
                    )
                }),
                (
                    metrics.longest_run().target(),
                    metrics.longest_run().length(),
                ),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Then
    assert_eq!(
        measured,
        [
            (
                "measured[base=0x0,stride=0x0]".to_owned(),
                vec![0, 0, 0],
                (0, 3, 12, 3),
                Some((0, 2, 8, 2)),
                (0, 3),
            ),
            (
                "measured[base=0x0,stride=0x1]".to_owned(),
                vec![0, 1, 2],
                (0, 1, 4, 3),
                Some((0, 1, 4, 2)),
                (0, 1),
            ),
            (
                "measured[base=0x1,stride=0x0]".to_owned(),
                vec![1, 1, 1],
                (1, 3, 12, 3),
                Some((1, 2, 8, 2)),
                (1, 3),
            ),
            (
                "measured[base=0x1,stride=0x1]".to_owned(),
                vec![1, 2, 3],
                (1, 1, 4, 3),
                Some((1, 1, 4, 2)),
                (1, 1),
            ),
        ]
    );
    Ok(())
}
