//! The eframe application: screens, fixed-timestep simulation, camera input,
//! and applying the `Action`s the UI collects.

use super::camera::Camera;
use super::render::{self, View};
use super::ui::{self, Action, Hud, MenuCmd, MenuState};
use crate::game::save;
use crate::game::types::*;
use crate::game::world::World;
use eframe::egui::{self, Color32, Key, PointerButton, Pos2, Rect, Sense, Vec2};
use std::time::Duration;

/// Which cursor tool is armed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Build(NodeType),
    Road { from: Option<u32> },
}

/// High-level screen. Game over is an overlay on Game (driven by `world.victory`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Game,
}

pub struct App {
    world: World,
    screen: Screen,
    camera: Camera,
    tool: Tool,
    selected: Option<u32>,
    hover: Option<u32>,
    speed: u8,
    acc: f64,
    terrain: Option<egui::TextureHandle>,
    show_help: bool,
    /// In-game pause menu (save/load/quit). While open the sim is frozen.
    show_menu: bool,
    /// Speed to restore when the pause menu closes.
    menu_resume_speed: u8,
    menu_state: MenuState,
    /// (text, expires-at) toast for command feedback.
    status: Option<(String, f64)>,
}

/// Cap on sim steps per frame (avoids spiral of death after long stalls).
const MAX_STEPS_PER_FRAME: u32 = 24;

impl App {
    pub fn new() -> Self {
        let world = World::new(42, Faction::SilkSpinners, Difficulty::Medium);
        Self {
            world,
            screen: Screen::Menu,
            camera: Camera::new(Vec2::new(28.0, 22.0), 18.0),
            tool: Tool::Select,
            selected: None,
            hover: None,
            speed: 1,
            acc: 0.0,
            terrain: None,
            show_help: false,
            show_menu: false,
            menu_resume_speed: 1,
            menu_state: MenuState {
                faction: Faction::SilkSpinners,
                difficulty: Difficulty::Medium,
            },
            status: None,
        }
    }

    fn toast(&mut self, msg: impl Into<String>, now: f64) {
        self.status = Some((msg.into(), now + 4.0));
    }

