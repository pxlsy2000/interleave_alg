use crate::input::scalar::AddressMagnitude;

use super::model::{AccessCount, Schedule, StreamName};

/// One structurally valid stride case payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrideScenario {
    pub(super) base_bytes: AddressMagnitude,
    pub(super) stride_bytes: AddressMagnitude,
    pub(super) accesses: Option<AccessCount>,
}

impl StrideScenario {
    /// Returns the first byte address.
    pub const fn base_bytes(&self) -> &AddressMagnitude {
        &self.base_bytes
    }

    /// Returns the byte stride.
    pub const fn stride_bytes(&self) -> &AddressMagnitude {
        &self.stride_bytes
    }

    /// Returns the case override, if declared.
    pub const fn accesses(&self) -> Option<AccessCount> {
        self.accesses
    }
}

/// One source-ordered sweep payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepScenario {
    pub(super) base_bytes: Vec<AddressMagnitude>,
    pub(super) stride_bytes: Vec<AddressMagnitude>,
    pub(super) accesses: Option<AccessCount>,
}

impl SweepScenario {
    /// Returns bases in declaration order.
    pub fn base_bytes(&self) -> &[AddressMagnitude] {
        &self.base_bytes
    }

    /// Returns strides in declaration order.
    pub fn stride_bytes(&self) -> &[AddressMagnitude] {
        &self.stride_bytes
    }

    /// Returns the per-combination override, if declared.
    pub const fn accesses(&self) -> Option<AccessCount> {
        self.accesses
    }
}

/// One structurally valid stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamScenario {
    pub(super) name: StreamName,
    pub(super) base_bytes: AddressMagnitude,
    pub(super) stride_bytes: AddressMagnitude,
    pub(super) accesses: AccessCount,
}

impl StreamScenario {
    /// Returns the stream name.
    pub const fn name(&self) -> &StreamName {
        &self.name
    }

    /// Returns the first byte address.
    pub const fn base_bytes(&self) -> &AddressMagnitude {
        &self.base_bytes
    }

    /// Returns the byte stride.
    pub const fn stride_bytes(&self) -> &AddressMagnitude {
        &self.stride_bytes
    }

    /// Returns the stream access count.
    pub const fn accesses(&self) -> AccessCount {
        self.accesses
    }
}

/// One source-ordered multi-stream payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiStreamScenario {
    pub(super) schedule: Schedule,
    pub(super) streams: Vec<StreamScenario>,
}

impl MultiStreamScenario {
    /// Returns the merge schedule.
    pub const fn schedule(&self) -> Schedule {
        self.schedule
    }

    /// Returns streams in declaration order.
    pub fn streams(&self) -> &[StreamScenario] {
        &self.streams
    }
}
