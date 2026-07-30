use std::io;

use interleave::{
    input::{load_yaml_bytes, scalar::Address},
    mapping::{AddressMapper, MappingModel, decode_mapping, validate_mapping},
    metrics::analyze_targets,
    report::{Report, RunCaseResult},
    scenario::{decode_scenario, preflight_scenarios, select_cases},
};

type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
pub(super) enum ValidationFixture {
    Natural,
    NonNatural,
    TargetUnreachable,
    NonBijective,
}

pub(super) fn validation_report(kind: ValidationFixture) -> FixtureResult<Report> {
    let mapping = mapping(kind)?;
    let validation = validate_mapping(&mapping);
    Ok(Report::validate(&mapping, "mapping.yaml", &validation))
}

pub(super) fn validation_report_with_input(input: &str) -> FixtureResult<Report> {
    let mapping = mapping(ValidationFixture::Natural)?;
    let validation = validate_mapping(&mapping);
    Ok(Report::validate(&mapping, input, &validation))
}

pub(super) fn zero_dimension_validation_report() -> FixtureResult<Report> {
    let source = concat!(
        "schema_version: 1\n",
        "name: zero-dimension\n",
        "address:\n",
        "  width_bits: 1\n",
        "  granule_bytes: 2\n",
        "targets:\n",
        "  count: 1\n",
        "mapping:\n",
        "  m:\n",
        "    rows: []\n",
        "  l:\n",
        "    mode: preserve_high\n",
    );
    let mapping = decode(source)?;
    let validation = validate_mapping(&mapping);
    Ok(Report::validate(&mapping, "zero.yaml", &validation))
}

pub(super) fn zero_target_rows_validation_report() -> FixtureResult<Report> {
    let source = concat!(
        "schema_version: 1\n",
        "name: zero-target-rows\n",
        "address:\n",
        "  width_bits: 4\n",
        "  granule_bytes: 1\n",
        "targets:\n",
        "  count: 1\n",
        "mapping:\n",
        "  m:\n",
        "    rows: []\n",
        "  l:\n",
        "    mode: preserve_high\n",
    );
    let mapping = decode(source)?;
    let validation = validate_mapping(&mapping);
    Ok(Report::validate(&mapping, "zero-rows.yaml", &validation))
}

pub(super) fn map_report(kind: ValidationFixture) -> FixtureResult<Report> {
    let mapping = mapping(kind)?;
    let validation = validate_mapping(&mapping);
    let mapper = AddressMapper::try_new(&mapping)?;
    let addresses = ["0x0", "0x40", "0x80", "0xc0", "0x1234"]
        .iter()
        .map(|lexeme| Address::parse(lexeme))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = mapper.map_addresses(&addresses)?;
    Ok(Report::map_with_input(
        &mapping,
        Some("mapping.yaml"),
        &validation,
        &rows,
    )?)
}

pub(super) fn run_report() -> FixtureResult<Report> {
    run_report_with_mapping(ValidationFixture::Natural)
}

pub(super) fn run_report_with_mapping(kind: ValidationFixture) -> FixtureResult<Report> {
    let mapping = mapping(kind)?;
    let validation = validate_mapping(&mapping);
    let scenario_source = concat!(
        "schema_version: 1\n",
        "defaults:\n",
        "  accesses: 1\n",
        "  window_sizes: [1]\n",
        "cases:\n",
        "  - name: sequential\n",
        "    kind: stride\n",
        "    base_bytes: 0\n",
        "    stride_bytes: 64\n",
        "    accesses: 4096\n",
        "    window_sizes: [4, 16, 64]\n",
        "  - name: tie-and-zero\n",
        "    kind: stride\n",
        "    base_bytes: 0\n",
        "    stride_bytes: 64\n",
        "    accesses: 4\n",
        "    window_sizes: [1, 2, 4]\n",
    );
    let document = load_yaml_bytes(scenario_source.as_bytes())?;
    let scenario = decode_scenario(&document)?;
    let selected = select_cases(&scenario, &[])?;
    let plan = preflight_scenarios(&mapping, &selected)?;
    let documented = documented_targets()?;
    let tie = vec![0, 1, 0, 1];
    let target_sequences = [documented.as_slice(), tie.as_slice()];
    let cases = plan
        .tests()
        .iter()
        .zip(target_sequences)
        .map(|(descriptor, targets)| {
            let metrics =
                analyze_targets(targets, mapping.target_count(), descriptor.window_sizes())?;
            Ok(RunCaseResult::from_metrics(descriptor, &metrics)?)
        })
        .collect::<FixtureResult<Vec<_>>>()?;
    Ok(Report::run_with_input(
        &mapping,
        Some("mapping.yaml"),
        &validation,
        cases,
    )?)
}

fn mapping(kind: ValidationFixture) -> FixtureResult<MappingModel> {
    let (name, target_rows, local) = match kind {
        ValidationFixture::Natural => (
            "example-4-target",
            "[[0, 4, 8], [1, 5, 9]]",
            "    mode: preserve_high\n".to_owned(),
        ),
        ValidationFixture::NonNatural => (
            "example-4-target",
            "[[0, 4, 8], [1, 5, 9]]",
            concat!(
                "    mode: explicit\n",
                "    rows: [[3], [2], [4], [5], [6], [7], [8], [9], [10], [11], [12], [13]]\n"
            )
            .to_owned(),
        ),
        ValidationFixture::TargetUnreachable => (
            "unreachable-4-target",
            "[[0], [0]]",
            "    mode: preserve_high\n".to_owned(),
        ),
        ValidationFixture::NonBijective => (
            "broken-4-target",
            "[[0, 4, 8], [1, 5, 9]]",
            concat!(
                "    mode: explicit\n",
                "    rows: [[2], [3], [4], [5], [6], [7], [8], [9], [10], [11], [12], [12]]\n"
            )
            .to_owned(),
        ),
    };
    let source = format!(
        "schema_version: 1\nname: {name}\naddress:\n  width_bits: 20\n  granule_bytes: 64\n\
         targets:\n  count: 4\nmapping:\n  m:\n    rows: {target_rows}\n  l:\n{local}"
    );
    decode(&source)
}

fn decode(source: &str) -> FixtureResult<MappingModel> {
    let document = load_yaml_bytes(source.as_bytes())?;
    Ok(decode_mapping(&document)?)
}

fn documented_targets() -> FixtureResult<Vec<u16>> {
    let mut block = (0..64)
        .map(|index| u16::try_from(index % 4))
        .collect::<Result<Vec<_>, _>>()?;
    replace(&mut block, 16, 1)?;
    replace(&mut block, 17, 0)?;
    replace(&mut block, 30, 0)?;
    replace(&mut block, 31, 2)?;
    replace(&mut block, 32, 2)?;
    replace(&mut block, 34, 3)?;
    let mut targets: Vec<_> = block.iter().copied().cycle().take(4096).collect();
    replace(&mut targets, 256, 1)?;
    replace(&mut targets, 257, 0)?;
    Ok(targets)
}

fn replace(values: &mut [u16], index: usize, value: u16) -> FixtureResult<()> {
    let slot = values
        .get_mut(index)
        .ok_or_else(|| io::Error::other("fixture index is outside the sequence"))?;
    *slot = value;
    Ok(())
}
