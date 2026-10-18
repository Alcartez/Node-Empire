//! Shared engine state: the resources the systems agree on, plus the
//! interaction enums that the HUD and the input code share.

use bevy::prelude::*;
use bevy_egui::egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;

use super::camera::Camera;
use super::ui::{MenuState, RightTab};
use crate::game::types::{Difficulty, Faction, NodeType};
use crate::game::world::World;

/// Which cursor tool is armed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Build(NodeType),
    Road { from: Option<u32> },
}

/// High-level screen. Game over is an overlay on `Game` (driven by
/// `world.victory`), so it is not a state of its own.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Menu,
    Game,
}

/// Camera plus the map viewport egui measured this frame. The viewport is the
/// area left over by the HUD panels, in window pixels with Y pointing down.
#[derive(Resource)]
pub struct CamView {
    pub camera: Camera,
    pub viewport: Rect,
    pub window: Vec2,
    /// Pointer position in window pixels, when it is over the map.
    pub pointer: Option<Pos2>,
    /// Pointer position in world (tile) units.
    pub cursor: Option<Vec2>,
}

impl Default for CamView {
    fn default() -> Self {
        Self {
            camera: Camera::new(Vec2::new(28.0, 22.0), 18.0),
            viewport: Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 900.0)),
            window: Vec2::new(1400.0, 900.0),
            pointer: None,
            cursor: None,
        }
    }
}

/// The live game plus the player's interaction state.
#[derive(Resource)]
pub struct GameState {
    pub world: World,
    /// Bumped whenever the map changes (new game / load) so visuals rebuild.
    pub map_gen: u32,
    pub tool: Tool,
    pub selected: Option<u32>,
    pub hover: Option<u32>,
    /// Which page the right-hand panel shows.
    pub right_tab: RightTab,
    /// 0 = paused, otherwise the simulation speed multiplier (1, 2 or 4).
    pub speed: u8,
    pub show_help: bool,
    pub show_menu: bool,
    pub menu_resume_speed: u8,
    pub menu_state: MenuState,
    /// (message, expires-at) toast for command feedback.
    pub status: Option<(String, f64)>,
    /// Real (wall-clock) seconds, refreshed once per frame.
    pub now: f64,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            world: World::new(42, Faction::SilkSpinners, Difficulty::Medium),
            map_gen: 0,
            tool: Tool::Select,
            selected: None,
            hover: None,
            right_tab: RightTab::Realm,
            speed: 1,
            show_help: false,
            show_menu: false,
            menu_resume_speed: 1,
            menu_state: MenuState::new(),
            status: None,
            now: 0.0,
        }
    }
}

impl GameState {
    /// Replace the world (new game / loaded save), reset interaction state, and
    /// return the tile the camera should focus on (the player's first holding).
    pub fn swap_world(&mut self, world: World) -> Option<(f32, f32)> {
        let focus = world
            .settlements_of(crate::game::types::Owner::Player)
            .first()
            .map(|n| (n.x, n.y));
        self.world = world;
        self.map_gen = self.map_gen.wrapping_add(1);
        self.tool = Tool::Select;
        self.selected = None;
        self.hover = None;
        self.right_tab = RightTab::Realm;
        self.speed = 1;
        self.show_menu = false;
        self.status = None;
        focus
    }
}

/// Shapes the renderer reuses: one unit-radius mesh per node kind plus a disc.
#[derive(Default)]
pub struct Shapes {
    /// Unit-radius disc (28 sides) for dots and deposits.
    pub circle: Handle<Mesh>,
    /// Unit-radius outline meshes, indexed by [`crate::engine::render::kind_index`].
    pub kinds: [Handle<Mesh>; 9],
}

/// Colours cached as materials, keyed by small indices.
#[derive(Default)]
pub struct Mats {
    /// The extended material that draws the baked terrain quad.
    pub terrain_ext: Option<Handle<super::terrain::TerrainMaterial>>,
    /// `(kind_index, owner_index)`.
    pub fills: HashMap<(u8, u8), Handle<ColorMaterial>>,
    pub edges: HashMap<u8, Handle<ColorMaterial>>,
    /// Owner index, plus 9 = caravan and 10/11 = copper/iron specks.
    pub dots: HashMap<u8, Handle<ColorMaterial>>,
    pub ghost_ok: Option<Handle<ColorMaterial>>,
    pub ghost_bad: Option<Handle<ColorMaterial>>,
}

/// Entities backing the rendered world, refreshed as the world changes.
#[derive(Resource, Default)]
pub struct WorldVis {
    /// Which `GameState::map_gen` these visuals were built for.
    pub map_gen: u32,
    pub shapes: Shapes,
    pub mats: Mats,
    pub terrain: Option<Entity>,
    pub specks: Vec<Entity>,
    pub nodes: HashMap<u32, Entity>,
    pub spies: Vec<Entity>,
    pub caravans: Vec<Entity>,
    pub ghost: Option<Entity>,
    /// Signature of the road set the current ribbons were built from.
    pub road_sig: u64,
    pub roads: Vec<Entity>,
}
