#[test]
fn arbitrary_address_magnitudes_use_range_errors_and_full_canonical_hex() -> TestResult {
    // Given
    let decimal_2_128 = "340282366920938463463374607431768211456".to_owned();
    let hex_2_128 = format!("0x1{}", "0".repeat(32));
    let long_decimal = "9".repeat(2048);
    let long_hex = format!("0x0000A{}", "bC".repeat(1024));
    let cases = [
        (decimal_2_128.clone(), decimal_to_hex_oracle(&decimal_2_128)?),
        (hex_2_128.clone(), hex_2_128),
        (long_decimal.clone(), decimal_to_hex_oracle(&long_decimal)?),
        (long_hex, format!("0xa{}", "bc".repeat(1024))),
    ];
    let width = AddressWidth::new(64)?;

    // When / Then
    for (lexeme, canonical) in cases {
        let Err(issues) = preflight_query_addresses(&[lexeme], width) else {
            return Err("arbitrary out-of-range magnitude must fail".into());
        };
        let issue = issues
            .first()
            .ok_or("out-of-range magnitude must emit one issue")?;
        assert_eq!(
            issue.code(),
            interleave::issue::IssueCode::AddressOutOfRange
        );
        assert_eq!(
            issue.message(),
            format!("address {canonical} is outside the 64-bit range")
        );
    }
    Ok(())
}

#[test]
fn cli_operands_are_not_subject_to_the_yaml_scalar_byte_limit() -> TestResult {
    // Given
    let decimal = "9".repeat(256);
    let canonical = decimal_to_hex_oracle(&decimal)?;

    // When
    let output = interleave(&[
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        &decimal,
        "--format",
        "json",
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json_at(&envelope, "/errors/0/code")?,
        "address.out_of_range"
    );
    assert_eq!(
        json_at(&envelope, "/errors/0/message")?,
        &format!("address {canonical} is outside the 20-bit range")
    );
    assert!(output.stdout.ends_with(b"\n"));
    assert!(!output.stdout.ends_with(b"\n\n"));
    Ok(())
}

#[test]
fn full_u64_magnitudes_pass_before_two_to_the_sixty_four_fails() -> TestResult {
    // Given
    let width = AddressWidth::new(64)?;
    let accepted = [
        "18446744073709551615".to_owned(),
        "0xffffffffffffffff".to_owned(),
    ];
    let rejected = [
        (
            "18446744073709551616".to_owned(),
            "0x10000000000000000",
        ),
        (
            "0x10000000000000000".to_owned(),
            "0x10000000000000000",
        ),
    ];

    // When
    let values = preflight_query_addresses(&accepted, width);

    // Then
    let Ok(values) = values else {
        return Err("u64 maximum values must pass".into());
    };
    assert_eq!(values.len(), 2);
    for (lexeme, canonical) in rejected {
        let Err(issues) = preflight_query_addresses(&[lexeme], width) else {
            return Err("2^64 must be outside a 64-bit Mapping".into());
        };
        let issue = issues.first().ok_or("2^64 must emit one issue")?;
        assert_eq!(
            issue.message(),
            format!("address {canonical} is outside the 64-bit range")
        );
    }
    Ok(())
}

fn decimal_to_hex_oracle(decimal: &str) -> TestResult<String> {
    let mut digits = decimal
        .bytes()
        .map(|byte| byte.checked_sub(b'0').ok_or("decimal oracle digit"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut reversed = Vec::new();
    while digits.iter().any(|digit| *digit != 0) {
        let mut quotient = Vec::with_capacity(digits.len());
        let mut carry = 0_u16;
        for digit in digits {
            let current = carry
                .checked_mul(10)
                .and_then(|value| value.checked_add(u16::from(digit)))
                .ok_or("decimal oracle overflow")?;
            let next = u8::try_from(current / 16)?;
            carry = current % 16;
            if !quotient.is_empty() || next != 0 {
                quotient.push(next);
            }
        }
        reversed.push(u8::try_from(carry)?);
        digits = quotient;
    }
    if reversed.is_empty() {
        reversed.push(0);
    }
    let mut canonical = String::from("0x");
    for digit in reversed.iter().rev().copied() {
        canonical.push(char::from_digit(u32::from(digit), 16).ok_or("hex oracle digit")?);
    }
    Ok(canonical)
}
