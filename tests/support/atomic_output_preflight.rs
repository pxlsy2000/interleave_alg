use std::fs;

use interleave::io::atomic_output::{OutputDestination, OutputRequest, preflight_report_output};
use tempfile::TempDir;

#[test]
fn read_only_preflight_refuses_an_existing_destination_without_creating_a_temp()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = TempDir::new()?;
    let destination = directory.path().join("report.txt");
    fs::write(&destination, b"original\n")?;
    let request = OutputRequest::new(OutputDestination::Path(&destination), false, &[]);

    // When
    let Err(error) = preflight_report_output(&request) else {
        return Err(std::io::Error::other("existing output must be refused").into());
    };

    // Then
    assert_eq!(
        error.issue().map(|issue| issue.code().as_str()),
        Some("output.exists")
    );
    assert_eq!(fs::read(&destination)?, b"original\n");
    let entries: Vec<_> = fs::read_dir(directory.path())?.collect();
    assert_eq!(entries.len(), 1);
    Ok(())
}
