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
