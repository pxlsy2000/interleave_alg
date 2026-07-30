use proptest::prelude::*;

proptest! {
    #[test]
    fn sweep_descriptors_match_direct_nested_loop_oracle(
        bases in prop::collection::btree_set(0_u16..512, 1..5),
        strides in prop::collection::btree_set(0_u16..64, 1..5),
    ) {
        // Given
        let bases = bases.into_iter().collect::<Vec<_>>();
        let strides = strides.into_iter().collect::<Vec<_>>();
        let base_source = bases.iter().map(u16::to_string).collect::<Vec<_>>().join(", ");
        let stride_source = strides.iter().map(u16::to_string).collect::<Vec<_>>().join(", ");
        let source = format!(
            "schema_version: 1\ndefaults: {{ accesses: 1, window_sizes: [1] }}\ncases:\n\
             \x20 - name: cart\n    kind: sweep\n    base_bytes: [{base_source}]\n\
             \x20   stride_bytes: [{stride_source}]\n"
        );
        let scenario = decode_expand_scenario(&source)
            .map_err(TestCaseError::fail)?;
        let mapping = decode_expand_mapping(16, 4)
            .map_err(TestCaseError::fail)?;
        let selected = select_cases(&scenario, &[])
            .map_err(|error| TestCaseError::fail(error.to_string()))?;

        // When
        let plan = preflight_scenarios(&mapping, &selected)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;

        // Then
        let expected = bases
            .iter()
            .flat_map(|base| {
                strides.iter().map(move |stride| {
                    format!("cart[base=0x{base:x},stride=0x{stride:x}]")
                })
            })
            .collect::<Vec<_>>();
        prop_assert_eq!(
            plan.tests()
                .iter()
                .map(interleave::scenario::ConcreteTestDescriptor::case_id)
                .collect::<Vec<_>>(),
            expected
        );
    }
}
