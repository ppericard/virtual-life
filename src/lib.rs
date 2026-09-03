//! VirtualLife simulation core.
//!
//! The core contains only the mathematical state and transition machinery.
//! Visualization, recording, and analysis live outside this library.

mod model;
mod simulation;
mod snapshot;

pub use model::{Agent, AgentId, Position, World};
pub use simulation::{Simulation, SimulationConfig};
pub use snapshot::{AgentSnapshot, Snapshot};
