use interleave::{
    error::ExitClass,
    input::load_yaml_bytes,
    issue::IssueCode,
    mapping::{LocalAddressMode, decode_mapping},
};

type TestResult<T = ()> = Result<T, String>;

fn decode(source: &str) -> Result<interleave::mapping::MappingModel, String> {
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    decode_mapping(&document).map_err(|error| {
        error
            .issues()
            .iter()
            .map(|issue| {
                format!(
                    "{}|{}|{}",
                    issue.code().as_str(),
                    issue.path().as_str(),
                    issue.message()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn errors(source: &str) -> TestResult<interleave::mapping::MappingDecodeError> {
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    decode_mapping(&document)
        .err()
        .ok_or_else(|| "mapping was unexpectedly accepted".to_owned())
}

const fn complete() -> &'static str {
    "schema_version: 1\nname: example-4-target\naddress:\n  width_bits: 20\n  granule_bytes: 64\ntargets:\n  count: 4\nmapping:\n  m:\n    rows:\n      - [0, 4, 8]\n      - [1, 5, 9]\n  l:\n    mode: preserve_high\n"
}

fn issue_at(
    error: &interleave::mapping::MappingDecodeError,
    index: usize,
) -> TestResult<&interleave::issue::Issue> {
    error
        .issues()
        .get(index)
        .ok_or_else(|| format!("missing issue at {index}"))
}

#[test]
fn decodes_complete_mapping_and_derived_dimensions() -> TestResult {
    // Given
    let source = complete();

    // When
    let model = decode(source)?;

    // Then
    assert_eq!(model.name().as_str(), "example-4-target");
    assert_eq!(model.address_width().get(), 20);
    assert_eq!(model.granule_bytes().get(), 64);
    assert_eq!(model.target_count().get(), 4);
    assert_eq!(model.offset_bits(), 6);
    assert_eq!(model.line_bits(), 14);
    assert_eq!(model.target_bits(), 2);
    assert_eq!(model.local_address_bits(), 12);
    let first_taps: Vec<_> = model
        .target_rows()
        .first()
        .ok_or_else(|| "missing first Target row".to_owned())?
        .taps()
        .iter()
        .map(|tap| tap.get())
        .collect();
    assert_eq!(first_taps, [0, 4, 8]);
    assert_eq!(model.local_address_mode(), LocalAddressMode::PreserveHigh);
    Ok(())
}

#[test]
fn accepts_zero_dimensional_boundaries() -> TestResult {
    // Given
    let n_one = "schema_version: 1\nname: one\naddress: { width_bits: 6, granule_bytes: 64 }\ntargets: { count: 1 }\nmapping:\n  m: { rows: [] }\n  l: { mode: preserve_high }\n";
    let all_target_bits = "schema_version: 1\nname: all\naddress: { width_bits: 4, granule_bytes: 1 }\ntargets: { count: 16 }\nmapping:\n  m: { rows: [[0], [1], [2], [3]] }\n  l: { mode: explicit, rows: [] }\n";

    // When
    let first = decode(n_one)?;
    let second = decode(all_target_bits)?;

    // Then
    assert_eq!(
        (
            first.line_bits(),
            first.target_bits(),
            first.local_address_bits()
        ),
        (0, 0, 0)
    );
    assert!(first.target_rows().is_empty());
    assert_eq!(
        (
            second.line_bits(),
            second.target_bits(),
            second.local_address_bits()
        ),
        (4, 4, 0)
    );
    assert_eq!(
        second
            .explicit_local_rows()
            .map(<[interleave::mapping::XorRow]>::len),
        Some(0)
    );
    Ok(())
}

#[test]
fn accepts_a64_g1_and_tap63() -> TestResult {
    // Given
    let source = "schema_version: 1\nname: bit63\naddress: { width_bits: 64, granule_bytes: 1 }\ntargets: { count: 2 }\nmapping:\n  m: { rows: [[63]] }\n  l: { mode: preserve_high }\n";

    // When
    let model = decode(source)?;

    // Then
    assert_eq!(model.line_bits(), 64);
    let taps: Vec<_> = model
        .target_rows()
        .first()
        .ok_or_else(|| "missing bit-63 row".to_owned())?
        .taps()
        .iter()
        .map(|tap| tap.get())
        .collect();
    assert_eq!(taps, [63]);
    Ok(())
}

#[test]
fn reports_unknown_raw_key_and_missing_fields_in_contract_order() -> TestResult {
    // Given
    let source = "bad.key: 1\nname: ok\naddress:\n  extra: 1\ntargets: {}\nmapping: {}\n";

    // When
    let error = errors(source)?;

    // Then
    let got: Vec<_> = error
        .issues()
        .iter()
        .map(|issue| (issue.code(), issue.path().as_str(), issue.message()))
        .collect();
    assert_eq!(
        got,
        [
            (
                IssueCode::InputUnknownField,
                "[\"bad.key\"]",
                "unknown field \"bad.key\""
            ),
            (
                IssueCode::InputUnknownField,
                "address[\"extra\"]",
                "unknown field \"extra\""
            ),
            (
                IssueCode::InputInvalidValue,
                "address.width_bits",
                "expected required field, observed missing"
            ),
            (
                IssueCode::InputInvalidValue,
                "address.granule_bytes",
                "expected required field, observed missing"
            ),
            (
                IssueCode::InputInvalidValue,
                "targets.count",
                "expected required field, observed missing"
            ),
            (
                IssueCode::InputInvalidValue,
                "mapping.m",
                "expected required field, observed missing"
            ),
            (
                IssueCode::InputInvalidValue,
                "mapping.l",
                "expected required field, observed missing"
            ),
            (
                IssueCode::InputInvalidValue,
                "schema_version",
                "expected required field, observed missing"
            ),
        ]
    );
    assert_eq!(error.exit_class(), ExitClass::Mapping);
    Ok(())
}
