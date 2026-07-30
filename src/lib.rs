#![doc = "Internal implementation surface for the `interleave` binary."]

mod app;
pub mod cli;
pub mod error;
pub mod input;
pub mod io;
pub mod issue;
/// Strict Mapping schema and domain types.
pub mod mapping;
/// Exact per-test congestion analysis.
pub mod metrics;
/// Canonical command reports and deterministic renderers.
pub mod report;
/// Strict Scenario schema and domain types.
pub mod scenario;
pub mod templates;

pub use app::run;
pub use cli::command;
