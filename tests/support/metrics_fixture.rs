use interleave::{
    input::load_yaml_bytes,
    mapping::{MappingModel, decode_mapping},
    metrics::{TestMetrics, analyze_targets},
    scenario::{
        ScenarioModel, WindowSize, decode_scenario, preflight_scenarios, select_cases,
    },
};

type MetricsTestResult<T = ()> = Result<T, String>;

fn metrics_mapping(targets: u32) -> MetricsTestResult<MappingModel> {
    let target_bits = targets.ilog2();
    let rows = (0..target_bits)
        .map(|bit| format!("[{bit}]"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!(
        "schema_version: 1\nname: metrics\naddress: {{ width_bits: 16, granule_bytes: 1 }}\n\
         targets: {{ count: {targets} }}\nmapping:\n  m: {{ rows: [{rows}] }}\n\
         \x20 l: {{ mode: preserve_high }}\n"
    );
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    decode_mapping(&document).map_err(|error| error.to_string())
}

fn metric_windows(
    mapping: &MappingModel,
    accesses: u64,
    windows: &str,
) -> MetricsTestResult<Vec<WindowSize>> {
    let source = format!(
        "schema_version: 1\ndefaults: {{ accesses: {accesses}, window_sizes: [{windows}] }}\n\
         cases:\n  - {{ name: metrics, kind: stride, base_bytes: 0, stride_bytes: 1 }}\n"
    );
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    let scenario: ScenarioModel =
        decode_scenario(&document).map_err(|error| error.to_string())?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;
    let plan = preflight_scenarios(mapping, &selected).map_err(|error| error.to_string())?;
    plan.tests()
        .first()
        .map(|test| test.window_sizes().to_vec())
        .ok_or_else(|| "missing metrics descriptor".to_owned())
}

fn analyze_fixture(
    targets: &[u16],
    target_count: u32,
    window_values: &[u64],
) -> MetricsTestResult<TestMetrics> {
    let mapping = metrics_mapping(target_count)?;
    let declared = window_values
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let windows = metric_windows(
        &mapping,
        u64::try_from(targets.len()).map_err(|error| error.to_string())?,
        &declared,
    )?;
    analyze_targets(targets, mapping.target_count(), &windows).map_err(|error| error.to_string())
}

const fn ratio_tuple(ratio: interleave::input::scalar::Ratio) -> (u128, u128) {
    (ratio.numerator(), ratio.denominator())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OracleWindow {
    size: u64,
    target: u16,
    start: u64,
    count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OracleRun {
    length: u64,
    target: u16,
    start: u64,
}

fn oracle_counts(targets: &[u16], target_count: u32) -> Vec<u64> {
    (0..target_count)
        .map(|target| {
            targets
                .iter()
                .filter(|observed| u32::from(**observed) == target)
                .count()
                .try_into()
                .unwrap_or(u64::MAX)
        })
        .collect()
}

fn oracle_window(targets: &[u16], target_count: u32, size: u64) -> OracleWindow {
    let size_usize = usize::try_from(size).unwrap_or(usize::MAX);
    let mut candidates = targets
        .windows(size_usize)
        .enumerate()
        .flat_map(|(start, window)| {
            (0..target_count).map(move |target| {
                let count = window
                    .iter()
                    .filter(|observed| u32::from(**observed) == target)
                    .count();
                (count, start, target)
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    let (count, start, target) = candidates.first().copied().unwrap_or_default();
    OracleWindow {
        size,
        target: u16::try_from(target).unwrap_or(u16::MAX),
        start: u64::try_from(start).unwrap_or(u64::MAX),
        count: u64::try_from(count).unwrap_or(u64::MAX),
    }
}

fn oracle_run(targets: &[u16]) -> OracleRun {
    let mut best = OracleRun {
        length: 0,
        target: 0,
        start: 0,
    };
    for start in 0..targets.len() {
        let Some(target) = targets.get(start).copied() else {
            continue;
        };
        let length = targets
            .iter()
            .skip(start)
            .take_while(|observed| **observed == target)
            .count();
        if length > usize::try_from(best.length).unwrap_or(usize::MAX) {
            best = OracleRun {
                length: u64::try_from(length).unwrap_or(u64::MAX),
                target,
                start: u64::try_from(start).unwrap_or(u64::MAX),
            };
        }
    }
    best
}
