//! Consumer-level exact scalar, issue, limit, and exit-routing contracts.

use interleave::error::{BoundaryError, DomainError, ExitClass, InputKind};
use interleave::input::limits::{MAX_ADDRESS_WIDTH_BITS, MAX_GRANULE_BYTES, MAX_RATIO_NUMERATOR};
use interleave::input::scalar::{
    Address, AddressError, AddressWidth, GenericInteger, GenericIntegerError, Ratio, RatioError,
};
use interleave::issue::{Issue, IssueCode, IssuePath};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

#[test]
fn public_address_parser_covers_contract_boundaries() {
    // Given
    let valid = [
        ("0", 0_u128, "0x0"),
        ("1_000", 1_000, "0x3e8"),
        ("0xAB_CD", 0xabcd, "0xabcd"),
        (
            "18446744073709551615",
            u128::from(u64::MAX),
            "0xffffffffffffffff",
        ),
        (
            "18446744073709551616",
            u128::from(u64::MAX) + 1,
            "0x10000000000000000",
        ),
    ];

    // When / Then
    for (lexeme, value, canonical) in valid {
        assert_eq!(Address::parse(lexeme).map(Address::get), Ok(value));
        assert_eq!(
            Address::parse(lexeme).map(Address::canonical),
            Ok(canonical.into())
        );
    }
}

#[test]
fn public_address_parser_rejects_every_adversarial_separator_class() {
    // Given
    let invalid = [
        "_1", "1_", "1__0", "0x_ff", "0x1_", "0x1__0", "00", "-1", "+1",
    ];

    // When / Then
    for (index, lexeme) in invalid.into_iter().enumerate() {
        assert_eq!(Address::parse(lexeme), Err(AddressError::InvalidLexeme));
        let issue = Issue::new(
            IssueCode::AddressInvalid,
            IssuePath::root().field("addresses").index(index),
            "invalid address",
        );
        assert_eq!(issue.code(), IssueCode::AddressInvalid);
        assert_eq!(issue.path().as_str(), format!("addresses[{index}]"));
    }
    assert_eq!(
        Address::parse("340282366920938463463374607431768211456"),
        Err(AddressError::Overflow)
    );
}

#[test]
fn public_generic_integer_parser_is_distinct_from_address_parser() {
    // Given / When / Then
    assert_eq!(
        GenericInteger::parse("1000").map(GenericInteger::get),
        Ok(1_000)
    );
    assert_eq!(
        GenericInteger::parse("0x3e8").map(GenericInteger::get),
        Ok(1_000)
    );
    assert_eq!(
        GenericInteger::parse("1_000"),
        Err(GenericIntegerError::InvalidLexeme)
    );
    assert_eq!(
        GenericInteger::parse("340282366920938463463374607431768211456"),
        Err(GenericIntegerError::Overflow)
    );
}

#[test]
fn width_and_granule_boundaries_remain_exact() {
    // Given
    let width = AddressWidth::new(MAX_ADDRESS_WIDTH_BITS);
    let maximum = Address::parse("18446744073709551615");
    let exclusive = Address::parse("18446744073709551616");
    let supported_granule = GenericInteger::parse("0x10000000000000");
    let unsupported_granule = GenericInteger::parse("0x20000000000000");

    // When / Then
    assert_eq!(
        maximum.and_then(|address| address.checked_u64(width?)),
        Ok(u64::MAX)
    );
    assert_eq!(
        exclusive.and_then(|address| address.checked_u64(width?)),
        Err(AddressError::OutOfRange {
            width_bits: MAX_ADDRESS_WIDTH_BITS
        })
    );
    assert_eq!(
        supported_granule.map(GenericInteger::get),
        Ok(MAX_GRANULE_BYTES)
    );
    assert_eq!(
        unsupported_granule.map(GenericInteger::get),
        Ok(1_u128 << 53)
    );
}

#[test]
fn ratios_cover_half_up_and_largest_v1_numerator() {
    // Given
    let tie = Ratio::new(1, 2_000_000);
    let largest = Ratio::new(u128::from(MAX_RATIO_NUMERATOR), 10_000_000);

    // When / Then
    assert_eq!(tie.and_then(Ratio::decimal_six), Ok("0.000001".into()));
    assert_eq!(
        largest.and_then(Ratio::decimal_six),
        Ok("65536.000000".into())
    );
    assert_eq!(
        Ratio::new(u128::MAX, 1).and_then(Ratio::decimal_six),
        Err(RatioError::Overflow)
    );
}

#[test]
fn typed_errors_cover_every_process_exit_class() {
    // Given / When / Then
    assert_eq!(ExitClass::Success.code(), 0);
    assert_eq!(BoundaryError::OutputIo.exit_class().code(), 1);
    assert_eq!(DomainError::InvalidMapping.exit_class().code(), 2);
    assert_eq!(
        DomainError::InvalidInput {
            kind: InputKind::Scenario,
        }
        .exit_class()
        .code(),
        3
    );
    assert_eq!(DomainError::AnalysisFailed.exit_class().code(), 4);
}

proptest! {
    #[test]
    fn canonical_parse_format_roundtrips_arbitrary_u64(value in any::<u64>()) {
        // Given
        let decimal = value.to_string();

        // When
        let address = Address::parse(&decimal)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let canonical = address.canonical();
        let reparsed = Address::parse(&canonical)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let lowercase = canonical.to_ascii_lowercase();

        // Then
        prop_assert_eq!(reparsed.get(), u128::from(value));
        prop_assert!(canonical.starts_with("0x"));
        prop_assert!(!canonical.contains('_'));
        prop_assert_eq!(canonical, lowercase);
    }

    #[test]
    fn scalar_parsers_never_panic_on_arbitrary_text(text in any::<String>()) {
        // Given / When
        let generic = GenericInteger::parse(&text);
        let address = Address::parse(&text);

        // Then
        prop_assert!(generic.is_ok() || generic.is_err());
        prop_assert!(address.is_ok() || address.is_err());
    }
}
