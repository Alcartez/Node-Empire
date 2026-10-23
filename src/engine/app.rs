//! The Bevy application: window, screens, fixed-timestep simulation, and the
//! systems that bridge the pure game layer to rendering and the egui HUD.
//!
//! Bevy owns the window, the 2D camera, the fixed-timestep clock and the
//! `Menu` / `Game` state machine. egui owns every glyph and panel. The world
//! itself is drawn by `render` (mesh fills + gizmo strokes) and every text
//! label on the map is painted by `ui::map_labels` in screen space, so the two
//! layers always agree as long as they share this camera.

use bevy::camera::visibility::VisibilitySystems;
use bevy::camera::{CameraUpdateSystems, ScalingMode};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPostUpdateSet, EguiPrimaryContextPass};
use egui::{Key, LayerId, PointerButton, Sense, Ui, UiBuilder, Vec2 as EVec2};

use super::errors;
use super::render;
use super::state::{CamView, GameState, Screen, Tool, WorldVis};
use super::terrain;
use super::theme;
use super::ui::{self, Action, Hud, MenuCmd, RightTab};
use crate::game::save;
use crate::game::types::*;
use crate::game::world::World;

/// Build and run the game.
pub fn run() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(window_plugin()))
        .add_plugins(EguiPlugin::default())
        .add_plugins(terrain::TerrainPlugin)
        .add_plugins(errors::ErrorPlugin)
        .init_state::<Screen>()
        // Fixed 5 Hz simulation clock; speed is virtual-time scaling (see
        // `sync_speed`), so pause and fast-forward come for free.
        .insert_resource(Time::<Fixed>::from_seconds(TICK_DT as f64))
        .insert_resource(render::map_clear_color())
        .init_resource::<GameState>()
        .init_resource::<CamView>()
        .init_resource::<WorldVis>()
        .add_systems(
            Startup,
            (setup_camera, render::setup_shapes, render::setup_gizmos).chain_weak(),
        )
        .add_systems(Update, sync_speed)
        .add_systems(FixedUpdate, sim_tick.run_if(in_state(Screen::Game)))
        // The world sync reads the map viewport egui measured this frame, so
        // it must run after the egui pass. `camera_follow` writes the camera's
        // transform and projection, so it has to sit between Bevy's transform
        // propagation and Bevy's camera update (which turns the projection into
        // the clip matrix) - otherwise the map lags the labels by a frame.
        .add_systems(
            PostUpdate,
            (camera_follow, render::sync_world, render::draw_strokes)
                .chain_weak()
                .after(EguiPostUpdateSet::EndPass)
                .after(TransformSystems::Propagate)
                .before(CameraUpdateSystems)
                // `sync_world` toggles `Visibility` to cull tiny shapes, so it
                // must land before Bevy turns that into view visibility.
                .before(VisibilitySystems::VisibilityPropagate)
                .run_if(in_state(Screen::Game)),
        )
        .add_systems(OnExit(Screen::Game), render::on_exit_game)
        .add_systems(EguiPrimaryContextPass, egui_frame);

    tighten_schedules(&mut app);
    app.run();
}

/// Development-only ECS hygiene.
///
/// `shuffle_seed` runs every schedule with the systems shuffled (constraints
/// still respected). A bug that only appears with a shuffled order means a
/// `before`/`after` constraint is missing — this is how the camera/visibility
/// ordering in `run` was found.
///
/// `NODE_EMPIRE_AUDIT=1` additionally turns on ambiguity reporting, which names
/// systems that touch the same data with no explicit order between them. It is
/// off by default because most of what it reports is Bevy's own internals
/// fighting over the asset pipeline.
#[cfg(debug_assertions)]
fn tighten_schedules(app: &mut App) {
    use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};

    let ambiguity_detection = if std::env::var_os("NODE_EMPIRE_AUDIT").is_some() {
        LogLevel::Warn
    } else {
        LogLevel::Ignore
    };

    macro_rules! tighten {
        ($($label:expr),* $(,)?) => {
            $(
                app.edit_schedule($label, |schedule| {
                    schedule.set_build_settings(ScheduleBuildSettings {
                        ambiguity_detection,
                        shuffle_seed: Some(0x4E4F_4445_5EED),
                        ..default()
                    });
                });
            )*
        };
    }

    tighten!(
        First,
        PreUpdate,
        Update,
        PostUpdate,
        Last,
        FixedUpdate,
        EguiPrimaryContextPass,
    );
}

