use crate::{
    input::scalar::{AddressMagnitude, AddressWidth},
    issue::Issue,
};

use super::{
    expand::analysis_issue,
    expand_model::{ConcreteStimulus, ConcreteTestDescriptor, LinearAccess, StreamAccess},
    expand_prepare::{PreparedCase, PreparedKind},
};

pub(super) fn build_descriptors(
    cases: &[PreparedCase<'_>],
    width: AddressWidth,
) -> Result<Vec<ConcreteTestDescriptor>, Issue> {
    let mut tests = Vec::new();
    for case in cases {
        match &case.kind {
            PreparedKind::Linear { base, stride } => tests.push(ConcreteTestDescriptor {
                case_id: case.name.to_owned(),
                source_case: case.name.to_owned(),
                accesses: case.accesses,
                window_sizes: case.windows.to_vec(),
                stimulus: ConcreteStimulus::Linear(
                    LinearSource {
                        base,
                        stride,
                        accesses: case.accesses,
                        width,
                    }
                    .checked()?,
                ),
            }),
            PreparedKind::Sweep { bases, strides } => {
                for base in *bases {
                    for stride in *strides {
                        tests.push(ConcreteTestDescriptor {
                            case_id: format!(
                                "{}[base={},stride={}]",
                                case.name,
                                base.canonical(),
                                stride.canonical()
                            ),
                            source_case: case.name.to_owned(),
                            accesses: case.accesses,
                            window_sizes: case.windows.to_vec(),
                            stimulus: ConcreteStimulus::Linear(
                                LinearSource {
                                    base,
                                    stride,
                                    accesses: case.accesses,
                                    width,
                                }
                                .checked()?,
                            ),
                        });
                    }
                }
            }
            PreparedKind::MultiStream { streams } => {
                let stream_descriptors = streams
                    .iter()
                    .map(|stream| {
                        LinearSource {
                            base: stream.base_bytes(),
                            stride: stream.stride_bytes(),
                            accesses: stream.accesses(),
                            width,
                        }
                        .checked()
                        .map(|linear| StreamAccess::new(stream.name().clone(), linear))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                tests.push(ConcreteTestDescriptor {
                    case_id: case.name.to_owned(),
                    source_case: case.name.to_owned(),
                    accesses: case.accesses,
                    window_sizes: case.windows.to_vec(),
                    stimulus: ConcreteStimulus::RoundRobin(stream_descriptors),
                });
            }
        }
    }
    Ok(tests)
}

struct LinearSource<'magnitude> {
    base: &'magnitude AddressMagnitude,
    stride: &'magnitude AddressMagnitude,
    accesses: super::AccessCount,
    width: AddressWidth,
}

impl LinearSource<'_> {
    fn checked(self) -> Result<LinearAccess, Issue> {
        let base = self.base.in_width(self.width).ok_or_else(analysis_issue)?;
        let stride = match (self.accesses.get(), self.stride.in_width(self.width)) {
            (1, checked) => checked,
            (_, Some(checked)) => Some(checked),
            (_, None) => return Err(analysis_issue()),
        };
        Ok(LinearAccess::new(base, stride, self.accesses))
    }
}
