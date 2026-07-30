use super::maximum_load_ratio;

#[test]
fn largest_approved_maximum_load_numerator_remains_unreduced() -> Result<(), String> {
    // Given
    let target_count = 65_536;
    let maximum_count = 10_000_000;
    let accesses = 10_000_000;

    // When
    let ratio = maximum_load_ratio(target_count, maximum_count, accesses)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(ratio.numerator(), 655_360_000_000);
    assert_eq!(ratio.denominator(), 10_000_000);
    assert_eq!(
        ratio.decimal_six().map_err(|error| error.to_string())?,
        "65536.000000"
    );
    Ok(())
}

#[test]
fn shared_ratio_formatter_rounds_an_exact_half_up() -> Result<(), String> {
    // Given
    let ratio = super::super::exact_ratio(1, 2_000_000).map_err(|error| error.to_string())?;

    // When
    let decimal = ratio.decimal_six().map_err(|error| error.to_string())?;

    // Then
    assert_eq!(decimal, "0.000001");
    Ok(())
}
