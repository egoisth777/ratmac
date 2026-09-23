//! WCP-002: the default aggregate binary for the lane sweep suites.
//!
//! One process runs every existing sweep suite with its assertions intact,
//! so an identical real-lane observation is executed once per workspace
//! invocation (`ratmac_qa::lane_runs`). Each suite stays explicitly runnable
//! as its own named target, e.g. `cargo test -p ratmac-qa --test
//! t109_pre_split_ports`; ordinary sweeps and expiry verification remain
//! separate observations.

#[path = "t108_lane_sweep.rs"]
mod t108_lane_sweep;
#[path = "t109_pre_split_ports.rs"]
mod t109_pre_split_ports;
#[path = "t110_post_split_ports.rs"]
mod t110_post_split_ports;
