#[test]
fn natural_mapping_matches_direct_formula_for_every_small_address() -> TestResult {
    // Given
    for address_width in 1_u8..=8 {
        for granule_bits in 0..address_width {
            let granule = 1_u64 << granule_bits;
            let line_bits = address_width - granule_bits;
            for target_bits in 0..=line_bits {
                let fixture = natural_fixture(address_width, granule, target_bits)?;
                let mapping = decode(&fixture.source())?;
                let mapper = AddressMapper::try_new(&mapping)
                    .map_err(|validation| validation.classification().as_str().to_owned())?;

                // When
                let bound = 1_u128 << address_width;
                for raw in 0..bound {
                    let input = address(&raw.to_string())?;
                    let row = mapper
                        .map_address(input)
                        .map_err(|error| error.to_string())?;
                    let line = u64::try_from(raw / u128::from(granule))
                        .map_err(|error| error.to_string())?;
                    let offset = u64::try_from(raw % u128::from(granule))
                        .map_err(|error| error.to_string())?;
                    let target_mask = if target_bits == 0 {
                        0
                    } else {
                        (1_u64 << target_bits) - 1
                    };

                    // Then
                    assert_eq!(u64::from(row.target()), line & target_mask);
                    assert_eq!(row.local_line_address(), line >> target_bits);
                    assert_eq!(
                        row.local_byte_address(),
                        (line >> target_bits) * granule + offset
                    );
                    let raw_u64 = u64::try_from(raw).map_err(|error| error.to_string())?;
                    assert_eq!(row.local_byte_address() % granule, raw_u64 % granule);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn explicit_mapping_matches_independent_xor_evaluation() -> TestResult {
    // Given
    let target_rows = [0b0101, 0b1010];
    let local_rows = [0b0001, 0b1000];
    let fixture = MappingFixture {
        address_width: 4,
        granule_bytes: 1,
        target_rows: &target_rows,
        local_rows: LocalRows::Explicit(&local_rows),
    };
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;

    // When
    for line in 0_u64..16 {
        let row = mapper
            .map_address(address(&line.to_string())?)
            .map_err(|error| error.to_string())?;

        // Then
        assert_eq!(u64::from(row.target()), direct_rows(&target_rows, line));
        assert_eq!(row.local_line_address(), direct_rows(&local_rows, line));
    }
    Ok(())
}

fn direct_rows(rows: &[u64], line: u64) -> u64 {
    rows.iter().enumerate().fold(0_u64, |output, (bit, row)| {
        output | (u64::from((row & line).count_ones() & 1) << bit)
    })
}
