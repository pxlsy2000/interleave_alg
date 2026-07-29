//! Centralized inclusive v1 resource and interoperability limits.

/// Maximum supported address width.
pub const MAX_ADDRESS_WIDTH_BITS: u8 = 64;
/// Maximum retained bytes in one Mapping or Scenario source.
pub const MAX_RAW_INPUT_BYTES: usize = 16 * 1024 * 1024;
/// Maximum address operands in one `map` command.
pub const MAX_QUERY_ADDRESSES: usize = 1_000_000;
/// Maximum v1 Target count.
pub const MAX_TARGET_COUNT: u128 = 65_536;
/// Maximum v1 granule size.
pub const MAX_GRANULE_BYTES: u128 = 1_u128 << 52;
/// Maximum accesses in one concrete test.
pub const MAX_ACCESSES_PER_TEST: u128 = 10_000_000;
/// Maximum concrete tests expanded by one run.
pub const MAX_CONCRETE_TESTS: usize = 10_000;
/// Maximum generated accesses summed across one run.
pub const MAX_TOTAL_GENERATED_ACCESSES: u128 = 100_000_000;
/// Maximum streams in one multi-stream case.
pub const MAX_STREAMS_PER_CASE: usize = 4_096;
/// Maximum effective window sizes in one case.
pub const MAX_WINDOW_SIZES_PER_CASE: usize = 1_024;
/// Maximum decoded UTF-8 bytes in a Mapping, case, or stream name.
pub const MAX_IDENTIFIER_UTF8_BYTES: usize = 128;
/// Maximum Target rows in one report.
pub const MAX_REPORT_TARGET_ROWS: u128 = 1_000_000;
/// Maximum window rows in one report.
pub const MAX_REPORT_WINDOW_ROWS: u128 = 1_000_000;
/// Maximum exact `sum(Q*K)` work in one run.
pub const MAX_WINDOW_WORK: u128 = 100_000_000;
/// Maximum complete report bytes, including the trailing LF.
pub const MAX_REPORT_BYTES: usize = 268_435_456;
/// Largest integer that remains exact in interoperable JSON numbers.
pub const MAX_JSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
/// Largest v1 maximum-load or window-ratio numerator.
pub const MAX_RATIO_NUMERATOR: u64 = 655_360_000_000;

#[cfg(test)]
#[path = "limits_tests.rs"]
mod tests;
