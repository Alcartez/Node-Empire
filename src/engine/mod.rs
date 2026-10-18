//! Engine layer: rendering, input, camera, and HUD on top of the pure game
//! simulation. Bevy 0.20 owns the window, camera, fixed-timestep clock, states
//! and world rendering; egui (via `bevy_egui`) draws the HUD, panels, overlays
//! and world text. Everything is shapes and code — the only asset is the WESL
//! terrain shader in `assets/shaders/` — and the game layer knows nothing about
//! this module.

pub mod app;
pub mod camera;
pub mod errors;
pub mod render;
pub mod state;
pub mod terrain;
pub mod theme;
pub mod ui;

pub use app::run;
pub use camera::Camera;
pub use state::{Screen, Tool};
