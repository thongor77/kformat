//! kformat core: disk model, safety guardrails and the udisks2 backend.
pub mod backend;
pub mod cli;
pub mod fs;
pub mod guard;
pub mod inhibit;
pub mod model;
pub mod plan;
pub mod preset;
pub mod session;
pub mod udisks;
pub mod view;
