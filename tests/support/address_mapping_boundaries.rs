#[test]
fn maps_granule_one_and_target_one() -> TestResult {
    // Given
    let fixture = natural_fixture(4, 1, 0)?;
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;

    // When
    let row = mapper
        .map_address(address("15")?)
        .map_err(|error| error.to_string())?;

    // Then
    assert_row(&row, ("15", 15, 0, 0, 15, 15))
}

#[test]
fn maps_full_target_space_with_zero_local_line_bits() -> TestResult {
    // Given
    let fixture = natural_fixture(3, 1, 3)?;
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;

    // When
    let row = mapper
        .map_address(address("7")?)
        .map_err(|error| error.to_string())?;

    // Then
    assert_row(&row, ("7", 7, 0, 7, 0, 0))
}

#[test]
fn maps_zero_line_bits_and_preserves_whole_address_as_offset() -> TestResult {
    // Given
    let fixture = natural_fixture(4, 16, 0)?;
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;

    // When
    let row = mapper
        .map_address(address("15")?)
        .map_err(|error| error.to_string())?;

    // Then
    assert_row(&row, ("15", 0, 15, 0, 0, 15))
}

#[test]
fn maps_largest_a64_address_without_intermediate_truncation() -> TestResult {
    // Given
    let fixture = natural_fixture(64, 1, 0)?;
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;

    // When
    let row = mapper
        .map_address(address("0xffff_ffff_ffff_ffff")?)
        .map_err(|error| error.to_string())?;

    // Then
    assert_row(
        &row,
        (
            "0xffffffffffffffff",
            u64::MAX,
            0,
            0,
            u64::MAX,
            u64::MAX,
        ),
    )
}

#[test]
fn rejects_exact_small_bound_after_valid_operand_in_source_order() -> TestResult {
    // Given
    let fixture = natural_fixture(8, 1, 1)?;
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;
    let inputs = [address("255")?, address("256")?];

    // When
    let result = mapper.map_addresses(&inputs);

    // Then
    assert_out_of_range(
        result,
        RangeExpectation {
            index: 1,
            canonical: "0x100",
            width: 8,
        },
    )
}

#[test]
fn rejects_exact_a64_bound_represented_in_u128() -> TestResult {
    // Given
    let fixture = natural_fixture(64, 1, 0)?;
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;
    let input = address("0x1_0000_0000_0000_0000")?;

    // When
    let result = mapper.map_address(input);

    // Then
    assert_out_of_range(
        result,
        RangeExpectation {
            index: 0,
            canonical: "0x10000000000000000",
            width: 64,
        },
    )
}

#[test]
fn maps_bit_63_tap_without_signed_conversion_or_shift_wrap() -> TestResult {
    // Given
    let local_rows = (0..63).map(|bit| 1_u64 << bit).collect::<Vec<_>>();
    let fixture = MappingFixture {
        address_width: 64,
        granule_bytes: 1,
        target_rows: &[1_u64 << 63],
        local_rows: LocalRows::Explicit(&local_rows),
    };
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;

    // When
    let row = mapper
        .map_address(address("0xffff_ffff_ffff_ffff")?)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(row.target(), 1);
    assert_eq!(row.local_line_address(), 0x7fff_ffff_ffff_ffff);
    assert_eq!(row.local_byte_address(), 0x7fff_ffff_ffff_ffff);
    Ok(())
}

#[test]
fn analysis_failure_uses_stable_public_diagnostic() {
    // Given
    let error = AddressMappingError::AnalysisFailed;

    // When
    let issue = error.issue();

    // Then
    assert_eq!(issue.code(), IssueCode::AnalysisFailed);
    assert_eq!(issue.path().as_str(), "");
    assert_eq!(issue.message(), "analysis could not be completed");
    assert_eq!(issue.severity(), interleave::issue::Severity::Error);
}

#[derive(Clone, Copy)]
struct RangeExpectation<'a> {
    index: usize,
    canonical: &'a str,
    width: u8,
}

fn assert_out_of_range<T>(
    result: Result<T, AddressMappingError>,
    expected: RangeExpectation<'_>,
) -> TestResult {
    let error = result
        .err()
        .ok_or_else(|| "out-of-range address unexpectedly mapped".to_owned())?;
    assert_eq!(error.address_index(), Some(expected.index));
    let issue = error.issue();
    assert_eq!(issue.code(), IssueCode::AddressOutOfRange);
    assert_eq!(
        issue.path().as_str(),
        format!("addresses[{}]", expected.index)
    );
    assert_eq!(
        issue.message(),
        format!(
            "address {} is outside the {}-bit range",
            expected.canonical, expected.width
        )
    );
    Ok(())
}
