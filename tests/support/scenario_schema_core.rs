use interleave::{
    error::ExitClass,
    input::load_yaml_bytes,
    issue::IssueCode,
    scenario::{ScenarioCaseKind, ScenarioDecodeError, ScenarioModel, Schedule, decode_scenario},
};

type ScenarioTestResult<T = ()> = Result<T, String>;

fn scenario_decode(source: &str) -> Result<ScenarioModel, String> {
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    decode_scenario(&document).map_err(|error| render_scenario_error(&error))
}

fn scenario_errors(source: &str) -> ScenarioTestResult<ScenarioDecodeError> {
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    decode_scenario(&document)
        .err()
        .ok_or_else(|| "Scenario was unexpectedly accepted".to_owned())
}

fn render_scenario_error(error: &ScenarioDecodeError) -> String {
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
}

fn scenario_case(model: &ScenarioModel, index: usize) -> ScenarioTestResult<&interleave::scenario::ScenarioCase> {
    model
        .cases()
        .get(index)
        .ok_or_else(|| format!("missing case at {index}"))
}

fn scenario_stream(
    streams: &[interleave::scenario::StreamScenario],
    index: usize,
) -> ScenarioTestResult<&interleave::scenario::StreamScenario> {
    streams
        .get(index)
        .ok_or_else(|| format!("missing stream at {index}"))
}

fn scenario_window(
    windows: &[interleave::scenario::WindowSize],
    index: usize,
) -> ScenarioTestResult<interleave::scenario::WindowSize> {
    windows
        .get(index)
        .copied()
        .ok_or_else(|| format!("missing window at {index}"))
}

const fn complete_scenario() -> &'static str {
    r"schema_version: 1
defaults:
  accesses: 4096
  window_sizes: [4, 16, 64]
cases:
  - name: sequential
    kind: stride
    base_bytes: 0
    stride_bytes: 64
  - name: stride-and-phase-sweep
    enabled: true
    kind: sweep
    base_bytes: [0, 64, 128, 192]
    stride_bytes: [64, 128, 256]
    accesses: 2048
    window_sizes: [4, 16, 64, 256]
  - name: two-masters
    kind: multi_stream
    schedule: round_robin
    window_sizes: [4, 16, 64]
    streams:
      - name: cpu
        base_bytes: 0
        stride_bytes: 256
        accesses: 3
      - name: dma
        base_bytes: 0x80
        stride_bytes: 256
        accesses: 2
"
}

#[test]
fn decodes_all_kinds_without_expanding_them() -> ScenarioTestResult {
    // Given
    let source = complete_scenario();

    // When
    let model = scenario_decode(source)?;

    // Then
    assert_eq!(model.defaults().accesses().get(), 4096);
    assert_eq!(
        model
            .defaults()
            .window_sizes()
            .iter()
            .map(|window| window.get())
            .collect::<Vec<_>>(),
        [4, 16, 64]
    );
    assert_eq!(
        model
            .cases()
            .iter()
            .map(|case| case.name().as_str())
            .collect::<Vec<_>>(),
        ["sequential", "stride-and-phase-sweep", "two-masters"]
    );
    assert!(scenario_case(&model, 0)?.enabled());
    match scenario_case(&model, 0)?.kind() {
        ScenarioCaseKind::Stride(value) => {
            assert_eq!(value.base_bytes().canonical(), "0x0");
            assert_eq!(value.stride_bytes().canonical(), "0x40");
            assert_eq!(value.accesses(), None);
        }
        ScenarioCaseKind::Sweep(_) | ScenarioCaseKind::MultiStream(_) => {
            return Err("first case did not retain stride kind".to_owned());
        }
    }
    match scenario_case(&model, 1)?.kind() {
        ScenarioCaseKind::Sweep(value) => {
            assert_eq!(value.base_bytes().len(), 4);
            assert_eq!(value.stride_bytes().len(), 3);
            assert_eq!(
                value.accesses().map(interleave::scenario::AccessCount::get),
                Some(2048)
            );
        }
        ScenarioCaseKind::Stride(_) | ScenarioCaseKind::MultiStream(_) => {
            return Err("second case did not retain sweep kind".to_owned());
        }
    }
    match scenario_case(&model, 2)?.kind() {
        ScenarioCaseKind::MultiStream(value) => {
            assert_eq!(value.schedule(), Schedule::RoundRobin);
            assert_eq!(
                value
                    .streams()
                    .iter()
                    .map(|stream| (stream.name().as_str(), stream.accesses().get()))
                    .collect::<Vec<_>>(),
                [("cpu", 3), ("dma", 2)]
            );
        }
        ScenarioCaseKind::Stride(_) | ScenarioCaseKind::Sweep(_) => {
            return Err("third case did not retain multi_stream kind".to_owned());
        }
    }
    Ok(())
}

#[test]
fn accepts_address_underscores_and_structural_boundaries() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [1] }
cases:
  - { name: zero-stride, kind: stride, base_bytes: 0xAB_CD, stride_bytes: 0 }
  - name: one-stream
    enabled: false
    kind: multi_stream
    schedule: round_robin
    streams:
      - { name: only, base_bytes: 1_000, stride_bytes: 0x1_0, accesses: 1 }
";

    // When
    let model = scenario_decode(source)?;

    // Then
    assert_eq!(model.cases().len(), 2);
    assert!(!scenario_case(&model, 1)?.enabled());
    match scenario_case(&model, 1)?.kind() {
        ScenarioCaseKind::MultiStream(value) => {
            assert_eq!(value.streams().len(), 1);
            assert_eq!(
                scenario_stream(value.streams(), 0)?
                    .base_bytes()
                    .canonical(),
                "0x3e8"
            );
        }
        ScenarioCaseKind::Stride(_) | ScenarioCaseKind::Sweep(_) => {
            return Err("one-stream case changed kind".to_owned());
        }
    }
    Ok(())
}

#[test]
fn retains_selection_dependent_values_for_later_validation() -> ScenarioTestResult {
    // Given: W > inherited Q is deliberately deferred to Scenario selection/expansion.
    let source = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [2] }
cases:
  - { name: deferred, enabled: false, kind: stride, base_bytes: 0, stride_bytes: 0 }
";

    // When
    let model = scenario_decode(source)?;

    // Then
    assert_eq!(model.cases().len(), 1);
    assert_eq!(scenario_window(model.defaults().window_sizes(), 0)?.get(), 2);
    Ok(())
}

#[test]
fn scenario_errors_use_exit_class_three() -> ScenarioTestResult {
    // Given
    let source = "schema_version: 1\ndefaults: {}\ncases: []\n";

    // When
    let error = scenario_errors(source)?;

    // Then
    assert_eq!(error.exit_class(), ExitClass::ScenarioOrAddress);
    assert!(error
        .issues()
        .iter()
        .all(|issue| matches!(issue.code(), IssueCode::ScenarioInvalid)));
    Ok(())
}
