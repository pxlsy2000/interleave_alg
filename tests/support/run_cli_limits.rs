fn sequence(count: usize, multiplier: usize) -> String {
    (0..count)
        .map(|value| (value * multiplier).to_string())
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
    let window_values = sequence(101, 1);
    let qk_values = sequence(1001, 1);
    let cases = [
        (&natural, sweep_scenario(101, 100, 1, "1")),
        (&natural, sweep_scenario(100, 100, 10_001, "1")),
        (&targets, sweep_scenario(100, 100, 1, "1")),
        (&natural, sweep_scenario(100, 100, 101, &window_values)),
        (
            &natural,
            format!(
                "schema_version: 1\ndefaults:\n  accesses: 100000\n  window_sizes: \
                 [{qk_values}]\ncases:\n  - name: qk\n    kind: stride\n    base_bytes: 0\n\
                 \x20   stride_bytes: 0\n"
            ),
        ),
    ];

    // When / Then
    for (index, (mapping, body)) in cases.iter().enumerate() {
        let scenario = write_source(&directory, &format!("cap-{index}.yaml"), body)?;
        let output = run_json(mapping, &scenario, &[])?;
        assert_eq!(output.status.code(), Some(3), "cap fixture {index}");
        let envelope: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            json_at(&envelope, "/errors/0/code")?,
            "scenario.invalid",
            "cap fixture {index}"
        );
        assert_eq!(json_at(&envelope, "/result")?, &Value::Null);
    }
    Ok(())
}

#[test]
fn per_case_window_and_stream_caps_fail_before_analysis() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_source(&directory, "mapping.yaml", NATURAL_MAPPING)?;
    let windows = sequence(1025, 1);
    let mut stream_rows = String::new();
    for index in 0..4097 {
        writeln!(
            stream_rows,
            "      - name: s{index}\n        base_bytes: 0\n        stride_bytes: 0\n\
             \x20       accesses: 1"
        )?;
    }
    let cases = [
        format!(
            "schema_version: 1\ndefaults:\n  accesses: 1025\n  window_sizes: [{windows}]\n\
             cases:\n  - name: windows\n    kind: stride\n    base_bytes: 0\n    stride_bytes: 0\n"
        ),
        format!(
            "schema_version: 1\ndefaults:\n  accesses: 1\n  window_sizes: [1]\n\
             cases:\n  - name: streams\n    kind: multi_stream\n    schedule: round_robin\n\
             \x20   streams:\n{stream_rows}"
        ),
    ];

    // When / Then
    for (index, body) in cases.iter().enumerate() {
        let scenario = write_source(&directory, &format!("local-{index}.yaml"), body)?;
        let output = run_json(&mapping, &scenario, &[])?;
        assert_eq!(output.status.code(), Some(3));
        let envelope: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(json_at(&envelope, "/errors/0/code")?, "scenario.invalid");
    }
    Ok(())
}
