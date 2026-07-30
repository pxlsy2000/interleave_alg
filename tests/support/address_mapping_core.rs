#[test]
fn maps_documented_example_and_preserves_unaligned_offset() -> TestResult {
    // Given
    let fixture = natural_fixture(16, 64, 2)?;
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping).map_err(|validation| {
        format!("unexpected classification: {:?}", validation.classification())
    })?;

    // When
    let row = mapper
        .map_address(address("0x1234")?)
        .map_err(|error| error.to_string())?;

    // Then
    assert_row(&row, ("0x1234", 0x48, 0x34, 0, 0x12, 0x4b4))?;
    assert_eq!(row.input_address_canonical(), "0x1234");
    assert_eq!(row.line_address_canonical(), "0x48");
    assert_eq!(row.byte_offset_canonical(), "0x34");
    assert_eq!(row.local_line_address_canonical(), "0x12");
    assert_eq!(row.local_byte_address_canonical(), "0x4b4");
    Ok(())
}

#[test]
fn preserves_source_order_and_duplicate_addresses() -> TestResult {
    // Given
    let fixture = natural_fixture(3, 1, 1)?;
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;
    let addresses = [address("5")?, address("1")?, address("5")?];

    // When
    let rows = mapper
        .map_addresses(&addresses)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(
        rows.iter()
            .map(MappedAddress::input_address)
            .collect::<Vec<_>>(),
        addresses
    );
    assert_eq!(
        rows.iter().map(MappedAddress::target).collect::<Vec<_>>(),
        [1, 1, 1]
    );
    Ok(())
}

#[test]
fn maps_valid_non_natural_explicit_local_rows() -> TestResult {
    // Given
    let local_rows = [0b011, 0b100];
    let fixture = MappingFixture {
        address_width: 3,
        granule_bytes: 1,
        target_rows: &[0b001],
        local_rows: LocalRows::Explicit(&local_rows),
    };
    let mapping = decode(&fixture.source())?;
    let mapper = AddressMapper::try_new(&mapping)
        .map_err(|validation| validation.classification().as_str().to_owned())?;

    // When
    let row = mapper
        .map_address(address("3")?)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(
        mapper.validation().classification(),
        MappingClassification::ValidNonNatural
    );
    assert_row(&row, ("3", 3, 0, 1, 0, 0))
}

#[test]
fn refuses_mathematically_invalid_mapping_before_address_work() -> TestResult {
    // Given
    let local_rows = [0b001, 0b100];
    let fixture = MappingFixture {
        address_width: 3,
        granule_bytes: 1,
        target_rows: &[0b001],
        local_rows: LocalRows::Explicit(&local_rows),
    };
    let mapping = decode(&fixture.source())?;

    // When
    let result = AddressMapper::try_new(&mapping);

    // Then
    let validation = result
        .err()
        .ok_or_else(|| "invalid Mapping unexpectedly constructed a mapper".to_owned())?;
    assert_eq!(
        validation.classification(),
        MappingClassification::InvalidNonBijective
    );
    Ok(())
}
