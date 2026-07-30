use std::{ffi::OsStr, path::Path};

use crate::input::{InputSnapshot, read_named, read_stdin};

use super::error::ExecutionError;

pub(super) fn acquire(path: &Path) -> Result<(InputSnapshot, String), ExecutionError> {
    if path.as_os_str() == OsStr::new("-") {
        return Ok((read_stdin()?, "-".to_owned()));
    }
    let snapshot = read_named(path)?;
    Ok((snapshot, path.to_string_lossy().into_owned()))
}
