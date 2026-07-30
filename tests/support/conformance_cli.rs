use std::process::{Command, Output};

use serde_json::Value;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn interleave(arguments: &[&str]) -> std::io::Result<Output> {
    Command::new(assert_cmd::cargo::cargo_bin!("interleave"))
        .args(arguments)
        .output()
}

pub fn json(output: &Output) -> TestResult<Value> {
    Ok(serde_json::from_slice(&output.stdout)?)
}

pub fn at<'value>(value: &'value Value, pointer: &str) -> TestResult<&'value Value> {
    value
        .pointer(pointer)
        .ok_or_else(|| format!("JSON pointer {pointer} must exist").into())
}