    /// Fresh world from menu choices; centers the camera on the capital.
    fn start_new_game(&mut self, ctx: &egui::Context) {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1337);
        self.world = World::new(seed, self.menu_state.faction, self.menu_state.difficulty);
        self.enter_game(ctx);
    }

    fn enter_game(&mut self, ctx: &egui::Context) {
        self.terrain = Some(render::bake_terrain(ctx, &self.world.map));
        self.screen = Screen::Game;
        self.tool = Tool::Select;
        self.selected = None;
        self.hover = None;
        self.speed = 1;
        self.acc = 0.0;
        self.status = None;
        self.show_menu = false;
        // Center on the player's first settlement (or map center).
        let focus = self
            .world
            .settlements_of(Owner::Player)
            .first()
            .map(|n| Vec2::new(n.x, n.y))
            .unwrap_or(Vec2::new(
                self.world.map.cols as f32 * 0.5,
                self.world.map.rows as f32 * 0.5,
            ));
        self.camera.center = focus;
        self.camera.zoom = 18.0;
    }

    fn open_menu(&mut self) {
        self.menu_resume_speed = if self.speed == 0 { 1 } else { self.speed };
        self.show_menu = true;
    }

    fn close_menu(&mut self) {
        self.show_menu = false;
        self.speed = self.menu_resume_speed;
    }

    fn apply(&mut self, act: Action, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        match act {
            Action::SetTool(t) => self.tool = t,
            Action::Select(id) => self.selected = id,
            Action::Build(kind, x, y) => {
                match self.world.place_building(kind, x, y, Owner::Player) {
                    Ok(_) => self.toast(format!("Built a {}.", kind.label()), now),
                    Err(e) => self.toast(e, now),
                }
            }
            Action::Road(from, to) => {
                let cost = self.world.road_cost(from, to);
                match self.world.build_road(from, to, Owner::Player) {
                    Ok(_) => self.toast(
                        format!(
                            "Road built{}.",
                            cost.map(|c| format!(" ({}g)", c)).unwrap_or_default()
                        ),
                        now,
                    ),
                    Err(e) => self.toast(e, now),
                }
            }
            Action::CutRoad(idx) => {
                if let Err(e) = self.world.cut_road(idx, Owner::Player) {
                    self.toast(e, now);
                }
            }
            Action::CaravanDelta(idx, d) => {
                if let Some(e) = self.world.edges.get_mut(idx) {
                    e.caravans = (e.caravans + d).clamp(0, MAX_CARRAVANS);
                }
            }
            Action::TrainWorker(id, tier) => {
                if let Err(e) = self.world.train_worker(id, tier, Owner::Player) {
                    self.toast(e, now);
                }
            }
            Action::DismissWorker(id, index) => {
                if let Err(e) = self.world.dismiss_worker(id, index, Owner::Player) {
                    self.toast(e, now);
                }
            }
            Action::TrainSpy(target) => {
                let name = self
                    .world
                    .node(target)
                    .map(|n| n.name.clone())
                    .unwrap_or_default();
                match self.world.train_spy(target, Owner::Player) {
                    Ok(_) => self.toast(format!("An agent slips toward {}.", name), now),
                    Err(e) => self.toast(e, now),
                }
            }
            Action::SetSpyOp(spy, op) => {
                if let Err(e) = self.world.set_spy_op(spy, op, Owner::Player) {
                    self.toast(e, now);
                }
            }
            Action::RecallSpy(spy) => {
                if let Err(e) = self.world.recall_spy(spy, Owner::Player) {
                    self.toast(e, now);
                }
            }
            Action::Foment(id) => {
                if let Err(e) = self.world.foment(id, Owner::Player) {
                    self.toast(e, now);
                } else {
                    self.toast("Whispers sown.", now);
                }
            }
            Action::BuySettlement(id) => {
                let name = self
                    .world
                    .node(id)
                    .map(|n| n.name.clone())
                    .unwrap_or_default();
                if let Err(e) = self.world.buy_settlement(id, Owner::Player) {
                    self.toast(e, now);
                } else {
                    self.toast(format!("{} swears to you.", name), now);
                }
            }
            Action::AdvanceAge => {
                if let Err(e) = self.world.advance_age(Owner::Player) {
                    self.toast(e, now);
                } else {
                    let age = self.world.player().age;
                    self.toast(format!("The {} begins.", age.label()), now);
                }
            }
            Action::Demolish(id) => {
                match self.world.demolish(id, Owner::Player) {
                    Ok(refund) => {
                        if self.selected == Some(id) {
                            self.selected = None;
                        }
                        self.toast(format!("Demolished ({}g back).", refund), now);
                    }
                    Err(e) => self.toast(e, now),
                }
            }
            Action::Save(slot) => match save::save(&self.world, slot) {
                Ok(()) => self.toast(format!("Saved to slot {}.", slot), now),
                Err(e) => self.toast(e, now),
            },
            Action::Load(slot) => match save::load(slot) {
                Ok(w) => {
                    self.world = w;
                    self.enter_game(ctx);
                    self.toast(format!("Loaded slot {}.", slot), now);
                }
                Err(e) => self.toast(e, now),
            },
            Action::NewGame => self.start_new_game(ctx),
            Action::QuitToMenu => {
                self.screen = Screen::Menu;
                self.selected = None;
                self.status = None;
                self.show_menu = false;
            }
            Action::SetSpeed(s) => {
                self.speed = s;
                // Speed buttons also dismiss the pause menu if it's open.
                if s > 0 {
                    self.show_menu = false;
                }
            }
            Action::TogglePause => {
                self.speed = if self.speed == 0 { 1 } else { 0 };
            }
            Action::ToggleHelp => self.show_help = !self.show_help,
            Action::ToggleMenu => {
                if self.show_menu {
                    self.close_menu();
                } else {
                    self.open_menu();
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // Simulation stepping
    // -----------------------------------------------------------------

    fn step_sim(&mut self, ctx: &egui::Context) {
        if self.screen != Screen::Game
            || self.speed == 0
            || self.show_menu
            || self.world.victory.is_some()
        {
            return;
        }
        // Keep spies/caravans animating smoothly while running.
        ctx.request_repaint_after(Duration::from_millis(15));

        let dt = ctx.input(|i| i.stable_dt) as f64;
        self.acc += dt * self.speed as f64;
        let mut steps = 0;
        while self.acc >= TICK_DT as f64 && steps < MAX_STEPS_PER_FRAME {
            self.acc -= TICK_DT as f64;
            self.world.tick();
            steps += 1;
        }
        if steps == MAX_STEPS_PER_FRAME {
            self.acc = 0.0;
        }
    }

    // -----------------------------------------------------------------
    // Map interaction
    // -----------------------------------------------------------------

    /// Find the node under a screen position (world-unit hit radius).
    fn node_at(&self, rect: Rect, pos: Pos2) -> Option<u32> {
        let w = self.camera.screen_to_world(rect, pos);
        let mut best: Option<(u32, f32)> = None;
        for n in self.world.nodes.values() {
            let d = (Vec2::new(n.x, n.y) - w).length();
            let r = n.radius().max(0.7);
            if d <= r && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((n.id, d));
            }
        }
        best.map(|(id, _)| id)
    }

    fn handle_map(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

        // Pan: middle drag anywhere; primary drag on empty ground with the
        // Select tool armed (so Build/Road drags never move the camera).
        let primary_drag = response.dragged_by(PointerButton::Primary);
        let middle_drag = response.dragged_by(PointerButton::Middle);
        if middle_drag || primary_drag {
            let started_on_node = response
                .interact_pointer_pos()
                .map(|p| self.node_at(rect, p).is_some())
                .unwrap_or(false);
            if middle_drag || (primary_drag && !started_on_node && self.tool == Tool::Select) {
                let d = response.drag_delta();
                self.camera.center -= d / self.camera.zoom;
                let (cols, rows) = (self.world.map.cols, self.world.map.rows);
                self.camera.clamp_center(cols, rows);
            }
        }

        // Zoom: wheel / pinch, anchored at the cursor.
        let hover_pos = response.hover_pos();
        if let Some(p) = hover_pos {
            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            let pinch = ctx.input(|i| i.zoom_delta());
            let factor = pinch * (1.0 + scroll * 0.0015);
            if (factor - 1.0).abs() > 0.001 {
                self.camera.zoom_at(rect, p, factor);
                let (cols, rows) = (self.world.map.cols, self.world.map.rows);
                self.camera.clamp_center(cols, rows);
            }
        }

        self.hover = hover_pos.and_then(|p| self.node_at(rect, p));

        // Keyboard pan (WASD / arrows), unless typing somewhere.
        if !ctx.wants_keyboard_input() {
            let (dx, dy) = ctx.input(|i| {
                let mut dx = 0.0;
                let mut dy = 0.0;
                if i.key_down(Key::A) || i.key_down(Key::ArrowLeft) {
                    dx -= 1.0;
                }
                if i.key_down(Key::D) || i.key_down(Key::ArrowRight) {
                    dx += 1.0;
                }
                if i.key_down(Key::W) || i.key_down(Key::ArrowUp) {
                    dy -= 1.0;
                }
                if i.key_down(Key::S) || i.key_down(Key::ArrowDown) {
                    dy += 1.0;
                }
                (dx, dy)
            });
            if dx != 0.0 || dy != 0.0 {
                let v = Vec2::new(dx, dy).normalized()
                    * (600.0 * ctx.input(|i| i.stable_dt))
                    / self.camera.zoom;
                self.camera.center += v;
                let (cols, rows) = (self.world.map.cols, self.world.map.rows);
                self.camera.clamp_center(cols, rows);
            }
        }

        // Right-click cancels the tool / deselects.
        if response.secondary_clicked() {
            match self.tool {
                Tool::Select => self.selected = None,
                _ => self.tool = Tool::Select,
            }
        }

        // Left click: tool action.
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos().or(response.hover_pos()) {
                match self.tool {
                    Tool::Select => {
                        let hit = self.node_at(rect, p);
                        self.selected = hit;
                    }
                    Tool::Build(kind) => {
                        let w = self.camera.screen_to_world(rect, p);
                        let act = Action::Build(kind, w.x, w.y);
                        self.apply(act, ctx);
                    }
                    Tool::Road { from } => {
                        let hit = self.node_at(rect, p);
                        if let Some(id) = hit {
                            match from {
                                None => self.tool = Tool::Road { from: Some(id) },
                                Some(a) if a != id => {
                                    let act = Action::Road(a, id);
                                    self.apply(act, ctx);
                                    self.tool = Tool::Road { from: None };
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Render the world.
        if let Some(terrain) = &self.terrain {
            let view = View {
                camera: &self.camera,
                tool: self.tool,
                selected: self.selected,
                cursor: hover_pos.map(|p| self.camera.screen_to_world(rect, p)),
                hover: self.hover,
            };
            let painter = ui.painter().with_clip_rect(rect);
            render::draw(&painter, rect, &self.world, terrain, &view);
        }

        // Toast at the bottom of the map.
        let now = ctx.input(|i| i.time);
        if let Some((msg, exp)) = &self.status {
            if now <= *exp {
                let p = Pos2::new(rect.center().x, rect.bottom() - 16.0);
                let painter = ui.painter().with_clip_rect(rect);
                painter.text(
                    p,
                    egui::Align2::CENTER_BOTTOM,
                    msg.clone(),
                    egui::FontId::proportional(14.0),
                    Color32::from_rgb(255, 240, 200),
                );
            }
        }
    }

    // -----------------------------------------------------------------
    // Hotkeys (global)
    // -----------------------------------------------------------------

    fn hotkeys(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        let (esc, h, v, r, space, s1, s2, s3, n0) = ctx.input(|i| {
            (
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::H),
                i.key_pressed(Key::V),
                i.key_pressed(Key::R),
                i.key_pressed(Key::Space),
                i.key_pressed(Key::Num1),
                i.key_pressed(Key::Num2),
                i.key_pressed(Key::Num3),
                i.key_pressed(Key::Num0),
            )
        });

        if self.screen == Screen::Menu {
            if esc && self.show_help {
                self.show_help = false;
            }
            if h {
                self.show_help = !self.show_help;
            }
            return;
        }

        // In game.
        if esc {
            if self.show_help {
                self.show_help = false;
            } else if self.show_menu {
                self.close_menu();
            } else if matches!(self.tool, Tool::Build(_) | Tool::Road { .. }) {
                self.tool = Tool::Select;
            } else {
                self.open_menu();
            }
        }
        if self.show_menu {
            // Only Esc works while the pause menu is open.
            return;
        }
        if h {
            self.show_help = !self.show_help;
        }
        if v {
            self.tool = Tool::Select;
        }
        if r {
            self.tool = Tool::Road { from: None };
        }
        if space {
            self.speed = if self.speed == 0 { 1 } else { 0 };
        }
        if s1 {
            self.speed = 1;
        }
        if s2 {
            self.speed = 2;
        }
        if s3 {
            self.speed = 4;
        }
        if n0 {
            // Quick save to slot 1.
            let now = ctx.input(|i| i.time);
            match save::save(&self.world, 1) {
                Ok(()) => self.toast("Quick-saved to slot 1.", now),
                Err(e) => self.toast(e, now),
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.hotkeys(ctx);
        self.step_sim(ctx);

        match self.screen {
            Screen::Menu => {
                let mut help = self.show_help;
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(Color32::from_rgb(16, 18, 24)))
                    .show(ctx, |ui| {
                        let cmd = ui::main_menu(ui, &mut self.menu_state);
                        if let Some(cmd) = cmd {
                            match cmd {
                                MenuCmd::Start => self.start_new_game(ctx),
                                MenuCmd::Continue => {
                                    // Most recent existing slot; prefer autosave.
                                    let slot = [0usize, 1, 2, 3]
                                        .into_iter()
                                        .find(|s| save::exists(*s))
                                        .unwrap_or(0);
                                    let act = Action::Load(slot);
                                    self.apply(act, ctx);
                                }
                                MenuCmd::Quit => {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                                }
                                MenuCmd::Help => self.show_help = true,
                                MenuCmd::SetFaction(f) => self.menu_state.faction = f,
                                MenuCmd::SetDifficulty(d) => self.menu_state.difficulty = d,
                            }
                        }
                    });
                ui::help_window(ctx, &mut help);
                self.show_help = help;
            }
            Screen::Game => {
                // Panels read state and collect actions; applied after draw.
                let mut actions: Vec<Action> = Vec::new();

                egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
                    let h = Hud {
                        world: &self.world,
                        tool: self.tool,
                        selected: self.selected,
                        speed: self.speed,
                    };
                    actions.extend(ui::top_bar(ui, &h));
                });

                egui::TopBottomPanel::bottom("log_panel")
                    .exact_height(118.0)
                    .show(ctx, |ui| {
                        ui::log_panel(ui, &self.world);
                    });

                egui::SidePanel::left("tools")
                    .resizable(true)
                    .default_width(220.0)
                    .min_width(180.0)
                    .max_width(300.0)
                    .show(ctx, |ui| {
                        let h = Hud {
                            world: &self.world,
                            tool: self.tool,
                            selected: self.selected,
                            speed: self.speed,
                        };
                        actions.extend(ui::left_panel(ui, &h));
                    });

                egui::SidePanel::right("inspector")
                    .resizable(true)
                    .default_width(330.0)
                    .min_width(260.0)
                    .max_width(430.0)
                    .show(ctx, |ui| {
                        let h = Hud {
                            world: &self.world,
                            tool: self.tool,
                            selected: self.selected,
                            speed: self.speed,
                        };
                        actions.extend(ui::right_panel(ui, &h));
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        self.handle_map(ui, ctx);
                    });

                let mut help = self.show_help;
                ui::help_window(ctx, &mut help);
                self.show_help = help;

                if self.world.victory.is_some() {
                    actions.extend(ui::game_over(ctx, &self.world));
                } else if self.show_menu {
                    actions.extend(ui::pause_menu(ctx, &self.world));
                }

                for a in actions {
                    self.apply(a, ctx);
                }
            }
        }
    }
}