#[cfg(not(debug_assertions))]
fn tighten_schedules(_app: &mut App) {}

fn window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: "Node Empire".into(),
            resolution: (1400u32, 900u32).into(),
            resize_constraints: WindowResizeConstraints {
                min_width: 960.0,
                min_height: 640.0,
                ..default()
            },
            ..default()
        }),
        ..default()
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            // Replaced every frame by `camera_follow`; the initial value only
            // has to be sane for the one frame before the first egui pass.
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 900.0 / 18.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Keep the Bevy camera in lockstep with the egui-space camera. The map viewport
/// is the area the HUD panel left over, so the world lines up with the labels
/// egui paints into that same rectangle.
fn camera_follow(
    cam: Res<CamView>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Ok((mut transform, mut projection)) = cameras.single_mut() else {
        return;
    };
    let (x, y, z) = cam.camera.bevy_translation(cam.viewport, cam.window);
    transform.translation = Vec3::new(x, y, z);
    if let Projection::Orthographic(ortho) = &mut *projection {
        ortho.scaling_mode = ScalingMode::FixedVertical {
            viewport_height: (cam.window.y / cam.camera.zoom).max(0.001),
        };
        ortho.scale = 1.0;
    }
}

// ---------------------------------------------------------------------------
// Simulation clock
// ---------------------------------------------------------------------------

/// Map the HUD speed (0 = pause, 1, 2, 4) onto Bevy's virtual clock, which in
/// turn drives the fixed 5 Hz simulation clock.
///
/// The pause menu pauses the clock as well as the simulation, so the world
/// (roads, weather, caravans) stops moving behind the overlay instead of
/// drifting on.
fn sync_speed(state: Res<GameState>, mut time: ResMut<Time<Virtual>>) {
    let speed = if state.show_menu { 0 } else { state.speed };
    if speed == 0 {
        if !time.is_paused() {
            time.pause();
        }
    } else {
        if time.is_paused() {
            time.unpause();
        }
        if (time.relative_speed() - speed as f32).abs() > f32::EPSILON {
            time.set_relative_speed(speed as f32);
        }
    }
}

/// One simulation tick. FixedUpdate already handles pause and fast-forward;
/// this only has to hold the game-layer rules (no ticks while the pause menu
/// is open, none after the game has ended).
fn sim_tick(mut state: ResMut<GameState>) {
    if state.speed == 0 || state.show_menu || state.world.victory.is_some() {
        return;
    }
    state.world.tick();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Node under a tile-space point. Radius aware, so a click near a big
/// settlement does not fall through to a farm behind it.
fn node_at(world: &World, p: EVec2) -> Option<u32> {
    let mut best: Option<(u32, f32)> = None;
    for n in world.nodes.values() {
        let d = (EVec2::new(n.x, n.y) - p).length();
        let r = n.radius().max(0.7);
        if d <= r && best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((n.id, d));
        }
    }
    best.map(|(id, _)| id)
}

fn toast(state: &mut GameState, msg: impl Into<String>, now: f64) {
    state.status = Some((msg.into(), now + 4.0));
}

fn open_menu(state: &mut GameState) {
    state.menu_resume_speed = if state.speed == 0 { 1 } else { state.speed };
    state.show_menu = true;
}

fn close_menu(state: &mut GameState) {
    state.show_menu = false;
    state.speed = state.menu_resume_speed;
}

/// A fresh world from the menu's faction / difficulty choices.
fn fresh_world(state: &GameState) -> World {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1337);
    World::new(seed, state.menu_state.faction, state.menu_state.difficulty)
}

