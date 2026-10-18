//! Engine layer: rendering, input, camera, and HUD on top of the pure game
//! simulation. egui draws everything (shapes only — zero assets); the game
//! layer knows nothing about this module.

pub mod app;
pub mod camera;
pub mod render;
pub mod ui;

pub use app::{App, Screen, Tool};
pub use camera::Camera;
