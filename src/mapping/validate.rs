use crate::issue::{CheckStatus, Issue, IssueCode, IssuePath};

use super::{
    matrix::MappingMatrices,
    model::{LocalAddressMode, MappingModel},
    validation_model::{
        MappingCheck, MappingCheckId, MappingCheckObservation, MappingClassification,
        MappingValidation,
    },
};

/// Runs all three GF(2) checks without short-circuiting.
pub fn validate_mapping(mapping: &MappingModel) -> MappingValidation {
    let matrices = MappingMatrices::build(mapping);
    let rank_m = matrices.target.rank();
    let rank_f = matrices.combined.rank();
    let rank_m_low = matrices.target_low.rank();
    let l_matches = matrices.local_matches_preserve_high();
    let target_passes = rank_m == mapping.target_bits;
    let bijective_passes = rank_f == mapping.line_bits;
    let natural_passes = rank_m_low == mapping.target_bits && l_matches;
    let facts = ValidationFacts {
        target: Predicate::from_bool(target_passes),
        bijective: Predicate::from_bool(bijective_passes),
        low_rank: Predicate::from_bool(rank_m_low == mapping.target_bits),
        preserve_high: Predicate::from_bool(l_matches),
    };

    let messages = ValidationMessages {
        target: if target_passes {
            "all targets are reachable".to_owned()
        } else {
            format!("rank(M)={rank_m}, expected {}", mapping.target_bits)
        },
        bijective: if bijective_passes {
            "mapping is bijective".to_owned()
        } else {
            format!("rank(F)={rank_f}, expected {}", mapping.line_bits)
        },
        natural: natural_message(rank_m_low, mapping.target_bits, l_matches),
    };

    let classification = facts.classification();
    let issues = primary_issue(mapping, &facts, &messages);
    let checks = [
        MappingCheck {
            id: MappingCheckId::TargetReachable,
            status: pass_or_fail(target_passes),
            observed: MappingCheckObservation::TargetReachable { rank_m },
            expected: MappingCheckObservation::TargetReachable {
                rank_m: mapping.target_bits,
            },
            message: messages.target,
        },
        MappingCheck {
            id: MappingCheckId::Bijective,
            status: pass_or_fail(bijective_passes),
            observed: MappingCheckObservation::Bijective { rank_f },
            expected: MappingCheckObservation::Bijective {
                rank_f: mapping.line_bits,
            },
            message: messages.bijective,
        },
        MappingCheck {
            id: MappingCheckId::NaturalLocalAddress,
            status: natural_status(target_passes, bijective_passes, natural_passes),
            observed: MappingCheckObservation::NaturalLocalAddress {
                rank_m_low,
                l_matches_preserve_high: l_matches,
            },
            expected: MappingCheckObservation::NaturalLocalAddress {
                rank_m_low: mapping.target_bits,
                l_matches_preserve_high: true,
            },
            message: messages.natural,
        },
    ];

    MappingValidation::new(checks, classification, issues)
}

struct ValidationFacts {
    target: Predicate,
    bijective: Predicate,
    low_rank: Predicate,
    preserve_high: Predicate,
}

impl ValidationFacts {
    const fn classification(&self) -> MappingClassification {
        match (self.target, self.bijective, self.natural()) {
            (
                Predicate::Fail,
                Predicate::Pass | Predicate::Fail,
                Predicate::Pass | Predicate::Fail,
            ) => MappingClassification::InvalidTargetUnreachable,
            (Predicate::Pass, Predicate::Fail, Predicate::Pass | Predicate::Fail) => {
                MappingClassification::InvalidNonBijective
            }
            (Predicate::Pass, Predicate::Pass, Predicate::Pass) => {
                MappingClassification::ValidNatural
            }
            (Predicate::Pass, Predicate::Pass, Predicate::Fail) => {
                MappingClassification::ValidNonNatural
            }
        }
    }

    const fn natural(&self) -> Predicate {
        match (self.low_rank, self.preserve_high) {
            (Predicate::Pass, Predicate::Pass) => Predicate::Pass,
            (Predicate::Pass | Predicate::Fail, Predicate::Fail)
            | (Predicate::Fail, Predicate::Pass) => Predicate::Fail,
        }
    }
}

#[derive(Clone, Copy)]
enum Predicate {
    Pass,
    Fail,
}

impl Predicate {
    const fn from_bool(passes: bool) -> Self {
        if passes { Self::Pass } else { Self::Fail }
    }

    const fn passes(self) -> bool {
        match self {
            Self::Pass => true,
            Self::Fail => false,
        }
    }
}

struct ValidationMessages {
    target: String,
    bijective: String,
    natural: String,
}

const fn pass_or_fail(passes: bool) -> CheckStatus {
    if passes {
        CheckStatus::Pass
    } else {
        CheckStatus::Fail
    }
}

const fn natural_status(
    target_passes: bool,
    bijective_passes: bool,
    natural_passes: bool,
) -> CheckStatus {
    if natural_passes {
        CheckStatus::Pass
    } else if target_passes && bijective_passes {
        CheckStatus::Warning
    } else {
        CheckStatus::Fail
    }
}

fn natural_message(rank_m_low: u8, target_bits: u8, l_matches: bool) -> String {
    match (rank_m_low == target_bits, l_matches) {
        (true, true) => "local address is naturally ordered".to_owned(),
        (false, true) => format!("rank(Mp)={rank_m_low}, expected {target_bits}"),
        (true, false) => format!("rank(Mp)={rank_m_low}; L != [0 I]"),
        (false, false) => {
            format!("rank(Mp)={rank_m_low}, expected {target_bits}; L != [0 I]")
        }
    }
}

fn primary_issue(
    mapping: &MappingModel,
    facts: &ValidationFacts,
    messages: &ValidationMessages,
) -> Vec<Issue> {
    let issue = match facts.classification() {
        MappingClassification::ValidNatural => return Vec::new(),
        MappingClassification::InvalidTargetUnreachable => Issue::new(
            IssueCode::MappingTargetUnreachable,
            IssuePath::root().field("mapping").field("m").field("rows"),
            &messages.target,
        ),
        MappingClassification::InvalidNonBijective => Issue::new(
            IssueCode::MappingNonBijective,
            non_bijective_path(mapping.local_address_mode()),
            &messages.bijective,
        ),
        MappingClassification::ValidNonNatural => Issue::new(
            IssueCode::MappingNonNatural,
            non_natural_path(facts.low_rank.passes(), facts.preserve_high.passes()),
            &messages.natural,
        ),
    };
    vec![issue]
}

fn non_bijective_path(mode: LocalAddressMode) -> IssuePath {
    match mode {
        LocalAddressMode::PreserveHigh => {
            IssuePath::root().field("mapping").field("m").field("rows")
        }
        LocalAddressMode::Explicit => IssuePath::root().field("mapping").field("l").field("rows"),
    }
}

fn non_natural_path(low_rank_passes: bool, l_matches: bool) -> IssuePath {
    match (low_rank_passes, l_matches) {
        (false, true) => IssuePath::root().field("mapping").field("m").field("rows"),
        (true, false) => IssuePath::root().field("mapping").field("l").field("rows"),
        (false, false) | (true, true) => IssuePath::root().field("mapping"),
    }
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
