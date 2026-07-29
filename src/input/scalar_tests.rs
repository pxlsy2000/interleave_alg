use super::{
    Address, AddressWidth, GenericInteger, Ratio, RatioError, exact_log2, is_power_of_two,
};

#[test]
fn generic_integer_accepts_only_the_plain_integer_grammar() {
    // Given
    let valid = [
        ("0", 0),
        ("42", 42),
        ("0x2A", 42),
        ("0xffffffffffffffff", u128::from(u64::MAX)),
    ];
    let invalid = ["00", "01", "+1", "-1", "1_0", "0X10", "0x", "0x_1"];

    // When / Then
    for (lexeme, expected) in valid {
        assert_eq!(
            GenericInteger::parse(lexeme).map(GenericInteger::get),
            Ok(expected)
        );
    }
    for lexeme in invalid {
        assert!(GenericInteger::parse(lexeme).is_err(), "{lexeme}");
    }
}

#[test]
fn address_accepts_only_between_digit_underscores() {
    // Given
    let valid = [
        ("0", 0),
        ("1_000", 1_000),
        ("0xAB_CD", 0xabcd),
        ("18446744073709551616", u128::from(u64::MAX) + 1),
    ];
    let invalid = [
        "_1", "1_", "1__0", "0_0", "00", "-1", "+1", "0x_ff", "0x1_", "0x1__0", "0X10",
    ];

    // When / Then
    for (lexeme, expected) in valid {
        assert_eq!(
            Address::parse(lexeme).map(Address::get),
            Ok(expected),
            "{lexeme}"
        );
    }
    for lexeme in invalid {
        assert!(Address::parse(lexeme).is_err(), "{lexeme}");
    }
}

#[test]
fn address_formats_canonical_lowercase_hex() {
    // Given
    let inputs = [("0", "0x0"), ("1_000", "0x3e8"), ("0xAB_CD", "0xabcd")];

    // When / Then
    for (lexeme, expected) in inputs {
        assert_eq!(
            Address::parse(lexeme).map(Address::canonical),
            Ok(expected.into())
        );
    }
}

#[test]
fn address_width_handles_the_full_u64_domain_without_shifting_by_64() {
    // Given
    let width = AddressWidth::new(64);
    let maximum = Address::parse("18446744073709551615");
    let exclusive = Address::parse("18446744073709551616");

    // When / Then
    assert_eq!(
        width.map(AddressWidth::exclusive_bound),
        Ok(u128::from(u64::MAX) + 1)
    );
    assert_eq!(
        maximum.and_then(|value| value.checked_for_width(AddressWidth::new(64)?)),
        Ok(u128::from(u64::MAX))
    );
    assert_eq!(
        maximum.and_then(|value| value.checked_u64(AddressWidth::new(64)?)),
        Ok(u64::MAX)
    );
    assert!(
        exclusive
            .and_then(|value| value.checked_u64(AddressWidth::new(64)?))
            .is_err()
    );
}

#[test]
fn power_of_two_helpers_cover_zero_and_high_bits() {
    // Given / When / Then
    assert!(!is_power_of_two(0));
    assert!(is_power_of_two(1));
    assert!(is_power_of_two(1_u128 << 52));
    assert_eq!(exact_log2(1_u128 << 52), Some(52));
    assert_eq!(exact_log2(3), None);
}

#[test]
fn ratio_uses_exact_integer_round_half_up() {
    // Given
    let below_tie = Ratio::new(1, 128);
    let exact_tie = Ratio::new(1, 2_000_000);
    let ordinary = Ratio::new(2, 3);

    // When / Then
    assert_eq!(
        below_tie.and_then(Ratio::decimal_six),
        Ok("0.007813".into())
    );
    assert_eq!(
        exact_tie.and_then(Ratio::decimal_six),
        Ok("0.000001".into())
    );
    assert_eq!(ordinary.and_then(Ratio::decimal_six), Ok("0.666667".into()));
}

#[test]
fn ratio_rejects_zero_denominator_and_checked_overflow() {
    // Given / When / Then
    assert_eq!(Ratio::new(1, 0), Err(RatioError::ZeroDenominator));
    assert_eq!(
        Ratio::new(u128::MAX, 1).and_then(Ratio::decimal_six),
        Err(RatioError::Overflow)
    );
}
