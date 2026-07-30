use crate::{
    input::{
        limits::MAX_QUERY_ADDRESSES,
        scalar::{Address, AddressMagnitude, AddressMagnitudeError, AddressWidth},
    },
    issue::{Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase, canonical_json_string},
};

/// Parses and range-checks every command address before any Mapping calculation.
///
/// # Errors
/// Returns all source-ordered address issues, or one resource/allocation issue.
pub fn preflight_query_addresses<T: AsRef<str>>(
    lexemes: &[T],
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
    for (index, operand) in lexemes.iter().enumerate() {
        let lexeme = operand.as_ref();
        let order = IssueOrderKey::new(IssuePhase::Address, index, 0);
        match AddressMagnitude::parse(lexeme) {
            Ok(magnitude) => match magnitude.in_width(width) {
                Some(address) => addresses.push(address),
                None => issues.push(
                    Issue::new(
                        IssueCode::AddressOutOfRange,
                        IssuePath::root().field("addresses").index(index),
                        format!(
                            "address {} is outside the {}-bit range",
                            magnitude.canonical(),
                            width.get()
                        ),
                    )
                    .with_order(order),
                ),
            },
            Err(AddressMagnitudeError::InvalidLexeme) => issues.push(
                Issue::new(
                    IssueCode::AddressInvalid,
                    IssuePath::root().field("addresses").index(index),
                    format!("invalid address {}", canonical_json_string(lexeme)),
                )
                .with_order(order),
            ),
            Err(AddressMagnitudeError::AnalysisFailed) => issues.push(
                Issue::new(
                    IssueCode::AnalysisFailed,
                    IssuePath::root(),
                    "analysis could not be completed",
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
