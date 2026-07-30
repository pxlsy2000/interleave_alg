use crate::{
    input::{ScalarKind, SpannedMappingEntry, SpannedYamlNode, scalar::Address},
    issue::{IssuePath, canonical_json_string},
};

use super::{
    decode::ScenarioDecoder,
    decode_case::{allowed_common, present},
    decode_support::{invalid, invalid_observed, parse_string, unknown},
    model::{AccessCount, ScenarioCaseKind, Schedule},
    model_kind::{MultiStreamScenario, StreamScenario, StrideScenario, SweepScenario},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CaseKindTag {
    Stride,
    Sweep,
    MultiStream,
    Invalid,
}

impl CaseKindTag {
    pub(super) fn probe(node: Option<&SpannedYamlNode>) -> Self {
        let Some(scalar) = node.and_then(SpannedYamlNode::as_scalar) else {
            return Self::Invalid;
        };
        if scalar.kind() != ScalarKind::String {
            return Self::Invalid;
        }
        match scalar.value() {
            "stride" => Self::Stride,
            "sweep" => Self::Sweep,
            "multi_stream" => Self::MultiStream,
            _ => Self::Invalid,
        }
    }

    pub(super) const fn is_valid(self) -> bool {
        matches!(self, Self::Stride | Self::Sweep | Self::MultiStream)
    }

    pub(super) fn decode(decoder: &mut ScenarioDecoder, node: &SpannedYamlNode, path: IssuePath) {
        let Some(value) = parse_string(node) else {
            decoder.issues.push(invalid(path, "string", node));
            return;
        };
        if !matches!(value.value(), "stride" | "sweep" | "multi_stream") {
            decoder.issues.push(invalid_observed(
                path,
                "one of [\"stride\",\"sweep\",\"multi_stream\"]",
                &canonical_json_string(value.value()),
            ));
        }
    }
}

pub(super) enum CasePayloadDraft {
    Stride {
        base: Option<Address>,
        stride: Option<Address>,
        accesses: Option<AccessCount>,
    },
    Sweep {
        bases: Option<Vec<Address>>,
        strides: Option<Vec<Address>>,
        accesses: Option<AccessCount>,
    },
    MultiStream {
        schedule: Option<Schedule>,
        streams: Option<Vec<StreamScenario>>,
    },
    Invalid,
}

impl CasePayloadDraft {
    pub(super) const fn new(kind: CaseKindTag) -> Self {
        match kind {
            CaseKindTag::Stride => Self::Stride {
                base: None,
                stride: None,
                accesses: None,
            },
            CaseKindTag::Sweep => Self::Sweep {
                bases: None,
                strides: None,
                accesses: None,
            },
            CaseKindTag::MultiStream => Self::MultiStream {
                schedule: None,
                streams: None,
            },
            CaseKindTag::Invalid => Self::Invalid,
        }
    }

    pub(super) fn finish(self) -> Option<ScenarioCaseKind> {
        match self {
            Self::Stride {
                base,
                stride,
                accesses,
            } => base.zip(stride).map(|(base_bytes, stride_bytes)| {
                ScenarioCaseKind::Stride(StrideScenario {
                    base_bytes,
                    stride_bytes,
                    accesses,
                })
            }),
            Self::Sweep {
                bases,
                strides,
                accesses,
            } => bases.zip(strides).map(|(base_bytes, stride_bytes)| {
                ScenarioCaseKind::Sweep(SweepScenario {
                    base_bytes,
                    stride_bytes,
                    accesses,
                })
            }),
            Self::MultiStream { schedule, streams } => {
                schedule.zip(streams).map(|(schedule, streams)| {
                    ScenarioCaseKind::MultiStream(MultiStreamScenario { schedule, streams })
                })
            }
            Self::Invalid => None,
        }
    }
}

impl ScenarioDecoder {
    pub(super) fn kind_field(
        &mut self,
        draft: &mut CasePayloadDraft,
        entry: &SpannedMappingEntry,
        path: IssuePath,
        key: &str,
    ) {
        match draft {
            CasePayloadDraft::Stride {
                base,
                stride,
                accesses,
            } => match key {
                "base_bytes" => *base = self.address(entry.value(), path.field(key)),
                "stride_bytes" => *stride = self.address(entry.value(), path.field(key)),
                "accesses" => *accesses = self.access_count(entry.value(), path.field(key)),
                _ if allowed_common(key) => {}
                _ => self.issues.push(unknown(path, key)),
            },
            CasePayloadDraft::Sweep {
                bases,
                strides,
                accesses,
            } => match key {
                "base_bytes" => *bases = self.address_list(entry.value(), path.field(key)),
                "stride_bytes" => *strides = self.address_list(entry.value(), path.field(key)),
                "accesses" => *accesses = self.access_count(entry.value(), path.field(key)),
                _ if allowed_common(key) => {}
                _ => self.issues.push(unknown(path, key)),
            },
            CasePayloadDraft::MultiStream { schedule, streams } => match key {
                "schedule" => *schedule = self.schedule(entry.value(), path.field(key)),
                "streams" => *streams = self.streams(entry.value(), path.field(key)),
                "accesses" => self.issues.push(invalid(
                    path.clone().field(key),
                    &format!("field absent when {}.kind=\"multi_stream\"", path.as_str()),
                    entry.value(),
                )),
                _ if allowed_common(key) => {}
                _ => self.issues.push(unknown(path, key)),
            },
            CasePayloadDraft::Invalid => {}
        }
    }

    pub(super) fn missing_kind_fields(
        &mut self,
        values: &[SpannedMappingEntry],
        kind: CaseKindTag,
        path: &IssuePath,
    ) {
        let required = match kind {
            CaseKindTag::Stride | CaseKindTag::Sweep => &["base_bytes", "stride_bytes"][..],
            CaseKindTag::MultiStream => &["schedule", "streams"][..],
            CaseKindTag::Invalid => &[][..],
        };
        for key in required {
            if !present(values, key) {
                self.issues
                    .push(super::decode_support::missing(path.clone().field(key)));
            }
        }
    }
}
