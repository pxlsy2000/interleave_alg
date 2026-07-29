//! Bounded acquisition of file and stream input snapshots.

use std::{
    fs::File,
    io::{self, Read},
    os::unix::fs::MetadataExt as _,
    path::Path,
};

use thiserror::Error;

use crate::{
    input::limits::MAX_RAW_INPUT_BYTES,
    issue::{Issue, IssueCode, IssuePath},
};

const READ_CHUNK_BYTES: usize = 8 * 1024;

/// Device and inode identity retained for an accepted regular-file snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InputIdentity {
    device: u64,
    inode: u64,
}

impl InputIdentity {
    /// Returns the source device number.
    pub const fn device(self) -> u64 {
        self.device
    }

    /// Returns the source inode number.
    pub const fn inode(self) -> u64 {
        self.inode
    }
}

/// Complete input bytes accepted only after EOF within the v1 bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputSnapshot {
    bytes: Vec<u8>,
    identity: Option<InputIdentity>,
}

impl InputSnapshot {
    /// Returns the complete retained source bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns regular-file identity when the source was a named regular file.
    pub const fn identity(&self) -> Option<InputIdentity> {
        self.identity
    }
}

/// Failure while acquiring one bounded input snapshot.
#[derive(Debug, Error)]
pub enum InputReadError {
    /// The source returned an I/O error before an accepted EOF.
    #[error("input could not be read")]
    Io {
        /// Stable diagnostic for the boundary failure.
        issue: Issue,
        /// Underlying operating-system error.
        #[source]
        source: io::Error,
    },
    /// The reader yielded byte 16 MiB + 1.
    #[error("input exceeds the v1 raw-byte limit")]
    TooLarge {
        /// Stable raw-size diagnostic.
        issue: Issue,
    },
}

impl InputReadError {
    /// Returns the single stable issue produced by this failure.
    pub const fn issue(&self) -> &Issue {
        match self {
            Self::Io { issue, .. } | Self::TooLarge { issue } => issue,
        }
    }

    /// Returns the observed raw size when the v1 cap was exceeded.
    pub const fn observed_raw_bytes(&self) -> Option<usize> {
        match self {
            Self::Io { .. } => None,
            Self::TooLarge { .. } => Some(MAX_RAW_INPUT_BYTES + 1),
        }
    }
}

/// Reads to EOF while retaining at most 16 MiB + 1 bytes.
pub fn read_bounded(reader: impl Read) -> Result<InputSnapshot, InputReadError> {
    read_snapshot(reader, None)
}

/// Opens and bounded-reads a named source.
pub fn read_named(path: &Path) -> Result<InputSnapshot, InputReadError> {
    let file = File::open(path).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    let identity = metadata.file_type().is_file().then_some(InputIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    });
    read_snapshot(file, identity)
}

/// Bounded-reads standard input through the same reader used for named streams.
pub fn read_stdin() -> Result<InputSnapshot, InputReadError> {
    read_snapshot(io::stdin().lock(), None)
}

fn read_snapshot(
    mut reader: impl Read,
    identity: Option<InputIdentity>,
) -> Result<InputSnapshot, InputReadError> {
    let mut bytes = Vec::with_capacity(READ_CHUNK_BYTES);
    let mut chunk = [0_u8; READ_CHUNK_BYTES];
    loop {
        let remaining = (MAX_RAW_INPUT_BYTES + 1).saturating_sub(bytes.len());
        let request = remaining.min(chunk.len());
        let Some(buffer) = chunk.get_mut(..request) else {
            return Err(too_large());
        };
        let count = match reader.read(buffer) {
            Ok(0) => return Ok(InputSnapshot { bytes, identity }),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(error)),
        };
        let Some(received) = buffer.get(..count) else {
            return Err(io_error(io::Error::new(
                io::ErrorKind::InvalidData,
                "reader returned an invalid byte count",
            )));
        };
        bytes.extend_from_slice(received);
        if bytes.len() > MAX_RAW_INPUT_BYTES {
            return Err(too_large());
        }
    }
}

fn io_error(source: io::Error) -> InputReadError {
    InputReadError::Io {
        issue: Issue::new(
            IssueCode::InputIo,
            IssuePath::root(),
            "input could not be read",
        ),
        source,
    }
}

fn too_large() -> InputReadError {
    InputReadError::TooLarge {
        issue: Issue::new(
            IssueCode::InputInvalidValue,
            IssuePath::root(),
            "expected at most 16777216 raw bytes, observed 16777217",
        ),
    }
}
