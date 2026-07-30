use crate::{
    input::{
        limits::MAX_QUERY_ADDRESSES,
        scalar::{Address, AddressError, AddressWidth},
    },
    issue::{Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase, canonical_json_string},
};

/// Parses and range-checks every command address before any Mapping calculation.
///
/// # Errors
/// Returns all source-ordered address issues, or one resource/allocation issue.
pub fn preflight_query_addresses(
    lexemes: &[String],
    width: AddressWidth,
) -> Result<Vec<Address>, Vec<Issue>> {
    if lexemes.len() > MAX_QUERY_ADDRESSES {
        return Err(vec![Issue::new(
            IssueCode::InputInvalidValue,
            IssuePath::root(),
            format!(
                "expected at most {MAX_QUERY_ADDRESSES} query addresses, observed {}",
                lexemes.len()
            ),
        )]);
    }

    let mut addresses = Vec::new();
    if addresses.try_reserve_exact(lexemes.len()).is_err() {
        return Err(vec![Issue::new(
            IssueCode::AnalysisFailed,
            IssuePath::root(),
            "analysis could not be completed",
        )]);
    }
    let mut issues = Vec::new();
    for (index, lexeme) in lexemes.iter().enumerate() {
        let order = IssueOrderKey::new(IssuePhase::Address, index, 0);
        match Address::parse(lexeme) {
            Ok(address) => match address.checked_for_width(width) {
                Ok(_) => addresses.push(address),
                Err(AddressError::OutOfRange { .. }) => issues.push(
                    Issue::new(
                        IssueCode::AddressOutOfRange,
                        IssuePath::root().field("addresses").index(index),
                        format!(
                            "address {} is outside the {}-bit range",
                            address.canonical(),
                            width.get()
                        ),
                    )
                    .with_order(order),
                ),
                Err(
                    AddressError::InvalidLexeme
                    | AddressError::Overflow
                    | AddressError::InvalidWidth { .. },
                ) => issues.push(
                    Issue::new(
                        IssueCode::AnalysisFailed,
                        IssuePath::root(),
                        "analysis could not be completed",
                    )
                    .with_order(order),
                ),
            },
            Err(
                AddressError::InvalidLexeme
                | AddressError::Overflow
                | AddressError::InvalidWidth { .. }
                | AddressError::OutOfRange { .. },
            ) => issues.push(
                Issue::new(
                    IssueCode::AddressInvalid,
                    IssuePath::root().field("addresses").index(index),
                    format!("invalid address {}", canonical_json_string(lexeme)),
                )
                .with_order(order),
            ),
        }
    }
    if issues.is_empty() {
        Ok(addresses)
    } else {
        Err(issues)
    }
}
