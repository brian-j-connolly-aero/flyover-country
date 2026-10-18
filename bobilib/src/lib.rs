//! Solvers for BOBILib (Bilevel Optimization (Benchmark) Instance Library)
//! instances, built around problem structure the general-purpose bilevel
//! solvers do not exploit.

pub mod coupled;
pub mod fire;
pub mod general;
pub mod kip;
pub mod kp;
pub mod milp;
pub mod mkip;
pub mod model;
pub mod mps;
pub mod robust_ss;
pub mod solution;
pub mod structure;
pub mod verify;
pub mod wf;
