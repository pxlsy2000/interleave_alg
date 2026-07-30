use std::{
    fs,
    io::{self, Write},
    os::unix::fs::MetadataExt as _,
    sync::{Arc, Barrier},
    thread,
};

use interleave::{
    io::{
        atomic_output::{
            AtomicOutputFault, OutputDestination, OutputRequest, format_stderr_issue, write_report,
        },
        input::read_named,
    },
    issue::{Issue, IssueCode},
};
use tempfile::tempdir;

fn temp_entries(directory: &std::path::Path) -> io::Result<Vec<String>> {
    fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .filter(|entry| {
            entry
                .as_ref()
                .map_or(true, |name| name.starts_with(".interleave.tmp."))
        })
        .collect()
}

#[test]
fn complete_report_is_committed_when_path_is_new() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("report.txt");
    let request = OutputRequest::new(OutputDestination::Path(&output), false, &[]);
    let mut stdout = Vec::new();

    // When
    write_report(&request, b"complete\n", &mut stdout)?;

    // Then
    assert_eq!(fs::read(&output)?, b"complete\n");
    assert!(stdout.is_empty());
    assert!(temp_entries(directory.path())?.is_empty());
    Ok(())
}

#[test]
fn existing_path_is_untouched_without_force() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("report.txt");
    fs::write(&output, b"prior\n")?;
    let request = OutputRequest::new(OutputDestination::Path(&output), false, &[]);

    // When
    let error = write_report(&request, b"replacement\n", &mut Vec::new())
        .err()
        .ok_or("existing output was not refused")?;

    // Then
    assert_eq!(
        error.issue().map(Issue::code),
        Some(IssueCode::OutputExists)
    );
    assert_eq!(error.exit_class().code(), 1);
    assert_eq!(fs::read(&output)?, b"prior\n");
    assert!(temp_entries(directory.path())?.is_empty());
    Ok(())
}

#[test]
fn existing_path_is_atomically_replaced_with_force() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("report.txt");
    fs::write(&output, b"prior\n")?;
    let original_inode = fs::metadata(&output)?.ino();
    let request = OutputRequest::new(OutputDestination::Path(&output), true, &[]);

    // When
    write_report(&request, b"replacement\n", &mut Vec::new())?;

    // Then
    assert_eq!(fs::read(&output)?, b"replacement\n");
    assert_ne!(fs::metadata(&output)?.ino(), original_inode);
    assert!(temp_entries(directory.path())?.is_empty());
    Ok(())
}

#[derive(Debug, Default)]
struct CountingWriter {
    bytes: Vec<u8>,
    writes: usize,
}

#[derive(Debug, Default)]
struct RejectingWriter;

impl Write for RejectingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "stdout rejected report",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for CountingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn stdout_receives_complete_report_in_one_write() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let request = OutputRequest::new(OutputDestination::Stdout, false, &[]);
    let mut stdout = CountingWriter::default();

    // When
    write_report(&request, b"complete\n", &mut stdout)?;

    // Then
    assert_eq!(stdout.bytes, b"complete\n");
    assert_eq!(stdout.writes, 1);
    Ok(())
}

#[test]
fn stdout_write_failure_is_output_io() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let request = OutputRequest::new(OutputDestination::Stdout, false, &[]);
    let mut stdout = RejectingWriter;

    // When
    let error = write_report(&request, b"complete\n", &mut stdout)
        .err()
        .ok_or("stdout failure was accepted")?;

    // Then
    assert_eq!(error.issue().map(Issue::code), Some(IssueCode::OutputIo));
    assert_eq!(error.exit_class().code(), 1);
    Ok(())
}

#[test]
fn force_is_usage_error_for_stdout() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let request = OutputRequest::new(OutputDestination::Stdout, true, &[]);
    let mut stdout = Vec::new();

    // When
    let error = write_report(&request, b"report\n", &mut stdout)
        .err()
        .ok_or("force was accepted for stdout")?;

    // Then
    assert!(error.issue().is_none());
    assert!(stdout.is_empty());
    Ok(())
}

#[test]
fn common_stderr_formatter_is_stable() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("report.txt");
    fs::write(&output, b"prior")?;
    let request = OutputRequest::new(OutputDestination::Path(&output), false, &[]);
    let error = write_report(&request, b"report\n", &mut Vec::new())
        .err()
        .ok_or("existing path did not fail")?;
    let issue = error.issue().ok_or("missing output issue")?;

    // When
    let diagnostic = format_stderr_issue(issue);

    // Then
    assert_eq!(
        diagnostic,
        "ERROR [output.exists]: output path already exists; use --force to replace it\n"
    );
    Ok(())
}

#[test]
fn input_io_uses_common_stderr_formatter() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let missing = std::path::Path::new("/definitely/missing/interleave-input");
    let error = read_named(missing)
        .err()
        .ok_or("missing input did not fail")?;

    // When
    let diagnostic = format_stderr_issue(error.issue());

    // Then
    assert_eq!(diagnostic, "ERROR [input.io]: input could not be read\n");
    Ok(())
}

#[test]
fn concurrent_no_force_writers_commit_exactly_one_complete_report()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = Arc::new(directory.path().join("report.txt"));
    let barrier = Arc::new(Barrier::new(2));
    let writers = [b"first\n".as_slice(), b"second\n".as_slice()].map(|report| {
        let output = Arc::clone(&output);
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            write_report(
                &OutputRequest::new(OutputDestination::Path(&output), false, &[])
                    .with_fault(AtomicOutputFault::None),
                report,
                &mut Vec::new(),
            )
        })
    });

    // When
    let outcomes = writers
        .into_iter()
        .map(thread::JoinHandle::join)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "writer thread panicked")?;

    // Then
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter_map(|outcome| outcome.as_ref().err())
            .filter_map(|error| error.issue())
            .filter(|issue| issue.code() == IssueCode::OutputExists)
            .count(),
        1
    );
    let bytes = fs::read(&*output)?;
    assert!(bytes == b"first\n" || bytes == b"second\n");
    assert!(temp_entries(directory.path())?.is_empty());
    Ok(())
}
