//! Core ratmac library.

pub mod abandon;
pub mod blocked;
pub mod cli;
pub mod completion;
pub mod contract;
pub mod doctor;
pub mod goal;
pub mod graph;
pub mod machine;
pub mod model;
pub mod ownership;
pub mod pin;
pub mod receipt;
pub mod scheduler;
pub mod state;

pub use scheduler::{GuardFailure, Scheduler, StepOutcome, StepRequest};
pub use state::{PhasePrompt, StateError, StateStore, StatusReport};
