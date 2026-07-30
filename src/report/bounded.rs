use std::{fmt, io};

use thiserror::Error;

/// A complete report attempted to exceed the v1 byte limit.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("report exceeds v1 limit 268435456 bytes")]
pub struct OutputTooLarge;

pub(super) struct BoundedBytes {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl BoundedBytes {
    pub(super) const fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            exceeded: false,
        }
    }

    pub(super) fn finish(self) -> Result<Vec<u8>, OutputTooLarge> {
        if self.exceeded {
            Err(OutputTooLarge)
        } else {
            Ok(self.bytes)
        }
    }

    pub(super) const fn exceeded(&self) -> bool {
        self.exceeded
    }

    #[cfg(test)]
    pub(super) fn retained(&self) -> &[u8] {
        &self.bytes
    }

    fn retain(&mut self, bytes: &[u8]) -> Result<(), OutputTooLarge> {
        let fits = self
            .limit
            .checked_sub(self.bytes.len())
            .is_some_and(|remaining| bytes.len() <= remaining);
        if self.exceeded || !fits {
            self.exceeded = true;
            return Err(OutputTooLarge);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

impl io::Write for BoundedBytes {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.retain(buffer).map_err(io::Error::other)?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Write for BoundedBytes {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.retain(value.as_bytes()).map_err(|_| fmt::Error)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::BoundedBytes;

    #[test]
    fn shared_sink_accepts_exact_limit_and_rejects_next_byte_without_retaining_it() {
        // Given
        let mut sink = BoundedBytes::new(3);

        // When
        std::io::Write::write_all(&mut sink, b"abc").expect("exact boundary must fit");
        let error = std::io::Write::write_all(&mut sink, b"d");

        // Then
        assert!(error.is_err());
        assert_eq!(sink.retained(), b"abc");
        assert!(sink.exceeded());
    }

    #[test]
    fn shared_sink_enforces_the_same_limit_for_formatted_text() {
        // Given
        let mut sink = BoundedBytes::new(3);

        // When
        write!(&mut sink, "abc").expect("exact boundary must fit");
        let error = write!(&mut sink, "d");

        // Then
        assert!(error.is_err());
        assert_eq!(sink.retained(), b"abc");
        assert!(sink.exceeded());
    }
}