/// Swap in a world (new game or loaded save), centre on the player's capital
/// and switch to the game screen.
fn enter_game(
    state: &mut GameState,
    world: World,
    cam: &mut CamView,
    next: &mut NextState<Screen>,
) {
    let focus = state.swap_world(world);
    cam.camera.center = match focus {
        Some((x, y)) => EVec2::new(x, y),
        None => EVec2::new(
            state.world.map.cols as f32 * 0.5,
            state.world.map.rows as f32 * 0.5,
        ),
    };
    cam.camera.zoom = 18.0;
    next.set(Screen::Game);
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/// Apply one command from the HUD. Commands are collected while drawing and
/// applied afterwards, which keeps "what happened" separate from "how it looks".
fn apply_action(
    state: &mut GameState,
    act: Action,
    now: f64,
    cam: &mut CamView,
    next: &mut NextState<Screen>,
) -> Result {
    match act {
        Action::SetTool(t) => state.tool = t,
        Action::SetTab(t) => state.right_tab = t,
        Action::Select(id) => state.selected = id,
        Action::Build(kind, x, y) => match state.world.place_building(kind, x, y, Owner::Player) {
            Ok(_) => toast(state, format!("Built a {}.", kind.label()), now),
            Err(e) => toast(state, e, now),
        },
        Action::Road(from, to) => {
            let cost = state.world.road_cost(from, to);
            match state.world.build_road(from, to, Owner::Player) {
                Ok(_) => toast(
                    state,
                    format!(
                        "Road built{}.",
                        cost.map(|c| format!(" ({}g)", c)).unwrap_or_default()
                    ),
                    now,
                ),
                Err(e) => toast(state, e, now),
            }
        }
        Action::CutRoad(idx) => {
            if let Err(e) = state.world.cut_road(idx, Owner::Player) {
                toast(state, e, now);
            }
        }
        Action::CaravanDelta(idx, d) => {
            if let Some(e) = state.world.edges.get_mut(idx) {
                e.caravans = (e.caravans + d).clamp(0, MAX_CARRAVANS);
            }
        }
        Action::TrainWorker(id, tier) => {
            if let Err(e) = state.world.train_worker(id, tier, Owner::Player) {
                toast(state, e, now);
            }
        }
        Action::DismissWorker(id, index) => {
            if let Err(e) = state.world.dismiss_worker(id, index, Owner::Player) {
                toast(state, e, now);
            }
        }
        Action::TrainSpy(target) => {
            let name = state
                .world
                .node(target)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            match state.world.train_spy(target, Owner::Player) {
                Ok(_) => toast(state, format!("An agent slips toward {}.", name), now),
                Err(e) => toast(state, e, now),
            }
        }
        Action::SetSpyOp(spy, op) => {
            if let Err(e) = state.world.set_spy_op(spy, op, Owner::Player) {
                toast(state, e, now);
            }
        }
        Action::RecallSpy(spy) => {
            if let Err(e) = state.world.recall_spy(spy, Owner::Player) {
                toast(state, e, now);
            }
        }
        Action::Foment(id) => match state.world.foment(id, Owner::Player) {
            Ok(_) => toast(state, "Whispers sown.", now),
            Err(e) => toast(state, e, now),
        },
        Action::BuySettlement(id) => {
            let name = state
                .world
                .node(id)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            match state.world.buy_settlement(id, Owner::Player) {
                Ok(_) => toast(state, format!("{} swears to you.", name), now),
                Err(e) => toast(state, e, now),
            }
        }
        Action::AdvanceAge => match state.world.advance_age(Owner::Player) {
            Ok(_) => {
                let age = state.world.player().age;
                toast(state, format!("The {} begins.", age.label()), now);
            }
            Err(e) => toast(state, e, now),
        },
        Action::Demolish(id) => match state.world.demolish(id, Owner::Player) {
            Ok(refund) => {
                if state.selected == Some(id) {
                    state.selected = None;
                }
                toast(state, format!("Demolished ({}g back).", refund), now);
            }
            Err(e) => toast(state, e, now),
        },
        Action::PayGeneralBonus(gid) => match state.world.pay_general_bonus(gid, Owner::Player) {
            Ok(_) => toast(
                state,
                "General bestowed with gold bonus (+15 loyalty).",
                now,
            ),
            Err(e) => toast(state, e, now),
        },
        Action::UpgradeFortEquipment(fid) => {
            match state.world.upgrade_fort_equipment(fid, Owner::Player) {
                Ok(_) => toast(state, "Fort armory upgraded to next equipment tier.", now),
                Err(e) => toast(state, e, now),
            }
        }
        Action::Save(slot) => {
            save::save(&state.world, slot).map_err(errors::engine_error)?;
            toast(state, format!("Saved to slot {}.", slot), now);
        }
        Action::Load(slot) => {
            let w = save::load(slot).map_err(errors::engine_error)?;
            enter_game(state, w, cam, next);
            toast(state, format!("Loaded slot {}.", slot), now);
        }
        Action::NewGame => {
            let w = fresh_world(state);
            enter_game(state, w, cam, next);
        }
        Action::QuitToMenu => {
            state.selected = None;
            state.status = None;
            state.show_menu = false;
            next.set(Screen::Menu);
        }
        Action::SetSpeed(s) => {
            state.speed = s;
            // Speed buttons also dismiss the pause menu if it is open.
            if s > 0 {
                state.show_menu = false;
            }
        }
        Action::TogglePause => {
            state.speed = if state.speed == 0 { 1 } else { 0 };
        }
        Action::ToggleHelp => state.show_help = !state.show_help,
        Action::ToggleMenu => {
            if state.show_menu {
                close_menu(state);
            } else {
                open_menu(state);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Hotkeys
// ---------------------------------------------------------------------------

fn hotkeys(state: &mut GameState, ctx: &egui::Context, in_game: bool) {
    if ctx.egui_wants_keyboard_input() {
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

    if !in_game {
        if esc && state.show_help {
            state.show_help = false;
        }
        if h {
            state.show_help = !state.show_help;
        }
        return;
    }

    if esc {
        if state.show_help {
            state.show_help = false;
        } else if state.show_menu {
            close_menu(state);
        } else if matches!(state.tool, Tool::Build(_) | Tool::Road { .. }) {
            state.tool = Tool::Select;
        } else {
            open_menu(state);
        }
    }
    if state.show_menu {
        // Only Esc works while the pause menu is open.
        return;
    }
    if h {
        state.show_help = !state.show_help;
    }
    if v {
        state.tool = Tool::Select;
    }
    if r {
        state.tool = Tool::Road { from: None };
    }
    if space {
        state.speed = if state.speed == 0 { 1 } else { 0 };
    }
    if s1 {
        state.speed = 1;
    }
    if s2 {
        state.speed = 2;
    }
    if s3 {
        state.speed = 4;
    }
    if n0 {
        let now = state.now;
        match save::save(&state.world, 1) {
            Ok(_) => toast(state, "Quick-saved to slot 1.", now),
            Err(e) => toast(state, e, now),
        }
    }
}

// ---------------------------------------------------------------------------
// Map interaction
// ---------------------------------------------------------------------------

/// The map area: panning, zooming, hover/selection, tool clicks — then the
/// world text layer painted straight on top of the Bevy-drawn shapes.
fn map_panel(
    ui: &mut Ui,
    ctx: &egui::Context,
    state: &mut GameState,
    cam: &mut CamView,
) -> Vec<Action> {
    let mut actions = Vec::new();
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let mut camera = cam.camera;
    let hover_pos = response.hover_pos();

    // Pan: middle drag anywhere; primary drag on empty ground with the Select
    // tool armed (so Build/Road drags never move the camera).
    let primary_drag = response.dragged_by(PointerButton::Primary);
    let middle_drag = response.dragged_by(PointerButton::Middle);
    if middle_drag || primary_drag {
        let started_on_node = response
            .interact_pointer_pos()
            .map(|p| node_at(&state.world, camera.screen_to_world(rect, p)).is_some())
            .unwrap_or(false);
        if middle_drag || (primary_drag && !started_on_node && state.tool == Tool::Select) {
            camera.pan_px(response.drag_delta());
            camera.clamp_center(state.world.map.cols, state.world.map.rows);
        }
    }

    // Zoom: wheel / pinch, anchored at the cursor.
    if let Some(p) = hover_pos {
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        let pinch = ctx.input(|i| i.zoom_delta());
        let factor = pinch * (1.0 + scroll * 0.0015);
        if (factor - 1.0).abs() > 0.001 {
            camera.zoom_at(rect, p, factor);
            camera.clamp_center(state.world.map.cols, state.world.map.rows);
        }
    }

    state.hover = hover_pos.and_then(|p| node_at(&state.world, camera.screen_to_world(rect, p)));

    // Keyboard pan (WASD / arrows), unless typing somewhere.
    if !ctx.egui_wants_keyboard_input() {
        let dt = ctx.input(|i| i.stable_dt);
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
            let v = EVec2::new(dx, dy).normalized() * (600.0 * dt) / camera.zoom;
            camera.center += v;
            camera.clamp_center(state.world.map.cols, state.world.map.rows);
        }
    }

    // Right-click cancels the tool / deselects.
    if response.secondary_clicked() {
        match state.tool {
            Tool::Select => state.selected = None,
            _ => state.tool = Tool::Select,
        }
    }

    // Left click: tool action.
    if response.clicked() {
        if let Some(p) = response.interact_pointer_pos().or(response.hover_pos()) {
            match state.tool {
                Tool::Select => {
                    state.selected = node_at(&state.world, camera.screen_to_world(rect, p));
                    if state.selected.is_some() {
                        actions.push(Action::SetTab(RightTab::Node));
                    }
                }
                Tool::Build(kind) => {
                    let w = camera.screen_to_world(rect, p);
                    actions.push(Action::Build(kind, w.x, w.y));
                }
                Tool::Road { from } => {
                    if let Some(id) = node_at(&state.world, camera.screen_to_world(rect, p)) {
                        match from {
                            None => state.tool = Tool::Road { from: Some(id) },
                            Some(a) if a != id => {
                                actions.push(Action::Road(a, id));
                                state.tool = Tool::Road { from: None };
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let cursor = hover_pos.map(|p| camera.screen_to_world(rect, p));
    cam.camera = camera;
    cam.viewport = rect;
    cam.window = ctx.viewport_rect().size();
    cam.pointer = hover_pos;
    cam.cursor = cursor;

    // World text layer, painted in screen space over the Bevy shapes.
    let now = state.now;
    let toast_msg = state
        .status
        .as_ref()
        .filter(|(_, exp)| now <= *exp)
        .map(|(m, exp)| (m.as_str(), (*exp - now).clamp(0.0, 1.0) as f32));
    let painter = ui.painter().with_clip_rect(rect);
    ui::map_labels(
        &painter,
        rect,
        &state.world,
        &camera,
        state.tool,
        cursor,
        state.hover,
        state.selected,
        toast_msg,
    );

    actions
}

// ---------------------------------------------------------------------------
// The egui pass
// ---------------------------------------------------------------------------

/// A background-layer root `Ui` the panels are carved out of.
fn root_ui(ctx: &egui::Context) -> Ui {
    Ui::new(
        ctx.clone(),
        "node_empire_root".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    )
}

#[allow(clippy::too_many_arguments)]
fn egui_frame(
    mut contexts: EguiContexts,
    mut state: ResMut<GameState>,
    mut cam: ResMut<CamView>,
    mut next: ResMut<NextState<Screen>>,
    screen: Res<State<Screen>>,
    mut exit: MessageWriter<AppExit>,
    mut styled: Local<bool>,
) -> Result {
    let ctx: &egui::Context = contexts.ctx_mut()?;
    if !*styled {
        theme::set_style(ctx);
        *styled = true;
    }

    let now = ctx.input(|i| i.time);
    state.now = now;
    if let Some((_, exp)) = &state.status {
        if now > *exp {
            state.status = None;
        }
    }

    // Anything Bevy's error handler caught this frame, plus anything queued by
    // hand, becomes a toast.
    while let Some(msg) = errors::take() {
        toast(&mut state, msg, now);
    }

    let in_game = matches!(*screen.get(), Screen::Game);
    hotkeys(&mut state, ctx, in_game);

    let mut actions: Vec<Action> = Vec::new();

    if in_game {
        let mut root = root_ui(ctx);

        egui::Panel::top("top_bar").show(&mut root, |ui| {
            let h = Hud {
                world: &state.world,
                tool: state.tool,
                tab: state.right_tab,
                selected: state.selected,
                speed: state.speed,
            };
            actions.extend(ui::top_bar(ui, &h));
        });

        egui::Panel::bottom("log_panel")
            .exact_size(96.0)
            .show(&mut root, |ui| ui::log_panel(ui, &state.world));

        egui::Panel::left("tools")
            .resizable(true)
            .default_size(238.0)
            .min_size(206.0)
            .max_size(320.0)
            .show(&mut root, |ui| {
                let h = Hud {
                    world: &state.world,
                    tool: state.tool,
                    tab: state.right_tab,
                    selected: state.selected,
                    speed: state.speed,
                };
                actions.extend(ui::left_panel(ui, &h));
            });

        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(330.0)
            .min_size(260.0)
            .max_size(430.0)
            .show(&mut root, |ui| {
                let h = Hud {
                    world: &state.world,
                    tool: state.tool,
                    tab: state.right_tab,
                    selected: state.selected,
                    speed: state.speed,
                };
                actions.extend(ui::right_panel(ui, &h));
            });

        egui::CentralPanel::no_frame().show(&mut root, |ui| {
            actions.extend(map_panel(ui, ctx, &mut state, &mut cam));
        });

        ui::help_window(ctx, &mut state.show_help);

        if state.world.victory.is_some() {
            actions.extend(ui::game_over(ctx, &state.world));
        } else if state.show_menu {
            actions.extend(ui::pause_menu(ctx, &state.world));
        }

        for a in actions {
            apply_action(&mut state, a, now, &mut cam, &mut next)?;
        }
    } else {
        let mut root = root_ui(ctx);
        let mut cmd = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(16, 18, 24)))
            .show(&mut root, |ui| {
                cmd = ui::main_menu(ui, &mut state.menu_state);
            });

        ui::help_window(ctx, &mut state.show_help);

        // Keyboard shortcuts for the menu, so it can be driven without a mouse.
        if ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space)) {
            cmd = Some(MenuCmd::Start);
        }
        if ctx.input(|i| i.key_pressed(Key::H)) {
            cmd = Some(MenuCmd::Help);
        }
        for (i, def) in FACTIONS.iter().enumerate() {
            let key = [
                Key::Num1,
                Key::Num2,
                Key::Num3,
                Key::Num4,
                Key::Num5,
                Key::Num6,
            ][i];
            if ctx.input(|inp| inp.key_pressed(key)) {
                cmd = Some(MenuCmd::SetFaction(def.id));
            }
        }

        match cmd {
            Some(MenuCmd::Start) => {
                let w = fresh_world(&state);
                enter_game(&mut state, w, &mut cam, &mut next);
            }
            Some(MenuCmd::Continue) => {
                // Most recent existing slot; prefer the autosave.
                let slot = [0usize, 1, 2, 3]
                    .into_iter()
                    .find(|s| save::exists(*s))
                    .unwrap_or(0);
                let w = save::load(slot).map_err(errors::engine_error)?;
                enter_game(&mut state, w, &mut cam, &mut next);
            }
            Some(MenuCmd::Quit) => {
                exit.write(AppExit::Success);
            }
            Some(MenuCmd::Help) => state.show_help = true,
            Some(MenuCmd::SetFaction(f)) => state.menu_state.faction = f,
            Some(MenuCmd::SetDifficulty(d)) => state.menu_state.difficulty = d,
            None => {}
        }
    }

    Ok(())
}
