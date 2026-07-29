use std::{
    fs,
    io::Write as _,
    os::unix::{fs::MetadataExt as _, net::UnixStream},
};

use interleave::input::{load_yaml_named, read_named};

#[test]
fn named_file_retains_regular_identity_after_accepted_eof() -> TestResult {
    // Given
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let path = directory.path().join("mapping.yaml");
    fs::write(&path, b"name: mapping\n").map_err(|error| error.to_string())?;
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;

    // When
    let (document, snapshot) = load_yaml_named(&path).map_err(|error| error.to_string())?;

    // Then
    assert!(mapping(&document).is_ok());
    let identity = snapshot
        .identity()
        .ok_or_else(|| "regular-file identity was not retained".to_owned())?;
    assert_eq!(identity.device(), metadata.dev());
    assert_eq!(identity.inode(), metadata.ino());
    Ok(())
}

#[test]
fn named_file_and_stream_share_exact_raw_boundaries() -> TestResult {
    // Given
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let exact = directory.path().join("exact.yaml");
    let too_large = directory.path().join("too-large.yaml");
    let exact_size = u64::try_from(LIMIT).map_err(|error| error.to_string())?;
    let too_large_size = u64::try_from(LIMIT + 1).map_err(|error| error.to_string())?;
    fs::File::create(&exact)
        .and_then(|file| file.set_len(exact_size))
        .map_err(|error| error.to_string())?;
    fs::File::create(&too_large)
        .and_then(|file| file.set_len(too_large_size))
        .map_err(|error| error.to_string())?;

    // When
    let accepted = read_named(&exact).map_err(|error| error.to_string())?;
    let rejected = read_named(&too_large)
        .err()
        .ok_or_else(|| "over-limit file was accepted".to_owned())?;

    // Then
    assert_eq!(accepted.bytes().len(), LIMIT);
    assert_eq!(rejected.observed_raw_bytes(), Some(LIMIT + 1));
    Ok(())
}

#[test]
fn pipe_and_device_like_sources_use_the_common_reader() -> TestResult {
    // Given
    let (mut writer, reader) = UnixStream::pair().map_err(|error| error.to_string())?;
    writer
        .write_all(b"name: piped\n")
        .map_err(|error| error.to_string())?;
    writer
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| error.to_string())?;

    // When
    let stream = read_bounded(reader).map_err(|error| error.to_string())?;
    let device =
        read_named(std::path::Path::new("/dev/null")).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(stream.bytes(), b"name: piped\n");
    assert!(device.bytes().is_empty());
    assert!(device.identity().is_none());
    Ok(())
}

#[test]
fn indicators_inside_comments_quotes_and_block_scalars_are_data() -> TestResult {
    // Given
    let source = b"# &anchor !tag *alias\nquoted: \"& ! * <<\"\nblock: |\n  & ! * <<\n\"<<\": ordinary\n";

    // When
    let document = load_yaml_bytes(source).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(mapping(&document)?.len(), 3);
    Ok(())
}

#[test]
fn earlier_prohibition_beats_later_malformed_utf8() -> TestResult {
    // Given
    let source = b"name: &saved value\nbad: \xff\n";

    // When
    let error = yaml_error(source)?;

    // Then
    let position = error
        .position()
        .ok_or_else(|| "missing winning position".to_owned())?;
    assert_eq!(position.byte_offset(), 6);
    assert_eq!(error.issue().message(), "invalid YAML syntax");
    Ok(())
}

#[test]
fn non_string_keys_never_enter_duplicate_comparison() -> TestResult {
    // Given
    let source = b"1: first\n1: second\n";

    // When
    let error = yaml_error(source)?;

    // Then
    assert!(error.issue().path().as_str().is_empty());
    assert_eq!(error.issue().message(), "invalid YAML syntax");
    assert_eq!(
        error
            .position()
            .ok_or_else(|| "missing key position".to_owned())?
            .byte_offset(),
        0
    );
    Ok(())
}

#[test]
fn explicit_empty_document_is_one_document_and_three_documents_count_exactly() -> TestResult {
    // Given
    let empty_document = b"---\n";
    let three_documents = b"---\na: 1\n---\nb: 2\n---\nc: 3\n";

    // When
    let accepted = load_yaml_bytes(empty_document);
    let rejected = yaml_error(three_documents)?;

    // Then
    assert!(accepted.is_ok());
    assert_eq!(
        rejected.issue().message(),
        "expected exactly one YAML document, found 3"
    );
    Ok(())
}

#[test]
fn lowercase_booleans_are_distinct_from_yaml_1_1_words() -> TestResult {
    // Given
    let source = b"true_value: true\nfalse_value: false\nyes_value: yes\non_value: on\n";

    // When
    let document = load_yaml_bytes(source).map_err(|error| error.to_string())?;

    // Then
    let entries = mapping(&document)?;
    assert_eq!(scalar_at(entries, 0)?.kind(), ScalarKind::Boolean);
    assert_eq!(scalar_at(entries, 1)?.kind(), ScalarKind::Boolean);
    assert_eq!(scalar_at(entries, 2)?.kind(), ScalarKind::String);
    assert_eq!(scalar_at(entries, 3)?.kind(), ScalarKind::String);
    Ok(())
}

#[test]
fn io_failure_is_one_input_io_issue() -> TestResult {
    // Given
    struct FailedReader;
    impl std::io::Read for FailedReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
        }
    }

    // When
    let error = read_bounded(FailedReader)
        .err()
        .ok_or_else(|| "failed reader was accepted".to_owned())?;

    // Then
    assert_eq!(error.issue().code(), IssueCode::InputIo);
    assert_eq!(error.issue().message(), "input could not be read");
    Ok(())
}

#[test]
fn normative_template_assets_parse_without_schema_coercion() -> TestResult {
    // Given
    let mapping_template = include_bytes!("../../assets/templates/mapping.yaml");
    let scenario_template = include_bytes!("../../assets/templates/scenario.yaml");

    // When
    let mapping_document =
        load_yaml_bytes(mapping_template).map_err(|error| error.to_string())?;
    let scenario_document =
        load_yaml_bytes(scenario_template).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(mapping(&mapping_document)?.len(), 5);
    assert_eq!(mapping(&scenario_document)?.len(), 3);
    Ok(())
}

#[test]
fn standalone_anchor_merge_and_explicit_tag_are_each_rejected() -> TestResult {
    // Given
    let sources = [
        b"name: &saved value\n".as_slice(),
        b"case:\n  <<: value\n".as_slice(),
        b"name: !!str value\n".as_slice(),
    ];

    // When
    let errors: Vec<_> = sources
        .into_iter()
        .map(yaml_error)
        .collect::<TestResult<_>>()?;

    // Then
    assert!(errors.iter().all(|error| {
        error.issue().code() == IssueCode::InputYamlParse
            && error.issue().message() == "invalid YAML syntax"
    }));
    Ok(())
}

#[test]
fn duplicate_paths_escape_terminal_control_and_line_separator_scalars() -> TestResult {
    // Given
    let source = b"\"\\u0085\\u2028\": first\n\"\\u0085\\u2028\": second\n";

    // When
    let error = yaml_error(source)?;

    // Then
    assert_eq!(
        error.issue().path().as_str(),
        "[\"\\u0085\\u2028\"]"
    );
    assert_eq!(
        error.issue().message(),
        "duplicate key \"\\u0085\\u2028\""
    );
    Ok(())
}
