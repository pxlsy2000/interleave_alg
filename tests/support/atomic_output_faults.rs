use std::{fs, path::Path};

use interleave::{
    io::atomic_output::{AtomicOutputFault, OutputDestination, OutputRequest, write_report},
    issue::{Issue, IssueCode},
};
use tempfile::tempdir;

fn temp_count(directory: &Path) -> Result<usize, std::io::Error> {
    Ok(fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".interleave.tmp.")
        })
        .count())
}

fn assert_fault_preserves_target(
    fault: AtomicOutputFault,
    expected: IssueCode,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("report.txt");
    fs::write(&output, b"prior\n")?;
    let request = OutputRequest::new(OutputDestination::Path(&output), true, &[]).with_fault(fault);

    // When
    let error = write_report(&request, b"replacement\n", &mut Vec::new())
        .err()
        .ok_or("injected failure did not abort transaction")?;

    // Then
    assert_eq!(error.issue().map(Issue::code), Some(expected));
    assert_eq!(fs::read(&output)?, b"prior\n");
    assert_eq!(temp_count(directory.path())?, 0);
    Ok(())
}

#[test]
fn injected_write_failure_preserves_target_and_cleans_temp()
-> Result<(), Box<dyn std::error::Error>> {
    assert_fault_preserves_target(AtomicOutputFault::Write, IssueCode::OutputIo)
}

#[test]
fn injected_close_failure_preserves_target_and_cleans_temp()
-> Result<(), Box<dyn std::error::Error>> {
    assert_fault_preserves_target(AtomicOutputFault::Close, IssueCode::OutputIo)
}

#[test]
fn injected_precommit_failure_preserves_target_and_cleans_temp()
-> Result<(), Box<dyn std::error::Error>> {
    assert_fault_preserves_target(AtomicOutputFault::Precommit, IssueCode::OutputIo)
}

#[test]
fn rename_nosys_is_atomic_unsupported_without_fallback() -> Result<(), Box<dyn std::error::Error>> {
    assert_unsupported_rename(AtomicOutputFault::RenameNoReplaceNoSys)
}

#[test]
fn rename_eopnotsupp_is_atomic_unsupported_without_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    assert_unsupported_rename(AtomicOutputFault::RenameNoReplaceNotSupported)
}

fn assert_unsupported_rename(fault: AtomicOutputFault) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("report.txt");
    let request =
        OutputRequest::new(OutputDestination::Path(&output), false, &[]).with_fault(fault);

    // When
    let error = write_report(&request, b"report\n", &mut Vec::new())
        .err()
        .ok_or("unsupported rename did not abort")?;

    // Then
    assert_eq!(
        error.issue().map(Issue::code),
        Some(IssueCode::OutputAtomicUnsupported)
    );
    assert!(!output.exists());
    assert_eq!(temp_count(directory.path())?, 0);
    Ok(())
}

#[test]
fn exhausted_temp_names_are_output_io_without_residue() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("report.txt");
    let request = OutputRequest::new(OutputDestination::Path(&output), false, &[])
        .with_fault(AtomicOutputFault::TempExhaustion);

    // When
    let error = write_report(&request, b"report\n", &mut Vec::new())
        .err()
        .ok_or("128 collisions did not exhaust transaction")?;

    // Then
    assert_eq!(error.issue().map(Issue::code), Some(IssueCode::OutputIo));
    assert!(!output.exists());
    assert_eq!(temp_count(directory.path())?, 0);
    Ok(())
}
