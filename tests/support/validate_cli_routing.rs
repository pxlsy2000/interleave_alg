#[test]
fn structural_failure_routes_text_to_stderr_and_leaves_output_untouched() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(&directory, "bad.yaml", "schema_version: [\n")?;
    let destination = directory.path().join("report.txt");
    fs::write(&destination, b"original\n")?;
    let output_path = destination.to_string_lossy().into_owned();

    // When
    let output = interleave(&[
        "validate",
        "--spec",
        &mapping,
        "--output",
        &output_path,
        "--force",
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("[input.yaml_parse]"));
    assert_eq!(
        fs::read(&destination)?,
        b"original\n"
    );
    Ok(())
}

#[test]
fn structural_failure_routes_json_envelope_to_the_selected_destination() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(&directory, "bad.yaml", "unknown: true\n")?;
    let destination = directory.path().join("report.json");
    let output_path = destination.to_string_lossy().into_owned();

    // When
    let output = interleave(&[
        "validate",
        "--spec",
        &mapping,
        "--format",
        "json",
        "--output",
        &output_path,
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let bytes = fs::read(&destination)?;
    let envelope: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(json_at(&envelope, "/status")?, "fail");
    assert!(json_at(&envelope, "/result")?.is_null());
    Ok(())
}

#[test]
fn output_preflight_precedes_yaml_parsing_after_the_input_snapshot_is_read() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(&directory, "bad.yaml", "schema_version: [\n")?;
    let destination = directory.path().join("report.json");
    fs::write(&destination, b"original\n")?;
    let output_path = destination.to_string_lossy().into_owned();

    // When
    let output = interleave(&[
        "validate",
        "--spec",
        &mapping,
        "--format",
        "json",
        "--output",
        &output_path,
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[output.exists]"));
    assert!(!stderr.contains("[input.yaml_parse]"));
    assert_eq!(
        fs::read(&destination)?,
        b"original\n"
    );
    Ok(())
}

#[test]
fn special_and_input_alias_outputs_win_before_malformed_yaml_without_temp_residue() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping_path = directory.path().join("bad.yaml");
    fs::write(&mapping_path, b"schema_version: [\n")?;
    let mapping = mapping_path.to_string_lossy().into_owned();
    let regular = directory.path().join("regular");
    let link_target = directory.path().join("link-target");
    let link = directory.path().join("link");
    let fifo = directory.path().join("fifo");
    let alias = directory.path().join("mapping-alias");
    fs::write(&regular, b"regular\n")?;
    fs::write(&link_target, b"link-target\n")?;
    symlink(&link_target, &link)?;
    mkfifoat(CWD, &fifo, Mode::RUSR | Mode::WUSR)?;
    fs::hard_link(&mapping_path, &alias)?;
    let cases = [
        (&regular, "[output.exists]"),
        (&link, "[output.invalid_target]"),
        (&fifo, "[output.invalid_target]"),
        (&alias, "[output.exists]"),
    ];

    // When
    let outputs = cases
        .iter()
        .map(|(path, _)| {
            interleave(&[
                "validate",
                "--spec",
                &mapping,
                "--format",
                "json",
                "--output",
                &path.to_string_lossy(),
            ])
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Then
    for ((_, code), output) in cases.iter().zip(outputs) {
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(code));
        assert!(!stderr.contains("[input.yaml_parse]"));
    }
    assert_eq!(
        fs::read(&mapping_path)?,
        b"schema_version: [\n"
    );
    assert_eq!(
        fs::read(&regular)?,
        b"regular\n"
    );
    assert_eq!(
        fs::read(&link_target)?,
        b"link-target\n"
    );
    let temp_count = fs::read_dir(directory.path())?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".interleave.tmp.")
        })
        .count();
    assert_eq!(temp_count, 0);
    Ok(())
}

#[test]
fn stdin_mapping_is_supported_and_inaccessible_file_is_an_io_failure() -> TestResult {
    // Given
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("interleave"));
    command
        .args(["validate", "--spec", "-", "--format", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| std::io::Error::other("stdin pipe must exist"))?
        .write_all(NON_NATURAL_MAPPING.as_bytes())
        ?;

    // When
    let stdin_output = child.wait_with_output()?;
    let inaccessible = interleave(&[
        "validate",
        "--spec",
        "/definitely/missing/interleave-mapping.yaml",
        "--format",
        "json",
    ])?;

    // Then
    assert_eq!(stdin_output.status.code(), Some(0));
    assert!(stdin_output.stderr.is_empty());
    assert_eq!(inaccessible.status.code(), Some(1));
    assert!(inaccessible.stdout.is_empty());
    assert!(String::from_utf8_lossy(&inaccessible.stderr).contains("[input.io]"));
    Ok(())
}

#[test]
fn successful_validate_refuses_then_force_replaces_an_existing_report() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(&directory, "mapping.yaml", NON_NATURAL_MAPPING)?;
    let destination = directory.path().join("report.json");
    fs::write(&destination, b"original\n")?;
    let output_path = destination.to_string_lossy().into_owned();

    // When
    let refused = interleave(&[
        "validate",
        "--spec",
        &mapping,
        "--format",
        "json",
        "--output",
        &output_path,
    ])?;
    let forced = interleave(&[
        "validate",
        "--spec",
        &mapping,
        "--format",
        "json",
        "--output",
        &output_path,
        "--force",
    ])?;

    // Then
    assert_eq!(refused.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("[output.exists]"));
    assert_eq!(forced.status.code(), Some(0));
    assert!(forced.stdout.is_empty());
    assert!(forced.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&fs::read(&destination)?)?;
    assert_eq!(json_at(&envelope, "/status")?, "warning");
    Ok(())
}
