use std::{
    fs,
    os::unix::fs::{PermissionsExt as _, symlink},
    path::Path,
};

use interleave::{
    io::atomic_output::{OutputDestination, OutputRequest, write_report},
    issue::{Issue, IssueCode},
};
use rustix::fs::{CWD, Mode, mkfifoat};
use tempfile::tempdir;

#[derive(Debug)]
struct UmaskGuard(Mode);

impl UmaskGuard {
    fn set(mask: Mode) -> Self {
        Self(rustix::process::umask(mask))
    }
}

impl Drop for UmaskGuard {
    fn drop(&mut self) {
        rustix::process::umask(self.0);
    }
}

fn assert_invalid_target(path: &Path) {
    let outcome = write_report(
        &OutputRequest::new(OutputDestination::Path(path), true, &[]),
        b"report\n",
        &mut Vec::new(),
    );
    assert_eq!(
        outcome
            .as_ref()
            .err()
            .and_then(|error| error.issue())
            .map(Issue::code),
        Some(IssueCode::OutputInvalidTarget)
    );
}

#[test]
fn symlink_and_dangling_symlink_are_refused_without_following()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let regular = directory.path().join("regular");
    let link = directory.path().join("link");
    let dangling = directory.path().join("dangling");
    fs::write(&regular, b"prior")?;
    symlink(&regular, &link)?;
    symlink(directory.path().join("missing"), &dangling)?;

    // When / Then
    assert_invalid_target(&link);
    assert_invalid_target(&dangling);
    assert_eq!(fs::read(&regular)?, b"prior");
    Ok(())
}

#[test]
fn directory_fifo_socket_and_device_are_refused_without_opening()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let fifo = directory.path().join("fifo");
    let socket = directory.path().join("socket");
    mkfifoat(CWD, &fifo, Mode::RUSR | Mode::WUSR)?;
    let _listener = std::os::unix::net::UnixListener::bind(&socket)?;

    // When / Then
    assert_invalid_target(directory.path());
    assert_invalid_target(&fifo);
    assert_invalid_target(&socket);
    assert_invalid_target(Path::new("/dev/null"));
    Ok(())
}

#[test]
fn absent_parent_and_unwritable_parent_are_output_io() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let absent = directory.path().join("missing/report.txt");
    let locked = directory.path().join("locked");
    fs::create_dir(&locked)?;
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o500))?;

    // When
    let absent_error = write_report(
        &OutputRequest::new(OutputDestination::Path(&absent), false, &[]),
        b"report\n",
        &mut Vec::new(),
    )
    .err()
    .ok_or("absent parent did not fail")?;

    // Then
    assert_eq!(
        absent_error.issue().map(Issue::code),
        Some(IssueCode::OutputIo)
    );
    if rustix::process::geteuid().as_raw() != 0 {
        let locked_output = locked.join("report.txt");
        let locked_error = write_report(
            &OutputRequest::new(OutputDestination::Path(&locked_output), false, &[]),
            b"report\n",
            &mut Vec::new(),
        )
        .err()
        .ok_or("unwritable parent did not fail")?;
        assert_eq!(
            locked_error.issue().map(Issue::code),
            Some(IssueCode::OutputIo)
        );
    }
    Ok(())
}

#[test]
fn new_file_mode_is_0666_subject_to_umask() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempdir()?;
    let output = directory.path().join("report.txt");
    let _umask = UmaskGuard::set(Mode::RGRP | Mode::WGRP | Mode::ROTH | Mode::WOTH);

    // When
    write_report(
        &OutputRequest::new(OutputDestination::Path(&output), false, &[]),
        b"report\n",
        &mut Vec::new(),
    )?;

    // Then
    assert_eq!(fs::metadata(&output)?.permissions().mode() & 0o777, 0o600);
    Ok(())
}
