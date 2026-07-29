#![doc = "Internal implementation surface for the `interleave` binary."]

mod app;
mod cli;
pub mod error;
pub mod input;
pub mod io;
pub mod issue;
/// Strict Mapping schema and domain types.
pub mod mapping;
mod metrics;
mod report;
mod scenario;
mod templates;

pub use cli::command;
