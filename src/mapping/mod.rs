mod address;
mod decode;
mod decode_probe;
mod decode_rows;
mod decode_support;
mod decode_values;
mod matrix;
mod model;
mod validate;
mod validation_model;

pub use address::{AddressMapper, AddressMappingError, InvalidMappingError, MappedAddress};
pub use decode::{MappingDecodeError, decode_mapping};
pub use model::{
    GranuleBytes, LocalAddressMode, MappingModel, MappingName, TargetCount, XorRow, XorTap,
};
pub use validate::validate_mapping;
pub use validation_model::{
    MappingCheck, MappingCheckId, MappingCheckObservation, MappingClassification, MappingValidation,
};
