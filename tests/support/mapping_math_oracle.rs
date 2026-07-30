use std::collections::BTreeSet;

use proptest::prelude::*;

fn mapping_source(n: u8, r: u8, target_rows: &[u64], local_rows: Option<&[u64]>) -> String {
    let target_count = 1_u32 << r;
    let address_width = n.max(1);
    let granule = if n == 0 { 2 } else { 1 };
    let mut source = format!(
        "schema_version: 1\nname: generated\naddress: {{ width_bits: {address_width}, granule_bytes: {granule} }}\ntargets: {{ count: {target_count} }}\nmapping:\n  m:\n"
    );
    append_rows(&mut source, target_rows, n, 4);
    match local_rows {
        None => source.push_str("  l: { mode: preserve_high }\n"),
        Some(rows) => {
            source.push_str("  l:\n    mode: explicit\n");
            append_rows(&mut source, rows, n, 4);
        }
    }
    source
}

fn append_rows(source: &mut String, rows: &[u64], n: u8, indentation: usize) {
    source.push_str(&" ".repeat(indentation));
    if rows.is_empty() {
        source.push_str("rows: []\n");
        return;
    }
    source.push_str("rows:\n");
    for row in rows {
        source.push_str(&" ".repeat(indentation.saturating_add(2)));
        source.push_str("- [");
        let mut first = true;
        for bit in 0..n {
            if row & (1_u64 << bit) != 0 {
                if !first {
                    source.push_str(", ");
                }
                source.push_str(&bit.to_string());
                first = false;
            }
        }
        source.push_str("]\n");
    }
}

fn matrix_case_strategy() -> impl Strategy<Value = (u8, u8, Vec<u64>, Vec<u64>)> {
    (1_u8..=8).prop_flat_map(|n| {
        (Just(n), 0_u8..=n).prop_flat_map(|(n, r)| {
            let bound = 1_u64 << n;
            (
                Just(n),
                Just(r),
                prop::collection::vec(0_u64..bound, usize::from(r)),
                prop::collection::vec(0_u64..bound, usize::from(n - r)),
            )
        })
    })
}

proptest! {
    #[test]
    fn ranks_match_direct_enumeration_for_every_input(
        (n, r, target_rows, local_rows) in matrix_case_strategy()
    ) {
        // Given
        let source = mapping_source(n, r, &target_rows, Some(&local_rows));
        let model = decode(&source).map_err(TestCaseError::fail)?;

        // When
        let result = validate_mapping(&model);
        let oracle = enumerate_outputs(n, r, &target_rows, &local_rows);

        // Then
        let target = result.checks().first().ok_or_else(|| {
            TestCaseError::fail("missing target check")
        })?;
        let bijective = result.checks().get(1).ok_or_else(|| {
            TestCaseError::fail("missing bijective check")
        })?;
        prop_assert_eq!(
            target.status() == CheckStatus::Pass,
            oracle.target_outputs == (1_usize << r)
        );
        prop_assert_eq!(
            bijective.status() == CheckStatus::Pass,
            oracle.combined_outputs == (1_usize << n)
        );
    }
}

struct EnumeratedOutputs {
    target_outputs: usize,
    combined_outputs: usize,
}

fn enumerate_outputs(
    n: u8,
    r: u8,
    target_rows: &[u64],
    local_rows: &[u64],
) -> EnumeratedOutputs {
    let mut targets = BTreeSet::new();
    let mut combined = BTreeSet::new();
    for input in 0_u64..(1_u64 << n) {
        let target = apply_rows(target_rows, input);
        let local = apply_rows(local_rows, input);
        targets.insert(target);
        combined.insert(target | (local << r));
    }
    EnumeratedOutputs {
        target_outputs: targets.len(),
        combined_outputs: combined.len(),
    }
}

fn apply_rows(rows: &[u64], input: u64) -> u64 {
    rows.iter().enumerate().fold(0_u64, |value, (bit, row)| {
        value | (u64::from((row & input).count_ones() & 1) << bit)
    })
}
