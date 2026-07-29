mod address;
mod decode;
mod decode_probe;
mod decode_rows;
mod decode_support;
mod decode_values;
mod matrix;
mod model;
mod validate;

pub use decode::{MappingDecodeError, decode_mapping};
pub use model::{
    GranuleBytes, LocalAddressMode, MappingModel, MappingName, TargetCount, XorRow, XorTap,
};
