use thiserror::Error;

use crate::{
    error::ExitClass,
    input::{SpannedYamlDocument, SpannedYamlKind},
    issue::{Issue, IssuePath},
};

use super::{
    decode_support::{find, invalid, missing, unknown},
    model::ScenarioModel,
};

pub(super) struct ScenarioDecoder {
    pub(super) issues: Vec<Issue>,
}

/// A deterministic list of Scenario schema issues.
#[derive(Debug, Error)]
#[error("Scenario input is invalid")]
pub struct ScenarioDecodeError {
    issues: Vec<Issue>,
}

impl ScenarioDecodeError {
    /// Returns issues in normative emitter order.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Returns Scenario failure exit class 3.
    pub const fn exit_class(&self) -> ExitClass {
        ExitClass::ScenarioOrAddress
    }
}

/// Decodes and structurally validates one strict YAML Scenario document.
pub fn decode_scenario(
    document: &SpannedYamlDocument,
) -> Result<ScenarioModel, ScenarioDecodeError> {
    let SpannedYamlKind::Mapping(root) = document.root().kind() else {
        return Err(ScenarioDecodeError {
            issues: vec![invalid(IssuePath::root(), "mapping", document.root())],
        });
    };
    let mut decoder = ScenarioDecoder { issues: Vec::new() };
    let mut schema_version = None;
    let mut defaults = None;
    let mut cases = None;
    for entry in root {
        match entry.key().value() {
            "schema_version" => schema_version = decoder.schema_version(entry.value()),
            "defaults" => defaults = decoder.defaults(entry.value()),
            "cases" => cases = decoder.cases(entry.value()),
            key => decoder.issues.push(unknown(IssuePath::root(), key)),
        }
    }
    for key in ["schema_version", "defaults", "cases"] {
        if find(root, key).is_none() {
            decoder.issues.push(missing(IssuePath::root().field(key)));
        }
    }
    if !decoder.issues.is_empty() {
        return Err(ScenarioDecodeError {
            issues: decoder.issues,
        });
    }
    let Some(((), defaults, cases)) = schema_version
        .zip(defaults)
        .zip(cases)
        .map(|((version, defaults), cases)| (version, defaults, cases))
    else {
        return Err(ScenarioDecodeError {
            issues: decoder.issues,
        });
    };
    Ok(ScenarioModel { defaults, cases })
}
