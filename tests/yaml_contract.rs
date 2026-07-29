//! Integration contract for bounded, strict YAML ingestion.

include!("support/yaml.rs");
include!("support/yaml_adversarial.rs");
include!("support/yaml_core.rs");
include!("support/yaml_scalar_limit.rs");

use std::io::Cursor;

use interleave::{
    input::{ScalarKind, ScalarStyle, load_yaml_reader, read_bounded},
    issue::IssueCode,
};

const LIMIT: usize = 16 * 1024 * 1024;

#[test]
fn retains_order_styles_and_byte_spans_when_block_yaml_is_valid() -> TestResult {
    // Given
    let source = fixture("valid")?;

    // When
    let document = load_yaml_bytes(source).map_err(|error| error.to_string())?;

    // Then
    let entries = mapping(&document)?;
    let keys: Vec<_> = entries.iter().map(|entry| entry.key().value()).collect();
    assert_eq!(
        keys,
        ["schema_version", "name", "address", "rows", "enabled"]
    );
    assert!(entries.windows(2).all(|pair| match pair {
        [left, right] => {
            left.key().span().start().byte_offset() < right.key().span().start().byte_offset()
        }
        _ => false,
    }));
    assert_eq!(scalar_at(entries, 0)?.kind(), ScalarKind::Integer);
    assert_eq!(scalar_at(entries, 1)?.kind(), ScalarKind::String);
    assert_eq!(scalar_at(entries, 1)?.value(), "on");
    assert_eq!(scalar_at(entries, 4)?.kind(), ScalarKind::Boolean);
    Ok(())
}

#[test]
fn distinguishes_plain_and_quoted_numeric_scalars() -> TestResult {
    // Given
    let source = b"plain: 1\nquoted: \"1\"\nyes_value: yes\non_value: on\n";

    // When
    let document = load_yaml_bytes(source).map_err(|error| error.to_string())?;

    // Then
    let entries = mapping(&document)?;
    assert_eq!(scalar_at(entries, 0)?.kind(), ScalarKind::Integer);
    assert_eq!(scalar_at(entries, 0)?.style(), ScalarStyle::Plain);
    assert_eq!(scalar_at(entries, 1)?.kind(), ScalarKind::String);
    assert_eq!(scalar_at(entries, 1)?.style(), ScalarStyle::DoubleQuoted);
    assert_eq!(scalar_at(entries, 2)?.kind(), ScalarKind::String);
    assert_eq!(scalar_at(entries, 3)?.kind(), ScalarKind::String);
    Ok(())
}

#[test]
fn accepts_nested_flow_sequences_but_rejects_a_flow_root() -> TestResult {
    // Given
    let nested = fixture("nested_flow")?;

    // When
    let accepted = load_yaml_bytes(nested);
    let rejected = yaml_error(fixture("flow_root")?)?;

    // Then
    assert!(accepted.is_ok());
    assert_eq!(rejected.issue().code(), IssueCode::InputYamlParse);
    assert_eq!(rejected.issue().message(), "invalid YAML syntax");
    Ok(())
}

#[test]
fn rejects_each_prohibited_yaml_form_with_one_stable_issue() -> TestResult {
    // Given
    let cases = ["alias", "merge", "tag", "non_string"];

    // When
    let errors: Vec<_> = cases
        .into_iter()
        .map(|name| fixture(name).and_then(yaml_error))
        .collect::<TestResult<_>>()?;

    // Then
    assert!(errors.iter().all(|error| {
        error.issue().code() == IssueCode::InputYamlParse
            && error.issue().path().as_str().is_empty()
            && error.issue().message() == "invalid YAML syntax"
    }));
    Ok(())
}

#[test]
fn duplicate_key_uses_second_occurrence_and_terminal_safe_path() -> TestResult {
    // Given
    let source = b"address:\n  \"bad.key\\n\": first\n  \"bad.key\\n\": second\n";

    // When
    let error = yaml_error(source)?;

    // Then
    assert_eq!(error.issue().code(), IssueCode::InputYamlParse);
    assert_eq!(error.issue().path().as_str(), "address[\"bad.key\\n\"]");
    assert_eq!(error.issue().message(), "duplicate key \"bad.key\\n\"");
    let position = error
        .position()
        .ok_or_else(|| "missing YAML position".to_owned())?;
    let expected = source
        .windows(b"\"bad.key\\n\"".len())
        .rposition(|window| window == b"\"bad.key\\n\"")
        .ok_or_else(|| "missing second key".to_owned())?;
    assert_eq!(position.byte_offset(), expected);
    Ok(())
}

