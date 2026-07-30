use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Write as _},
};

use rustix::{
    fd::OwnedFd,
    fs::{self, AtFlags, Mode, OFlags},
    io::Errno,
};

use super::{AtomicOutputFault, OutputError, injected_error};

pub(super) struct TempFile<'directory> {
    directory: &'directory OwnedFd,
    name: OsString,
    file: Option<File>,
    cleanup: bool,
}

impl<'directory> TempFile<'directory> {
    pub(super) fn create(directory: &'directory OwnedFd, name: OsString) -> Result<Self, Errno> {
        let descriptor = fs::openat(
            directory,
            &name,
            OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::WRONLY | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR | Mode::RGRP | Mode::WGRP | Mode::ROTH | Mode::WOTH,
        )?;
        Ok(Self {
            directory,
            name,
            file: Some(File::from(descriptor)),
            cleanup: true,
        })
    }

    pub(super) fn name(&self) -> &OsStr {
        &self.name
    }

    pub(super) fn write_and_close(
        &mut self,
        report: &[u8],
        fault: AtomicOutputFault,
    ) -> Result<(), OutputError> {
        if fault == AtomicOutputFault::Write {
            return Err(OutputError::io(injected_error("write")));
        }
        let file = self.file.as_mut().ok_or_else(|| {
            OutputError::io(io::Error::other("temporary output is already closed"))
        })?;
        file.write_all(report).map_err(OutputError::io)?;
        file.flush().map_err(OutputError::io)?;
        drop(self.file.take());
        if fault == AtomicOutputFault::Close {
            return Err(OutputError::io(injected_error("close")));
        }
        Ok(())
    }

    pub(super) const fn commit(&mut self) {
        self.cleanup = false;
    }
}

impl Drop for TempFile<'_> {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = fs::unlinkat(self.directory, &self.name, AtFlags::empty());
        }
    }
}
