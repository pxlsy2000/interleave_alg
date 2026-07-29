use thiserror::Error;

use crate::{
    error::ExitClass,
    input::{SpannedYamlDocument, SpannedYamlKind},
    issue::{Issue, IssuePath},
};

use super::{
    decode_support::{find, missing, unknown},
    model::MappingModel,
};

pub(super) struct MappingDecoder {
    pub(super) issues: Vec<Issue>,
}

pub(super) struct AddressDraft {
    pub(super) width: crate::input::scalar::AddressWidth,
    pub(super) granule: super::model::GranuleBytes,
}

pub(super) struct TargetDraft {
    pub(super) count: super::model::TargetCount,
}

pub(super) struct MatrixDraft {
    pub(super) target_rows: Vec<super::model::XorRow>,
    pub(super) local_rows: super::model::LocalAddressRows,
}

pub(super) struct DimensionProbe {
    pub(super) address_width: Option<u8>,
    pub(super) offset_bits: Option<u8>,
    pub(super) line_bits: Option<u8>,
    pub(super) target_bits: Option<u8>,
    pub(super) local_bits: Option<u8>,
}

/// A deterministic list of Mapping schema issues.
#[derive(Debug, Error)]
#[error("Mapping input is invalid")]
pub struct MappingDecodeError {
    issues: Vec<Issue>,
}

impl MappingDecodeError {
    /// Returns issues in normative emitter order.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Returns Mapping failure exit class 2.
    pub const fn exit_class(&self) -> ExitClass {
        ExitClass::Mapping
    }
}

/// Decodes and structurally validates one strict YAML Mapping document.
pub fn decode_mapping(document: &SpannedYamlDocument) -> Result<MappingModel, MappingDecodeError> {
    let SpannedYamlKind::Mapping(root) = document.root().kind() else {
        return Err(MappingDecodeError {
            issues: vec![super::decode_support::invalid(
                IssuePath::root(),
                "mapping",
                document.root(),
            )],
        });
    };

    let probe = MappingDecoder::probe_dimensions(root);
    let mut decoder = MappingDecoder { issues: Vec::new() };
    let mut schema_version = None;
    let mut name = None;
    let mut address = None;
    let mut targets = None;
    let mut matrix = None;

    for entry in root {
        match entry.key().value() {
            "schema_version" => schema_version = decoder.schema_version(entry.value()),
            "name" => name = decoder.name(entry.value()),
            "address" => address = decoder.address(entry.value(), &probe),
            "targets" => targets = decoder.targets(entry.value(), &probe),
            "mapping" => matrix = decoder.matrix(entry.value(), &probe),
            key => decoder.issues.push(unknown(IssuePath::root(), key)),
        }
    }
    for (key, present) in [
        ("schema_version", find(root, "schema_version").is_some()),
        ("name", find(root, "name").is_some()),
        ("address", find(root, "address").is_some()),
        ("targets", find(root, "targets").is_some()),
        ("mapping", find(root, "mapping").is_some()),
    ] {
        if !present {
            decoder.issues.push(missing(IssuePath::root().field(key)));
        }
    }

    if !decoder.issues.is_empty() {
        return Err(MappingDecodeError {
            issues: decoder.issues,
        });
    }
    let Some((name, address, targets, matrix)) = name
        .zip(address)
        .zip(targets)
        .zip(matrix)
        .map(|(((name, address), targets), matrix)| (name, address, targets, matrix))
    else {
        return Err(MappingDecodeError {
            issues: decoder.issues,
        });
    };
    if schema_version != Some(()) {
        return Err(MappingDecodeError {
            issues: decoder.issues,
        });
    }
    let Some((offset_bits, line_bits, target_bits, local_address_bits)) = probe
        .offset_bits
        .zip(probe.line_bits)
        .zip(probe.target_bits)
        .zip(probe.local_bits)
        .map(|(((offset, line), target), local)| (offset, line, target, local))
    else {
        return Err(MappingDecodeError {
            issues: decoder.issues,
        });
    };

    Ok(MappingModel {
        name,
        address_width: address.width,
        granule_bytes: address.granule,
        target_count: targets.count,
        offset_bits,
        line_bits,
        target_bits,
        local_address_bits,
        target_rows: matrix.target_rows,
        local_rows: matrix.local_rows,
    })
}