#[test]
fn rejects_empty_and_multiple_document_streams() -> TestResult {
    // Given
    let empty = b"# only a comment\n";
    let multiple = fixture("two_documents")?;

    // When
    let empty_error = yaml_error(empty)?;
    let multiple_error = yaml_error(multiple)?;

    // Then
    assert_eq!(
        empty_error.issue().message(),
        "expected exactly one YAML document, found 0"
    );
    assert_eq!(
        multiple_error.issue().message(),
        "expected exactly one YAML document, found 2"
    );
    Ok(())
}

#[test]
fn enforces_utf8_and_bom_policy_before_yaml() -> TestResult {
    // Given
    let accepted = b"\xef\xbb\xbfname: valid\n";
    let rejected = [
        b"\xff\xfename: invalid\n".as_slice(),
        b"name: \xff\n".as_slice(),
        b"\xef\xbb\xbf\xef\xbb\xbfname: invalid\n".as_slice(),
        b"name: \xef\xbb\xbf\n".as_slice(),
    ];

    // When
    let accepted_result = load_yaml_bytes(accepted);
    let errors: Vec<_> = rejected
        .into_iter()
        .map(yaml_error)
        .collect::<TestResult<_>>()?;

    // Then
    assert!(accepted_result.is_ok());
    assert!(errors.iter().all(|error| {
        error.issue().code() == IssueCode::InputYamlParse
            && error.issue().message() == "invalid YAML syntax"
    }));
    Ok(())
}

#[test]
fn converts_character_markers_to_utf8_byte_offsets() -> TestResult {
    // Given
    let source = "name: ok\n重复: first\n重复: second\n".as_bytes();

    // When
    let error = yaml_error(source)?;

    // Then
    let expected = "name: ok\n重复: first\n".len();
    let position = error
        .position()
        .ok_or_else(|| "missing YAML position".to_owned())?;
    assert_eq!(position.byte_offset(), expected);
    Ok(())
}

#[test]
fn raw_limit_accepts_exact_eof_and_stops_on_the_next_byte() -> TestResult {
    // Given
    let exact = vec![b' '; LIMIT];
    let too_large = vec![b' '; LIMIT + 8];

    // When
    let accepted = read_bounded(Cursor::new(exact)).map_err(|error| error.to_string())?;
    let rejected = read_bounded(Cursor::new(too_large))
        .err()
        .ok_or_else(|| "over-limit source was accepted".to_owned())?;

    // Then
    assert_eq!(accepted.bytes().len(), LIMIT);
    assert_eq!(rejected.issue().code(), IssueCode::InputInvalidValue);
    assert_eq!(
        rejected.issue().message(),
        "expected at most 16777216 raw bytes, observed 16777217"
    );
    Ok(())
}

#[test]
fn stream_seam_uses_the_same_bound_without_read_ahead() -> TestResult {
    // Given
    let (reader, consumed) = ObservedReader::new(LIMIT + 50);

    // When
    let error = load_yaml_reader(reader)
        .err()
        .ok_or_else(|| "over-limit stream was accepted".to_owned())?;

    // Then
    assert_eq!(error.issue().code(), IssueCode::InputInvalidValue);
    assert_eq!(error.observed_raw_bytes(), Some(LIMIT + 1));
    assert_eq!(consumed.get(), LIMIT + 1);
    Ok(())
}

#[test]
fn earliest_byte_wins_before_same_position_priority() -> TestResult {
    // Given: an anchor occurs before a malformed trailing flow collection.
    let earlier_anchor = b"name: &saved value\nbroken: [1,\n";
    // Given: tag and anchor start the same node; source order remains decisive.
    let tag_then_anchor = b"name: !custom &saved value\n";

    // When
    let first = yaml_error(earlier_anchor)?;
    let second = yaml_error(tag_then_anchor)?;

    // Then
    assert_eq!(
        first
            .position()
            .ok_or_else(|| "missing first position".to_owned())?
            .byte_offset(),
        6
    );
    assert_eq!(
        second
            .position()
            .ok_or_else(|| "missing second position".to_owned())?
            .byte_offset(),
        6
    );
    assert_eq!(first.issue().message(), "invalid YAML syntax");
    assert_eq!(second.issue().message(), "invalid YAML syntax");
    Ok(())
}
