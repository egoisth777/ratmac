//! Core ratmac library.

pub mod cli;
pub mod graph;
pub mod machine;
pub mod model;
pub mod scheduler;
pub mod state;

pub use scheduler::{GuardFailure, Scheduler, StepOutcome, StepRequest};
pub use state::{PhasePrompt, StateError, StateStore, StatusReport};
