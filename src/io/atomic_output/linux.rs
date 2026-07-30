use std::{
    ffi::{OsStr, OsString},
    io,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use rustix::{
    fd::OwnedFd,
    fs::{self, AtFlags, FileType, Mode, OFlags, RenameFlags},
    io::Errno,
};

use super::{AtomicOutputFault, OutputError, OutputRequest, injected_error, temp::TempFile};
use crate::io::input::InputIdentity;

static NEXT_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const TEMP_ATTEMPTS: usize = 128;

pub(super) fn write_path(
    request: OutputRequest<'_>,
    output: &Path,
    report: &[u8],
) -> Result<(), OutputError> {
    let (parent, name) = parent_and_name(output)?;
    let directory = fs::open(
        parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(errno_io)?;
    let initial = inspect(&directory, name, request.input_identities())?;
    if let TargetState::Regular { aliases_input } = initial
        && !request.force()
    {
        return Err(OutputError::exists(aliases_input));
    }

    let mut temporary = create_temp(&directory, request.fault())?;
    temporary.write_and_close(report, request.fault())?;
    if request.fault() == AtomicOutputFault::Precommit {
        return Err(OutputError::io(injected_error("precommit")));
    }

    if request.force() {
        let _rechecked = inspect(&directory, name, &[])?;
        fs::renameat(&directory, temporary.name(), &directory, name).map_err(errno_io)?;
    } else {
        commit_noreplace(&directory, temporary.name(), name, request.fault())?;
    }
    temporary.commit();
    Ok(())
}

fn parent_and_name(path: &Path) -> Result<(&Path, &OsStr), OutputError> {
    let Some(name) = path.file_name().filter(|name| !name.is_empty()) else {
        return match std::fs::symlink_metadata(path) {
            Ok(_) => Err(OutputError::invalid_target()),
            Err(error) => Err(OutputError::io(error)),
        };
    };
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    Ok((parent, name))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TargetState {
    Absent,
    Regular { aliases_input: bool },
}

fn inspect(
    directory: &OwnedFd,
    name: &OsStr,
    inputs: &[InputIdentity],
) -> Result<TargetState, OutputError> {
    match fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => {
            if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
                return Err(OutputError::invalid_target());
            }
            let aliases_input = inputs.iter().any(|identity| {
                identity.device() == stat.st_dev && identity.inode() == stat.st_ino
            });
            Ok(TargetState::Regular { aliases_input })
        }
        Err(Errno::NOENT) => Ok(TargetState::Absent),
        Err(error) => Err(errno_io(error)),
    }
}

fn create_temp(directory: &OwnedFd, fault: AtomicOutputFault) -> Result<TempFile<'_>, OutputError> {
    for _attempt in 0..TEMP_ATTEMPTS {
        let counter = NEXT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = OsString::from(format!(
            ".interleave.tmp.{}.{}",
            std::process::id(),
            counter
        ));
        if fault == AtomicOutputFault::TempExhaustion {
            continue;
        }
        match TempFile::create(directory, name) {
            Ok(temporary) => return Ok(temporary),
            Err(Errno::EXIST) => {}
            Err(error) => return Err(errno_io(error)),
        }
    }
    Err(OutputError::io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "temporary output name attempts exhausted",
    )))
}

fn commit_noreplace(
    directory: &OwnedFd,
    temporary: &OsStr,
    output: &OsStr,
    fault: AtomicOutputFault,
) -> Result<(), OutputError> {
    let outcome = match fault {
        AtomicOutputFault::RenameNoReplaceNoSys => Err(Errno::NOSYS),
        AtomicOutputFault::RenameNoReplaceNotSupported => Err(Errno::OPNOTSUPP),
        AtomicOutputFault::None
        | AtomicOutputFault::Write
        | AtomicOutputFault::Close
        | AtomicOutputFault::Precommit
        | AtomicOutputFault::TempExhaustion => fs::renameat_with(
            directory,
            temporary,
            directory,
            output,
            RenameFlags::NOREPLACE,
        ),
    };
    match outcome {
        Ok(()) => Ok(()),
        Err(Errno::EXIST) => Err(OutputError::exists(false)),
        Err(error) if unsupported(error) => Err(OutputError::atomic_unsupported(errno(error))),
        Err(error) => Err(errno_io(error)),
    }
}

fn unsupported(error: Errno) -> bool {
    error == Errno::NOSYS || error == Errno::INVAL || error == Errno::OPNOTSUPP
}

fn errno_io(error: Errno) -> OutputError {
    OutputError::io(errno(error))
}

fn errno(error: Errno) -> io::Error {
    io::Error::from_raw_os_error(error.raw_os_error())
}
