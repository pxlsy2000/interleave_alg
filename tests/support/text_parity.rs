use interleave::report::{TextReportStyle, render_json, render_text};
use serde_json::Value;

use super::{TestResult, fixture};

#[test]
fn text_and_json_project_the_same_run_semantics() -> TestResult {
    // Given
    let report = fixture::run_report_with_mapping(fixture::ValidationFixture::NonNatural)?;

    // When
    let text = String::from_utf8(render_text(&report, TextReportStyle::Standard)?)?;
    let json: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    let classification = string_at(&json, "/result/mapping_classification")?;
    assert!(text.contains(&format!("Result: {classification}\n")));
    let cases = array_at(&json, "/result/cases")?;
    let warnings = array_at(&json, "/warnings")?;
    assert_eq!(text.matches("Case: ").count(), cases.len());
    assert_eq!(
        text.matches("WARNING [").count(),
        warnings.len() * cases.len()
    );
    let mut cursor = TextCursor::new(&text);
    for case in cases {
        assert_case_semantics(&mut cursor, case, warnings)?;
    }
    Ok(())
}

struct TextCursor<'text> {
    text: &'text str,
    position: usize,
}

impl<'text> TextCursor<'text> {
    const fn new(text: &'text str) -> Self {
        Self { text, position: 0 }
    }

    fn require(&mut self, needle: &str) -> TestResult {
        let remainder = self
            .text
            .get(self.position..)
            .ok_or("semantic parity cursor is outside text")?;
        let offset = remainder
            .find(needle)
            .ok_or_else(|| format!("text report is missing ordered value {needle:?}"))?;
        self.position += offset + needle.len();
        Ok(())
    }
}

fn assert_case_semantics(
    cursor: &mut TextCursor<'_>,
    case: &Value,
    warnings: &[Value],
) -> TestResult {
    cursor.require(&format!("Case: {}\n", string_at(case, "/case_id")?))?;
    cursor.require(&format!("Accesses: {}\n", integer_at(case, "/accesses")?))?;
    assert_targets(cursor, case)?;
    assert_maximum(cursor, case)?;
    assert_windows(cursor, case)?;
    assert_longest_run(cursor, case)?;
    assert_warnings(cursor, warnings)
}

fn assert_targets(cursor: &mut TextCursor<'_>, case: &Value) -> TestResult {
    for row in array_at(case, "/targets")? {
        let target = integer_at(row, "/target")?;
        let count = integer_at(row, "/count")?;
        let decimal = string_at(row, "/share/decimal")?;
        cursor.require(&format!("{target:<8}{count:<7}{decimal}\n"))?;
    }
    Ok(())
}

fn assert_maximum(cursor: &mut TextCursor<'_>, case: &Value) -> TestResult {
    let target = integer_at(case, "/max_load/target")?;
    let count = integer_at(case, "/max_load/count")?;
    let decimal = string_at(case, "/max_load/ratio/decimal")?;
    cursor.require(&format!("{target:<8}{count:<7}{decimal}\n"))
}

fn assert_windows(cursor: &mut TextCursor<'_>, case: &Value) -> TestResult {
    for row in array_at(case, "/windows")? {
        let size = integer_at(row, "/size")?;
        let target = integer_at(row, "/target")?;
        let start = integer_at(row, "/start_index")?;
        let count = integer_at(row, "/count")?;
        let decimal = string_at(row, "/ratio/decimal")?;
        cursor.require(&format!(
            "{size:<6}{target:<8}{start:<13}{count:<7}{decimal}\n"
        ))?;
    }
    Ok(())
}

fn assert_longest_run(cursor: &mut TextCursor<'_>, case: &Value) -> TestResult {
    let length = integer_at(case, "/longest_run/length")?;
    let target = integer_at(case, "/longest_run/target")?;
    let start = integer_at(case, "/longest_run/start_index")?;
    cursor.require(&format!("{length:<8}{target:<8}{start}\n"))
}

fn assert_warnings(cursor: &mut TextCursor<'_>, warnings: &[Value]) -> TestResult {
    for issue in warnings {
        let code = string_at(issue, "/code")?;
        let path = string_at(issue, "/path")?;
        let message = string_at(issue, "/message")?;
        cursor.require(&format!("WARNING [{code}] {path}: {message}\n"))?;
    }
    Ok(())
}

fn array_at<'value>(value: &'value Value, pointer: &str) -> TestResult<&'value [Value]> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("missing array at {pointer}").into())
}

fn string_at<'value>(value: &'value Value, pointer: &str) -> TestResult<&'value str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string at {pointer}").into())
}

fn integer_at(value: &Value, pointer: &str) -> TestResult<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing integer at {pointer}").into())
}
