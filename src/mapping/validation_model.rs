use serde::Serialize;

use crate::issue::{CheckStatus, Issue};

/// Stable identifier for one mathematical Mapping check.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingCheckId {
    /// Target reachability through `M`.
    TargetReachable,
    /// Bijectivity through stacked `F`.
    Bijective,
    /// Natural local-address ordering.
    NaturalLocalAddress,
}

/// Exact observed or expected values for a mathematical check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MappingCheckObservation {
    /// Rank of the Target matrix.
    TargetReachable {
        /// Observed or expected `rank(M)`.
        rank_m: u8,
    },
    /// Rank of the stacked matrix.
    Bijective {
        /// Observed or expected `rank(F)`.
        rank_f: u8,
    },
    /// Low Target rank and preserve-high equality.
    NaturalLocalAddress {
        /// Observed or expected `rank(M_p)`.
        rank_m_low: u8,
        /// Whether `L` equals `[0 I]`.
        l_matches_preserve_high: bool,
    },
}

/// One complete mathematical Mapping check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MappingCheck {
    pub(super) id: MappingCheckId,
    pub(super) status: CheckStatus,
    pub(super) observed: MappingCheckObservation,
    pub(super) expected: MappingCheckObservation,
    pub(super) message: String,
}

impl MappingCheck {
    /// Returns the fixed check identifier.
    pub const fn id(&self) -> MappingCheckId {
        self.id
    }

    /// Returns the check status.
    pub const fn status(&self) -> CheckStatus {
        self.status
    }

    /// Returns the exact observed object.
    pub const fn observed(&self) -> &MappingCheckObservation {
        &self.observed
    }

    /// Returns the exact expected object.
    pub const fn expected(&self) -> &MappingCheckObservation {
        &self.expected
    }

    /// Returns the exact check message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Final mathematical Mapping classification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingClassification {
    /// Reachable, bijective, and naturally ordered.
    ValidNatural,
    /// Reachable and bijective with non-natural local-address order.
    ValidNonNatural,
    /// At least one Target is unreachable.
    InvalidTargetUnreachable,
    /// Targets are reachable but the combined Mapping is not bijective.
    InvalidNonBijective,
}

impl MappingClassification {
    /// Returns the stable lowercase report value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValidNatural => "valid_natural",
            Self::ValidNonNatural => "valid_non_natural",
            Self::InvalidTargetUnreachable => "invalid_target_unreachable",
            Self::InvalidNonBijective => "invalid_non_bijective",
        }
    }
}

/// Completed Mapping checks, classification, and primary mathematical issue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingValidation {
    checks: [MappingCheck; 3],
    classification: MappingClassification,
    issues: Vec<Issue>,
}

impl MappingValidation {
    pub(super) const fn new(
        checks: [MappingCheck; 3],
        classification: MappingClassification,
        issues: Vec<Issue>,
    ) -> Self {
        Self {
            checks,
            classification,
            issues,
        }
    }

    /// Returns all three checks in normative order.
    pub const fn checks(&self) -> &[MappingCheck; 3] {
        &self.checks
    }

    /// Returns the final mathematical classification.
    pub const fn classification(&self) -> MappingClassification {
        self.classification
    }

    /// Returns the sole primary error, sole warning, or an empty slice.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }
}
