fn sequence(count: usize, multiplier: usize) -> String {
    (0..count)
        .map(|value| (value * multiplier).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn positive_sequence(count: usize) -> String {
    (1..=count)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn sweep_scenario(bases: usize, strides: usize, accesses: u64, windows: &str) -> String {
    format!(
        "schema_version: 1\n\
         defaults:\n  accesses: {accesses}\n  window_sizes: [{windows}]\n\
         cases:\n  - name: limit\n    kind: sweep\n\
         \x20   base_bytes: [{}]\n    stride_bytes: [{}]\n",
        sequence(bases, 1),
        sequence(strides, 1)
    )
}

fn target_count_mapping(target_count: usize) -> Result<String, std::fmt::Error> {
    let target_bits = target_count.trailing_zeros();
    let mut rows = String::new();
    for bit in 0..target_bits {
        writeln!(rows, "      - [{bit}]")?;
    }
    Ok(format!(
        "schema_version: 1\nname: target-cap\naddress:\n  width_bits: 20\n\
         \x20 granule_bytes: 1\ntargets:\n  count: {target_count}\n\
         mapping:\n  m:\n    rows:\n{rows}  l:\n    mode: preserve_high\n"
    ))
}

#[test]
fn all_five_global_resource_caps_fail_as_scenario_errors() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let natural = write_source(&directory, "natural.yaml", NATURAL_MAPPING)?;
    let targets = write_source(&directory, "targets.yaml", &target_count_mapping(128)?)?;
    let window_values = positive_sequence(101);
    let qk_values = positive_sequence(1001);
    let cases = [
        (
            &natural,
            sweep_scenario(101, 100, 1, "1"),
            "expected integer <= 10000, observed 10100",
        ),
        (
            &natural,
            sweep_scenario(100, 100, 10_001, "1"),
            "expected integer <= 100000000, observed 100010000",
        ),
        (
            &targets,
            sweep_scenario(100, 100, 1, "1"),
            "expected integer <= 1000000, observed 1280000",
        ),
        (
            &natural,
            sweep_scenario(100, 100, 101, &window_values),
            "expected integer <= 1000000, observed 1010000",
        ),
        (
            &natural,
            format!(
                "schema_version: 1\ndefaults:\n  accesses: 100000\n  window_sizes: \
                 [{qk_values}]\ncases:\n  - name: qk\n    kind: stride\n    base_bytes: 0\n\
                 \x20   stride_bytes: 0\n"
            ),
            "expected sum(Q*K) <= 100000000, observed 100100000",
        ),
    ];

    // When / Then
    for (index, (mapping, body, message)) in cases.iter().enumerate() {
        let scenario = write_source(&directory, &format!("cap-{index}.yaml"), body)?;
        let output = run_json(mapping, &scenario, &[])?;
        assert_eq!(output.status.code(), Some(3), "cap fixture {index}");
        assert!(output.stderr.is_empty(), "cap fixture {index}");
        let envelope: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            json_at(&envelope, "/errors")?
                .as_array()
                .map(Vec::len),
            Some(1),
            "cap fixture {index}"
        );
        assert_eq!(
            json_at(&envelope, "/errors/0/code")?,
            "scenario.invalid",
            "cap fixture {index}"
        );
        assert_eq!(
            json_at(&envelope, "/errors/0/path")?,
            "cases",
            "cap fixture {index}"
        );
        assert_eq!(
            json_at(&envelope, "/errors/0/message")?,
            message,
            "cap fixture {index}"
        );
        assert_eq!(
            json_at(&envelope, "/result")?,
            &Value::Null,
            "cap fixture {index}"
        );
    }
    Ok(())
}

#[test]
fn per_case_window_and_stream_caps_fail_before_analysis() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_source(&directory, "mapping.yaml", NATURAL_MAPPING)?;
    let windows = positive_sequence(1025);
    let mut stream_rows = String::new();
    for index in 0..4097 {
        writeln!(
            stream_rows,
            "      - name: s{index}\n        base_bytes: 0\n        stride_bytes: 0\n\
             \x20       accesses: 1"
        )?;
    }
    let cases = [
        (
            format!(
                "schema_version: 1\ndefaults:\n  accesses: 1025\n  window_sizes: [{windows}]\n\
                 cases:\n  - name: windows\n    kind: stride\n    base_bytes: 0\n    stride_bytes: 0\n"
            ),
            "defaults.window_sizes",
            "expected integer <= 1024, observed 1025",
        ),
        (
            format!(
                "schema_version: 1\ndefaults:\n  accesses: 1\n  window_sizes: [1]\n\
                 cases:\n  - name: streams\n    kind: multi_stream\n    schedule: round_robin\n\
                 \x20   streams:\n{stream_rows}"
            ),
            "cases[0].streams",
            "expected integer <= 4096, observed 4097",
        ),
    ];

    // When / Then
    for (index, (body, path, message)) in cases.iter().enumerate() {
        let scenario = write_source(&directory, &format!("local-{index}.yaml"), body)?;
        let output = run_json(&mapping, &scenario, &[])?;
        assert_eq!(output.status.code(), Some(3));
        assert!(output.stderr.is_empty());
        let envelope: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(json_at(&envelope, "/errors/0/code")?, "scenario.invalid");
        assert_eq!(json_at(&envelope, "/errors/0/path")?, path);
        assert_eq!(json_at(&envelope, "/errors/0/message")?, message);
        assert_eq!(json_at(&envelope, "/result")?, &Value::Null);
    }
    Ok(())
}
