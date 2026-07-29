#![doc = "Internal implementation surface for the `interleave` binary."]

mod app;
mod cli;
mod error;
mod input;
mod io;
mod issue;
mod mapping;
mod metrics;
mod report;
mod scenario;
mod templates;

pub use cli::command;
