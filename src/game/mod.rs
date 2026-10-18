//! Game layer: pure simulation, no rendering or input concerns.

pub mod ai;
pub mod map;
pub mod save;
pub mod sim;
pub mod types;
pub mod world;

pub use types::*;
pub use world::World;
