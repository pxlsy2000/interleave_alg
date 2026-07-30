use interleave::{
    input::{load_yaml_bytes, scalar::Address},
    mapping::{AddressMapper, MappingModel, MappingValidation, decode_mapping, validate_mapping},
    metrics::analyze_targets,
    report::{Report, RunCaseResult},
    scenario::{decode_scenario, generate_target_sequence, preflight_scenarios, select_cases},
};

pub(super) type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
pub(super) enum MappingFixture {
    Natural,
    NonNatural,
    TargetUnreachable,
    NonBijective,
}

pub(super) fn mapping(kind: MappingFixture) -> FixtureResult<MappingModel> {
    let local = match kind {
        MappingFixture::Natural | MappingFixture::TargetUnreachable => {
            "  l: { mode: preserve_high }\n"
        }
        MappingFixture::NonNatural => {
            "  l:\n    mode: explicit\n    rows: [[3], [2], [4], [5], [6], [7]]\n"
        }
        MappingFixture::NonBijective => {
            "  l:\n    mode: explicit\n    rows: [[2], [3], [4], [5], [6], [6]]\n"
        }
    };
    let target_rows = match kind {
        MappingFixture::TargetUnreachable => "[[0], [0]]",
        MappingFixture::Natural | MappingFixture::NonNatural | MappingFixture::NonBijective => {
            "[[0], [1]]"
        }
    };
    let source = format!(
        "schema_version: 1\nname: report-map\naddress:\n  width_bits: 8\n  granule_bytes: 1\n\
         targets:\n  count: 4\nmapping:\n  m:\n    rows: {target_rows}\n{local}"
    );
    let document = load_yaml_bytes(source.as_bytes())?;
    Ok(decode_mapping(&document)?)
}

pub(super) fn validated(kind: MappingFixture) -> FixtureResult<(MappingModel, MappingValidation)> {
    let mapping = mapping(kind)?;
    let validation = validate_mapping(&mapping);
    Ok((mapping, validation))
}

pub(super) fn validation_report(kind: MappingFixture) -> FixtureResult<Report> {
    let (mapping, validation) = validated(kind)?;
    Ok(Report::validate(&mapping, "mapping.yaml", &validation))
}

pub(super) fn map_report(kind: MappingFixture, inputs: &[&str]) -> FixtureResult<Report> {
    let (mapping, validation) = validated(kind)?;
    let mapper = AddressMapper::try_new(&mapping)?;
    let addresses = inputs
        .iter()
        .map(|input| Address::parse(input))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = mapper.map_addresses(&addresses)?;
    Ok(Report::map(&mapping, &validation, &rows)?)
}

pub(super) fn run_report() -> FixtureResult<Report> {
    let (mapping, validation) = validated(MappingFixture::Natural)?;
    let source = r"
schema_version: 1
defaults:
  accesses: 4
  window_sizes: [2]
cases:
  - name: sequential
    kind: stride
    base_bytes: 0
    stride_bytes: 1
  - name: two-stream
    kind: multi_stream
    window_sizes: [2, 4]
    schedule: round_robin
    streams:
      - name: first
        base_bytes: 0
        stride_bytes: 4
        accesses: 2
      - name: second
        base_bytes: 1
        stride_bytes: 4
        accesses: 2
";
    let document = load_yaml_bytes(source.as_bytes())?;
    let scenario = decode_scenario(&document)?;
    let selected = select_cases(&scenario, &[])?;
    let plan = preflight_scenarios(&mapping, &selected)?;
    let mapper = AddressMapper::try_new(&mapping)?;
    let cases = plan
        .tests()
        .iter()
        .map(|descriptor| {
            let targets = generate_target_sequence(&mapper, descriptor)?;
            let metrics =
                analyze_targets(&targets, mapping.target_count(), descriptor.window_sizes())?;
            Ok(RunCaseResult::from_metrics(descriptor, &metrics)?)
        })
        .collect::<FixtureResult<Vec<_>>>()?;
    Ok(Report::run(&mapping, &validation, cases)?)
}
