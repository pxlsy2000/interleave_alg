use std::{fs, os::unix::fs::MetadataExt as _};

use interleave::{
    io::{
        atomic_output::{OutputDestination, OutputRequest, write_report},
        input::read_named,
    },
    issue::{Issue, IssueCode},
};
use tempfile::tempdir;

#[test]
fn same_input_and_output_requires_force() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("mapping.yaml");
    fs::write(&output, b"input snapshot")?;
    let snapshot = read_named(&output)?;
    let identities = snapshot.identity().into_iter().collect::<Vec<_>>();

    // When
    let error = write_report(
        &OutputRequest::new(OutputDestination::Path(&output), false, &identities),
        b"report\n",
        &mut Vec::new(),
    )
    .err()
    .ok_or("same file did not require force")?;

    // Then
    assert_eq!(snapshot.bytes(), b"input snapshot");
    assert_eq!(
        error.issue().map(Issue::code),
        Some(IssueCode::OutputExists)
    );
    assert!(error.aliases_input());
    assert_eq!(fs::read(&output)?, b"input snapshot");
    Ok(())
}

#[test]
fn same_input_and_output_is_replaced_only_after_complete_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("mapping.yaml");
    fs::write(&output, b"input snapshot")?;
    let snapshot = read_named(&output)?;
    let identities = snapshot.identity().into_iter().collect::<Vec<_>>();

    // When
    write_report(
        &OutputRequest::new(OutputDestination::Path(&output), true, &identities),
        b"report\n",
        &mut Vec::new(),
    )?;

    // Then
    assert_eq!(snapshot.bytes(), b"input snapshot");
    assert_eq!(fs::read(&output)?, b"report\n");
    Ok(())
}

#[test]
fn hardlink_alias_is_detected_by_device_and_inode() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let input = directory.path().join("mapping.yaml");
    let output = directory.path().join("alias.txt");
    fs::write(&input, b"input")?;
    fs::hard_link(&input, &output)?;
    let snapshot = read_named(&input)?;
    let identities = snapshot.identity().into_iter().collect::<Vec<_>>();

    // When
    let error = write_report(
        &OutputRequest::new(OutputDestination::Path(&output), false, &identities),
        b"report\n",
        &mut Vec::new(),
    )
    .err()
    .ok_or("hardlink alias did not require force")?;

    // Then
    assert_eq!(
        error.issue().map(Issue::code),
        Some(IssueCode::OutputExists)
    );
    assert!(error.aliases_input());
    assert_eq!(fs::read(&input)?, b"input");
    assert_eq!(fs::read(&output)?, b"input");
    Ok(())
}

#[test]
fn hardlink_alias_is_detached_from_input_with_force() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let input = directory.path().join("mapping.yaml");
    let output = directory.path().join("alias.txt");
    fs::write(&input, b"input")?;
    fs::hard_link(&input, &output)?;
    let snapshot = read_named(&input)?;
    let identities = snapshot.identity().into_iter().collect::<Vec<_>>();

    // When
    write_report(
        &OutputRequest::new(OutputDestination::Path(&output), true, &identities),
        b"report\n",
        &mut Vec::new(),
    )?;

    // Then
    assert_eq!(snapshot.bytes(), b"input");
    assert_eq!(fs::read(&input)?, b"input");
    assert_eq!(fs::read(&output)?, b"report\n");
    assert_ne!(fs::metadata(&input)?.ino(), fs::metadata(&output)?.ino());
    Ok(())
}
