//! Core ratmac library.

pub mod abandon;
pub mod blocked;
pub mod cli;
pub mod completion;
pub mod contract;
pub mod doctor;
pub mod goal;
pub mod graph;
pub mod ledger;
pub mod lock;
pub mod machine;
pub mod mint;
pub mod model;
pub mod ownership;
pub mod pin;
pub mod receipt;
pub mod root;
pub mod roots;
pub mod scaffold;
pub mod scheduler;
pub mod state;
pub mod verdict;

pub use scheduler::{GuardFailure, RespawnRequest, Scheduler, StepOutcome, StepRequest};
pub use state::{StateError, StatePrompt, StateStore, StatusReport};
