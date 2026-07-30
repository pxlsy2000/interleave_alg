use serde::{
    Serialize, Serializer,
    ser::{Error as _, SerializeStruct},
};

use super::serialize_complete;

struct FailsAfterPrefix;

impl Serialize for FailsAfterPrefix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut report = serializer.serialize_struct("Report", 2)?;
        report.serialize_field("schema_version", &1)?;
        Err(S::Error::custom("injected failure after prefix"))
    }
}

#[test]
fn serialization_failure_exposes_no_partial_buffer() {
    // Given
    let value = FailsAfterPrefix;

    // When
    let result = serialize_complete(&value);

    // Then
    assert!(result.is_err());
}
