//! Camera: world↔screen transforms, pan and cursor-anchored zoom.
//!
//! The camera works in *tile* units and screen pixels with Y pointing down
//! (egui convention). Bevy renders with Y up, so [`Camera::bevy_transform`]
//! maps our state onto a Bevy 2D camera.

use bevy_egui::egui::{Pos2, Rect, Vec2};

/// Pixels per tile (world unit) limits.
pub const ZOOM_MIN: f32 = 6.0;
pub const ZOOM_MAX: f32 = 48.0;

/// `center` is in world units (tiles); `zoom` is pixels per tile.
#[derive(Clone, Copy)]
pub struct Camera {
    pub center: Vec2,
    pub zoom: f32,
}

impl Camera {
    pub fn new(center: Vec2, zoom: f32) -> Self {
        Self {
            center,
            zoom: zoom.clamp(ZOOM_MIN, ZOOM_MAX),
        }
    }

    pub fn world_to_screen(&self, rect: Rect, w: Vec2) -> Pos2 {
        rect.center() + (w - self.center) * self.zoom
    }

    pub fn screen_to_world(&self, rect: Rect, p: Pos2) -> Vec2 {
        self.center + (p - rect.center()) / self.zoom
    }

    /// Zoom by `factor` keeping the world point under `cursor` fixed.
    pub fn zoom_at(&mut self, rect: Rect, cursor: Pos2, factor: f32) {
        let before = self.screen_to_world(rect, cursor);
        self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        let after = self.screen_to_world(rect, cursor);
        self.center += before - after;
    }

    /// Pan by a screen-space delta (drag: content follows the pointer).
    pub fn pan_px(&mut self, delta: Vec2) {
        self.center -= delta / self.zoom;
    }

    /// Keep the view near the map so you can't pan into the void.
    pub fn clamp_center(&mut self, cols: i32, rows: i32) {
        self.center.x = self.center.x.clamp(-4.0, cols as f32 + 4.0);
        self.center.y = self.center.y.clamp(-4.0, rows as f32 + 4.0);
    }

    /// The world point that appears at the centre of the window.
    ///
    /// `viewport` is the map area in screen pixels (egui, Y down);
    /// `window` is the whole window size in the same units.
    pub fn window_center_world(&self, viewport: Rect, window: Vec2) -> Vec2 {
        self.center + (window * 0.5 - viewport.center().to_vec2()) / self.zoom
    }

    /// Translation for a Bevy 2D camera so its view matches this camera.
    /// World position mapping is `bevy = (x, -y)`; the camera sits at `z = 0`,
    /// which is the canonical 2D setup (near/far span in `default_2d`).
    pub fn bevy_translation(&self, viewport: Rect, window: Vec2) -> (f32, f32, f32) {
        let c = self.window_center_world(viewport, window);
        (c.x, -c.y, 0.0)
    }
}
