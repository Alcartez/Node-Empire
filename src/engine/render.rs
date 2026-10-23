//! Bevy world rendering: the terrain texture, filled shapes as `Mesh2d`
//! entities, and every outline / ring / preview as immediate-mode gizmos.
//!
//! Text stays in egui (`ui::map_labels`), so this module owns pixels only.
//! Colours are exposed as helpers because the HUD paints matching chips.
//!
//! Coordinate systems: the game works in *tile* units with Y pointing down
//! (egui's convention). Bevy is Y-up, so every world position is mirrored
//! through `wpos()` when it is handed to the renderer.

use bevy::asset::RenderAssetUsages;
use bevy::color::Alpha;
use bevy::image::ImageSampler;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite_render::AlphaMode2d;

use super::state::{CamView, GameState, Mats, Screen, Tool, WorldVis};
use super::terrain;
use crate::game::types::*;
use crate::game::world::World;

// ---------------------------------------------------------------------------
// Layers (z within the 2D transparent phase, back to front)
// ---------------------------------------------------------------------------

const Z_TERRAIN: f32 = 0.0;
const Z_SPECK: f32 = 1.0;
const Z_ROAD: f32 = 1.5;
const Z_NODE: f32 = 2.0;
const Z_DOT: f32 = 3.0;
const Z_GHOST: f32 = 4.0;

/// Minimum on-screen radius, in pixels, for things drawn as filled shapes.
const MIN_NODE_PX: f32 = 4.0;
const GHOST_PX: f32 = 6.0;

/// Node kinds in the order `kind_index` uses.
pub const KINDS: [NodeType; 9] = [
    NodeType::Settlement,
    NodeType::Farm,
    NodeType::Mine,
    NodeType::Woodcamp,
    NodeType::Factory,
    NodeType::Market,
    NodeType::Fort,
    NodeType::Laboratory,
    NodeType::BanditCamp,
];

/// Stable small index for a node kind (used to key meshes and materials).
pub fn kind_index(kind: NodeType) -> usize {
    match kind {
        NodeType::Settlement => 0,
        NodeType::Farm => 1,
        NodeType::Mine => 2,
        NodeType::Woodcamp => 3,
        NodeType::Factory => 4,
        NodeType::Market => 5,
        NodeType::Fort => 6,
        NodeType::Laboratory => 7,
        NodeType::BanditCamp => 8,
    }
}

/// Stable small index for an owner.
pub fn owner_index(owner: Owner) -> u8 {
    match owner {
        Owner::Player => 0,
        Owner::Enemy => 1,
        Owner::Neutral => 2,
        Owner::Bandit => 3,
    }
}

/// Extra material keys in `Mats::dots`.
const DOT_CARAVAN: u8 = 9;
const DOT_COPPER: u8 = 10;
const DOT_IRON: u8 = 11;

// ---------------------------------------------------------------------------
// Coordinates
// ---------------------------------------------------------------------------

/// Tile-space (Y down) to Bevy world space (Y up).
#[inline]
pub fn wpos(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, -y)
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

pub fn rgb(c: [u8; 3]) -> Color {
    Color::srgb_u8(c[0], c[1], c[2])
}

pub fn rgba(c: [u8; 3], a: u8) -> Color {
    Color::srgba_u8(c[0], c[1], c[2], a)
}

pub const MAP_BG: [u8; 3] = [18, 20, 24];

pub fn owner_color(world: &World, owner: Owner) -> Color {
    match owner {
        Owner::Player => rgb(faction_def(world.player_faction).color),
        Owner::Enemy => rgb(faction_def(world.enemy_faction).color),
        Owner::Neutral => rgb([175, 175, 165]),
        Owner::Bandit => rgb([145, 62, 52]),
    }
}

pub fn kind_color(kind: NodeType) -> Color {
    rgb(match kind {
        NodeType::Settlement => [220, 210, 190],
        NodeType::Farm => [122, 192, 92],
        NodeType::Mine => [192, 142, 92],
        NodeType::Woodcamp => [78, 142, 82],
        NodeType::Factory => [132, 152, 182],
        NodeType::Market => [232, 192, 82],
        NodeType::Fort => [172, 122, 112],
        NodeType::Laboratory => [162, 122, 212],
        NodeType::BanditCamp => [152, 72, 62],
    })
}

pub fn edge_color(kind: EdgeType) -> Color {
    rgb(match kind {
        EdgeType::Resource => [208, 178, 118],
        EdgeType::Supply => [112, 192, 132],
        EdgeType::Combat => [222, 82, 72],
        EdgeType::Siege => [238, 152, 58],
        EdgeType::Trade => [238, 218, 108],
    })
}

pub fn edge_index(kind: EdgeType) -> u8 {
    match kind {
        EdgeType::Resource => 0,
        EdgeType::Supply => 1,
        EdgeType::Combat => 2,
        EdgeType::Siege => 3,
        EdgeType::Trade => 4,
    }
}

fn deposit_color(d: Deposit) -> Option<([u8; 3], u8)> {
    match d {
        Deposit::None => None,
        Deposit::Copper => Some(([214, 127, 50], DOT_COPPER)),
        Deposit::Iron => Some(([176, 176, 192], DOT_IRON)),
    }
}

/// The colour a node's body is filled with.
fn node_fill(world: &World, n: &crate::game::world::Node) -> Color {
    if n.kind == NodeType::Settlement {
        owner_color(world, n.owner)
    } else {
        kind_color(n.kind)
    }
}

// ---------------------------------------------------------------------------
// Meshes
// ---------------------------------------------------------------------------

/// A unit-radius regular polygon (fan), rotated by `rot` radians.
fn poly_mesh(sides: usize, rot: f32) -> Mesh {
    let mut positions = Vec::with_capacity(sides + 1);
    let mut uvs = Vec::with_capacity(sides + 1);
    let mut indices = Vec::with_capacity(sides * 3);

    positions.push([0.0, 0.0, 0.0]);
    uvs.push([0.5, 0.5]);

    for i in 0..sides {
        let a = rot + i as f32 * std::f32::consts::TAU / sides as f32;
        let (s, c) = a.sin_cos();
        positions.push([c, s, 0.0]);
        uvs.push([0.5 + 0.5 * c, 0.5 + 0.5 * s]);
    }
    for i in 0..sides {
        let a = 1 + i as u32;
        let b = 1 + ((i + 1) % sides) as u32;
        indices.extend_from_slice(&[0, a, b]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Polygon definition for a node kind: `Some((sides, rotation))` or `None` for
/// a circle. The rotation is already mirrored into Bevy space.
fn kind_shape(kind: NodeType) -> Option<(usize, f32)> {
    let p = std::f32::consts::FRAC_PI_2;
    match kind {
        NodeType::Settlement | NodeType::Market | NodeType::Laboratory => None,
        NodeType::Farm => Some((3, p)),
        NodeType::Mine => Some((4, 0.0)),
        NodeType::Woodcamp => Some((4, -p / 2.0)),
        NodeType::Factory => Some((6, 0.0)),
        NodeType::Fort => Some((5, p)),
        NodeType::BanditCamp => Some((3, -p)),
    }
}

/// The outline of a node kind at `center`/`radius`, for gizmo strokes.
fn kind_outline(kind: NodeType, center: Vec2, r: f32, out: &mut Vec<Vec2>) {
    out.clear();
    if let Some((sides, rot)) = kind_shape(kind) {
        for i in 0..sides {
            let a = rot + i as f32 * std::f32::consts::TAU / sides as f32;
            let (s, c) = a.sin_cos();
            out.push(center + Vec2::new(c, s) * r);
        }
    }
}

/// A road ribbon: a triangle strip with a constant world-space half-width.
fn ribbon_mesh(path: &[(i32, i32)], half: f32) -> Mesh {
    let pts: Vec<Vec2> = path
        .iter()
        .map(|&(x, y)| Vec2::new(x as f32 + 0.5, y as f32 + 0.5))
        .collect();
    let n = pts.len();

    let mut positions = Vec::with_capacity(n * 2);
    let mut uvs = Vec::with_capacity(n * 2);
    for i in 0..n {
        let p = pts[i];
        let prev = if i == 0 { p } else { pts[i - 1] };
        let next = if i + 1 >= n { p } else { pts[i + 1] };
        let dir = (next - prev).normalize_or_zero();
        let off = Vec2::new(-dir.y, dir.x) * half;
        let a = wpos(p.x + off.x, p.y + off.y);
        let b = wpos(p.x - off.x, p.y - off.y);
        positions.push([a.x, a.y, 0.0]);
        positions.push([b.x, b.y, 0.0]);
        uvs.push([0.0, 0.0]);
        uvs.push([1.0, 1.0]);
    }

    let mut indices = Vec::new();
    for i in 0..n.saturating_sub(1) {
        let a = (i * 2) as u32;
        indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n * 2])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

// ---------------------------------------------------------------------------
// Terrain texture
// ---------------------------------------------------------------------------

fn terrain_rgba(t: Terrain, shade: f32) -> [u8; 4] {
    let base = match t {
        Terrain::Water => [40, 75, 130],
        Terrain::Sand => [214, 196, 140],
        Terrain::Grass => [110, 150, 90],
        Terrain::Forest => [66, 110, 66],
        Terrain::Mountain => [128, 122, 118],
    };
    let s = shade.clamp(0.5, 1.5);
    let f = |v: u8| (v as f32 * s).clamp(0.0, 255.0) as u8;
    [f(base[0]), f(base[1]), f(base[2]), 255]
}

/// Bake the whole map into one small texture (1 texel per tile, nearest
/// filtered) so the terrain costs a single quad to draw.
fn bake_terrain(map: &crate::game::map::Map) -> Image {
    let cols = map.cols.max(1) as usize;
    let rows = map.rows.max(1) as usize;
    let mut data = Vec::with_capacity(cols * rows * 4);
    for y in 0..map.rows {
        for x in 0..map.cols {
            data.extend_from_slice(&terrain_rgba(
                map.terrain_at(x, y),
                map.shade[map.idx(x, y)],
            ));
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: cols as u32,
            height: rows as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

// ---------------------------------------------------------------------------
// Material cache
// ---------------------------------------------------------------------------

impl Mats {
    /// Opaque-ish flat colour, forced into the blended phase so the 2D
    /// transparent pass sorts it by `Transform::z`.
    fn flat(assets: &mut Assets<ColorMaterial>, color: Color) -> Handle<ColorMaterial> {
        assets.add(ColorMaterial {
            color,
            alpha_mode: AlphaMode2d::Blend,
            ..default()
        })
    }

    fn fill(
        &mut self,
        assets: &mut Assets<ColorMaterial>,
        key: (u8, u8),
        color: Color,
    ) -> Handle<ColorMaterial> {
        let a = assets;
        self.fills
            .entry(key)
            .or_insert_with(|| Self::flat(a, color))
            .clone()
    }

    fn edge(
        &mut self,
        assets: &mut Assets<ColorMaterial>,
        kind: EdgeType,
    ) -> Handle<ColorMaterial> {
        let key = edge_index(kind);
        self.roads_mut(assets, key, edge_color(kind))
    }

    fn roads_mut(
        &mut self,
        assets: &mut Assets<ColorMaterial>,
        key: u8,
        color: Color,
    ) -> Handle<ColorMaterial> {
        let a = assets;
        self.edges
            .entry(key)
            .or_insert_with(|| Self::flat(a, color))
            .clone()
    }

    fn dot(
        &mut self,
        assets: &mut Assets<ColorMaterial>,
        key: u8,
        color: Color,
    ) -> Handle<ColorMaterial> {
        let a = assets;
        self.dots
            .entry(key)
            .or_insert_with(|| Self::flat(a, color))
            .clone()
    }

    fn ghost(&mut self, assets: &mut Assets<ColorMaterial>, ok: bool) -> Handle<ColorMaterial> {
        let slot = if ok {
            &mut self.ghost_ok
        } else {
            &mut self.ghost_bad
        };
        let a = assets;
        slot.get_or_insert_with(|| {
            Self::flat(
                a,
                if ok {
                    rgba([140, 255, 160], 150)
                } else {
                    rgba([255, 110, 100], 150)
                },
            )
        })
        .clone()
    }
}

/// Owner colours are cached under the first four dot keys.
fn dot_key_for_owner(owner: Owner) -> u8 {
    owner_index(owner)
}

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

/// Build the shared unit meshes (one per node kind) and constant gizmo styles.
pub fn setup_shapes(mut meshes: ResMut<Assets<Mesh>>, mut vis: ResMut<WorldVis>) {
    let circle = meshes.add(poly_mesh(28, 0.0));
    vis.shapes.circle = circle.clone();
    for (i, kind) in KINDS.iter().enumerate() {
        vis.shapes.kinds[i] = match kind_shape(*kind) {
            Some((sides, rot)) => meshes.add(poly_mesh(sides, rot)),
            None => circle.clone(),
        };
    }
}

/// Constant-width strokes: one global pixel width for every map outline.
pub fn setup_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 1.8;
    config.line.joints = GizmoLineJoint::Round(4);
}

// ---------------------------------------------------------------------------
// Sync: world state -> entities
// ---------------------------------------------------------------------------

/// Rebuild everything when the map changes, then keep the per-frame visuals
/// (nodes, specks, spies, caravans, ghost) in step with the world.
#[allow(clippy::too_many_arguments)]
pub fn sync_world(
    state: Res<GameState>,
    cam: Res<CamView>,
    time: Res<Time>,
    mut vis: ResMut<WorldVis>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut terrain_mats: ResMut<Assets<terrain::TerrainMaterial>>,
    mut transforms: Query<&mut Transform>,
    mut mesh_query: Query<&mut Mesh2d>,
    mut mat_query: Query<&mut MeshMaterial2d<ColorMaterial>>,
    mut vis_query: Query<&mut Visibility>,
) {
    let world = &state.world;
    let zoom = cam.camera.zoom;
    let vis = &mut *vis;

    // ---- rebuild on new game / load ------------------------------------
    if vis.map_gen != state.map_gen {
        // Bulk despawn: one command drops every map visual, whether or not it
        // is still tracked below (they all carry `DespawnOnExit(Game)`). New
        // spawns are queued after this command, so they survive.
        commands.despawn_all::<With<DespawnOnExit<Screen>>>();
        vis.terrain = None;
        vis.specks.clear();
        vis.nodes.clear();
        vis.spies.clear();
        vis.caravans.clear();
        vis.ghost = None;
        vis.roads.clear();
        vis.road_sig = 0;
        vis.map_gen = state.map_gen;

        let map = &world.map;
        let cols = map.cols as f32;
        let rows = map.rows as f32;

        let image = images.add(bake_terrain(map));
        let material = terrain_mats.add(terrain::TerrainMaterial {
            base: ColorMaterial {
                color: Color::WHITE,
                alpha_mode: AlphaMode2d::Opaque,
                texture: Some(image),
                ..default()
            },
            extension: terrain::WeatherExtension {
                params: terrain::Weather::for_map(cols, rows),
            },
        });
        vis.mats.terrain_ext = Some(material.clone());
        vis.terrain = Some(
            commands
                .spawn((
                    Mesh2d(meshes.add(Rectangle::new(cols, rows))),
                    MeshMaterial2d(material),
                    Transform::from_xyz(cols * 0.5, -rows * 0.5, Z_TERRAIN),
                    DespawnOnExit(Screen::Game),
                ))
                .id(),
        );

        // Static mineral deposits, hidden until the zoom makes them useful.
        let speck = vis.shapes.circle.clone();
        for y in 0..map.rows {
            for x in 0..map.cols {
                let Some((color, key)) = deposit_color(map.deposit_at(x, y)) else {
                    continue;
                };
                let mat = vis.mats.dot(&mut materials, key, rgb(color));
                let e = commands
                    .spawn((
                        Mesh2d(speck.clone()),
                        MeshMaterial2d(mat),
                        Transform::from_xyz(x as f32 + 0.5, -(y as f32 + 0.5), Z_SPECK)
                            .with_scale(Vec3::splat(0.02)),
                        DespawnOnExit(Screen::Game),
                    ))
                    .id();
                vis.specks.push(e);
            }
        }
    }

    // ---- terrain weather: the clock drifts the shimmer ----------------
    // `Time` is the virtual clock, so the effect freezes with the pause menu
    // and speeds up with the fast-forward buttons for free.
    if let Some(handle) = &vis.mats.terrain_ext {
        if let Some(mut terrain_mat) = terrain_mats.get_mut(handle) {
            terrain_mat.extension.params.data.x = time.elapsed_secs();
        }
    }

    // ---- deposits: scale + visibility track the zoom -------------------
    let speck_r = (zoom * 0.14).max(1.5) / zoom;
    let specks_visible = zoom >= 9.0;
    for &e in &vis.specks {
        if let Ok(mut t) = transforms.get_mut(e) {
            t.scale = Vec3::splat(speck_r);
        }
        if let Ok(mut v) = vis_query.get_mut(e) {
            *v = if specks_visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    // ---- nodes ---------------------------------------------------------
    for n in world.nodes.values() {
        let r = (n.radius() * 0.5).max(MIN_NODE_PX / zoom);
        let pos = wpos(n.x, n.y);
        let key = (kind_index(n.kind) as u8, owner_index(n.owner));
        let color = node_fill(world, n);
        let mat = vis.mats.fill(&mut materials, key, color);

        match vis.nodes.get(&n.id).copied() {
            Some(e) => {
                if let Ok(mut t) = transforms.get_mut(e) {
                    t.translation = pos.extend(Z_NODE);
                    t.scale = Vec3::splat(r);
                }
                if let Ok(mut m) = mat_query.get_mut(e) {
                    if m.0.id() != mat.id() {
                        m.0 = mat;
                    }
                }
            }
            None => {
                let mesh = vis.shapes.kinds[kind_index(n.kind)].clone();
                let e = commands
                    .spawn((
                        Mesh2d(mesh),
                        MeshMaterial2d(mat),
                        Transform::from_xyz(pos.x, pos.y, Z_NODE).with_scale(Vec3::splat(r)),
                        DespawnOnExit(Screen::Game),
                    ))
                    .id();
                vis.nodes.insert(n.id, e);
            }
        }
    }
    // Nodes are never destroyed by the simulation, but stay honest anyway.
    let stale: Vec<u32> = vis
        .nodes
        .keys()
        .copied()
        .filter(|id| !world.nodes.contains_key(id))
        .collect();
    for id in stale {
        if let Some(e) = vis.nodes.remove(&id) {
            commands.entity(e).despawn();
        }
    }

    // ---- spies (pooled) ------------------------------------------------
    while vis.spies.len() < world.spies.len() {
        let e = commands
            .spawn((
                Mesh2d(vis.shapes.circle.clone()),
                MeshMaterial2d(Mats::flat(&mut materials, Color::WHITE)),
                Transform::from_xyz(0.0, 0.0, Z_DOT).with_scale(Vec3::splat(1.0)),
                Visibility::Hidden,
                DespawnOnExit(Screen::Game),
            ))
            .id();
        vis.spies.push(e);
    }
    for (i, s) in world.spies.iter().enumerate() {
        let e = vis.spies[i];
        let embedded = s.status == SpyStatus::Embedded;
        let r_px = if embedded { 3.0 } else { 4.0 };
        let color = owner_color(world, s.owner);
        let mat = vis
            .mats
            .dot(&mut materials, dot_key_for_owner(s.owner), color);
        match spy_pos(world, s) {
            Some(p) => {
                let pos = wpos(p.x, p.y);
                if let Ok(mut t) = transforms.get_mut(e) {
                    t.translation = pos.extend(Z_DOT);
                    t.scale = Vec3::splat(r_px / zoom);
                }
                if let Ok(mut m) = mat_query.get_mut(e) {
                    if m.0.id() != mat.id() {
                        m.0 = mat;
                    }
                }
                if let Ok(mut v) = vis_query.get_mut(e) {
                    *v = Visibility::Visible;
                }
            }
            None => {
                if let Ok(mut v) = vis_query.get_mut(e) {
                    *v = Visibility::Hidden;
                }
            }
        }
    }
    for &e in vis.spies.iter().skip(world.spies.len()) {
        if let Ok(mut v) = vis_query.get_mut(e) {
            *v = Visibility::Hidden;
        }
    }

    // ---- caravans (pooled dots, one per parallel caravan) --------------
    let caravan_mat = vis
        .mats
        .dot(&mut materials, DOT_CARAVAN, rgb([250, 245, 225]));
    let mut used = 0usize;
    for e in &world.edges {
        let carry = matches!(e.kind, EdgeType::Resource | EdgeType::Trade);
        if !carry || e.caravans <= 0 || e.path.len() < 2 {
            continue;
        }
        let period = (e.length() * TILE_TRAVEL_TICKS / e.caravans as f32).max(2.0);
        let base = (e.progress / period).clamp(0.0, 1.0);
        for k in 0..e.caravans as usize {
            let Some(p) = along_path(&e.path, base + k as f32 / e.caravans as f32) else {
                continue;
            };
            if used >= vis.caravans.len() {
                let e = commands
                    .spawn((
                        Mesh2d(vis.shapes.circle.clone()),
                        MeshMaterial2d(caravan_mat.clone()),
                        Transform::from_xyz(0.0, 0.0, Z_DOT),
                        DespawnOnExit(Screen::Game),
                    ))
                    .id();
                vis.caravans.push(e);
            }
            let pe = vis.caravans[used];
            if let Ok(mut t) = transforms.get_mut(pe) {
                let pos = wpos(p.x, p.y);
                t.translation = pos.extend(Z_DOT);
                t.scale = Vec3::splat((3.0 / zoom).max(0.04));
            }
            if let Ok(mut v) = vis_query.get_mut(pe) {
                *v = Visibility::Visible;
            }
            used += 1;
        }
    }
    for &e in vis.caravans.iter().skip(used) {
        if let Ok(mut v) = vis_query.get_mut(e) {
            *v = Visibility::Hidden;
        }
    }

    // ---- build ghost ---------------------------------------------------
    let ghost_kind = match state.tool {
        Tool::Build(kind) => Some(kind),
        _ => None,
    };
    match (ghost_kind, cam.cursor) {
        (Some(kind), Some(cursor)) => {
            let tx = cursor.x.floor() + 0.5;
            let ty = cursor.y.floor() + 0.5;
            let ok = world.can_place(kind, tx, ty, Owner::Player).is_ok();
            let r = (if kind == NodeType::Settlement {
                zoom * 0.7
            } else {
                zoom * 0.5
            }
            .max(GHOST_PX))
                / zoom;
            let mesh = vis.shapes.kinds[kind_index(kind)].clone();
            let mat = vis.mats.ghost(&mut materials, ok);
            let pos = wpos(tx, ty);

            let e = vis.ghost.unwrap_or_else(|| {
                let e = commands
                    .spawn((
                        Mesh2d(mesh.clone()),
                        MeshMaterial2d(mat.clone()),
                        Transform::from_xyz(pos.x, pos.y, Z_GHOST),
                        DespawnOnExit(Screen::Game),
                    ))
                    .id();
                vis.ghost = Some(e);
                e
            });
            if let Ok(mut t) = transforms.get_mut(e) {
                t.translation = pos.extend(Z_GHOST);
                t.scale = Vec3::splat(r);
            }
            if let Ok(mut m) = mesh_query.get_mut(e) {
                m.0 = mesh;
            }
            if let Ok(mut m) = mat_query.get_mut(e) {
                if m.0.id() != mat.id() {
                    m.0 = mat;
                }
            }
            if let Ok(mut v) = vis_query.get_mut(e) {
                *v = Visibility::Visible;
            }
        }
        _ => {
            if let Some(e) = vis.ghost {
                if let Ok(mut v) = vis_query.get_mut(e) {
                    *v = Visibility::Hidden;
                }
            }
        }
    }

    // ---- roads (ribbons, rebuilt only when the set changes) ------------
    let sig = road_signature(world, zoom);
    if sig != vis.road_sig {
        vis.road_sig = sig;
        for e in vis.roads.drain(..) {
            commands.entity(e).despawn();
        }
        for e in &world.edges {
            if e.path.len() < 2 {
                continue;
            }
            let carry = matches!(e.kind, EdgeType::Resource | EdgeType::Trade);
            let px = if carry {
                1.6 + e.caravans.max(0) as f32 * 0.45
            } else {
                2.2
            };
            let mat = vis.mats.edge(&mut materials, e.kind);
            let mesh = meshes.add(ribbon_mesh(&e.path, 0.5 * px / zoom));
            let ent = commands
                .spawn((
                    Mesh2d(mesh),
                    MeshMaterial2d(mat),
                    Transform::from_xyz(0.0, 0.0, Z_ROAD),
                    DespawnOnExit(Screen::Game),
                ))
                .id();
            vis.roads.push(ent);
        }
    }
}

/// Position of a spy between its home and target node.
pub fn spy_pos(world: &World, s: &crate::game::types::Spy) -> Option<bevy::math::Vec2> {
    let h = world.node(s.home)?;
    let t = world.node(s.target)?;
    let f = s.progress.clamp(0.0, 1.0);
    Some(match s.status {
        SpyStatus::Embedded => bevy::math::Vec2::new(t.x, t.y),
        SpyStatus::Traveling => bevy::math::Vec2::new(h.x + (t.x - h.x) * f, h.y + (t.y - h.y) * f),
        SpyStatus::Returning => bevy::math::Vec2::new(t.x + (h.x - t.x) * f, t.y + (h.y - t.y) * f),
    })
}

/// A point `frac` (wrapping) along a tile path, taking a shorter path first.
fn along_path(path: &[(i32, i32)], frac: f32) -> Option<bevy::math::Vec2> {
    if path.len() < 2 {
        return None;
    }
    let frac = frac.rem_euclid(1.0);
    let spans = (path.len() - 1) as f32;
    let seg = (frac * spans).min(spans - f32::EPSILON);
    let i = seg.floor() as usize;
    let t = seg - i as f32;
    let (ax, ay) = path[i];
    let (bx, by) = path[i + 1];
    Some(bevy::math::Vec2::new(
        ax as f32 + 0.5 + (bx - ax) as f32 * t,
        ay as f32 + 0.5 + (by - ay) as f32 * t,
    ))
}

/// Cheap hash of everything that changes road geometry.
fn road_signature(world: &World, zoom: f32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x100_0000_01b3);
    };
    mix((zoom * 4.0) as u64);
    for e in &world.edges {
        mix(e.id as u64 * 31 + e.path.len() as u64);
        mix(e.caravans.max(0) as u64 * 7 + edge_index(e.kind) as u64);
    }
    h
}

// ---------------------------------------------------------------------------
// Sync: strokes (gizmos)
// ---------------------------------------------------------------------------

/// Every outline, ring, spy thread and build/road preview, drawn as vector
/// lines on top of the filled shapes.
pub fn draw_strokes(mut gizmos: Gizmos, state: Res<GameState>, cam: Res<CamView>) {
    let world = &state.world;
    let zoom = cam.camera.zoom;
    let px = 1.0 / zoom; // one pixel in world units

    // Faint spy threads, home -> target.
    for s in &world.spies {
        if let (Some(h), Some(t)) = (world.node(s.home), world.node(s.target)) {
            let c = owner_color(world, s.owner);
            gizmos.line_2d(wpos(h.x, h.y), wpos(t.x, t.y), c.with_alpha(0.27));
        }
    }

    // Roads being drawn.
    if let Tool::Road { from } = state.tool {
        if let (Some(from_id), Some(cursor)) = (from, cam.cursor) {
            if let Some(from_n) = world.node(from_id) {
                let a = wpos(from_n.x, from_n.y);
                let target = state.hover.filter(|&h| h != from_id);
                let (color, path, cost) = match target {
                    Some(tid) => {
                        let kind = world.classify_edge(from_id, tid);
                        let cost = world.road_cost(from_id, tid);
                        let path = world.node(tid).and_then(|t| {
                            world.map.find_road(
                                (from_n.x as i32, from_n.y as i32),
                                (t.x as i32, t.y as i32),
                            )
                        });
                        (
                            kind.map(edge_color).unwrap_or(rgb([255, 120, 110])),
                            path,
                            cost,
                        )
                    }
                    None => (rgb([240, 240, 230]), None, None),
                };
                let _ = cost;
                match path {
                    Some(path) if path.len() >= 2 => {
                        let pts: Vec<Vec2> = path
                            .iter()
                            .map(|&(x, y)| wpos(x as f32 + 0.5, y as f32 + 0.5))
                            .collect();
                        gizmos.linestrip_2d(pts, color);
                    }
                    _ => {
                        gizmos.line_2d(a, wpos(cursor.x, cursor.y), color);
                    }
                }
                gizmos.circle_2d(wpos(cursor.x, cursor.y), 8.0 * px, color);
            }
        }
    }

    // Node outlines, rings and inner details.
    let mut pts: Vec<Vec2> = Vec::with_capacity(8);
    for n in world.nodes.values() {
        let center = wpos(n.x, n.y);
        let r = (n.radius() * 0.5).max(MIN_NODE_PX * px);
        let owner = owner_color(world, n.owner);
        let strong = matches!(n.owner, Owner::Player | Owner::Enemy);
        let stroke = if strong {
            owner
        } else {
            owner.with_alpha(0.75)
        };

        if n.kind == NodeType::Settlement {
            gizmos.circle_2d(center, r, stroke);
        } else {
            kind_outline(n.kind, center, r, &mut pts);
            gizmos.lineloop_2d(pts.iter().copied(), stroke);
        }

        // Settlement tier rings and fort markers stay inside the body.
        if zoom >= 14.0 {
            let inner = rgba([255, 255, 255], 90);
            match n.kind {
                NodeType::Settlement => {
                    if n.tier >= 1 {
                        gizmos.circle_2d(center, r * 0.66, inner);
                    }
                    if n.tier >= 3 {
                        gizmos.circle_2d(center, r * 0.36, inner);
                    }
                }
                NodeType::Fort => {
                    let d = r * 0.42;
                    gizmos.line_2d(center + Vec2::new(-d, -d), center + Vec2::new(d, d), inner);
                    gizmos.line_2d(center + Vec2::new(-d, d), center + Vec2::new(d, -d), inner);
                }
                _ => {}
            }
        }

        if n.besieged {
            gizmos.circle_2d(center, r + 4.0 * px, rgb([255, 90, 70]));
        } else if n.sabotage > 0 {
            gizmos.circle_2d(center, r + 4.0 * px, rgb([200, 120, 255]));
        }
    }

    // Spy markers: embedded agents get a wide ring, movers a white outline.
    for s in &world.spies {
        let Some(p) = spy_pos(world, s) else { continue };
        let center = wpos(p.x, p.y);
        let c = owner_color(world, s.owner);
        if s.status == SpyStatus::Embedded {
            gizmos.circle_2d(center, 7.0 * px, c);
        } else {
            gizmos.circle_2d(center, 4.0 * px, Color::WHITE);
        }
    }

    // Hover + selection rings.
    if let Some(id) = state.hover {
        if state.selected != Some(id) {
            if let Some(n) = world.node(id) {
                let center = wpos(n.x, n.y);
                let r = (n.radius() * 0.5).max(MIN_NODE_PX * px);
                gizmos.circle_2d(center, r + 4.0 * px, Color::WHITE);
            }
        }
    }
    if let Some(id) = state.selected {
        if let Some(n) = world.node(id) {
            let center = wpos(n.x, n.y);
            let r = (n.radius() * 0.5).max(MIN_NODE_PX * px);
            gizmos.circle_2d(center, r + 7.0 * px, rgb([255, 220, 80]));
        }
    }

    // Build ghost outline.
    if let Tool::Build(kind) = state.tool {
        if let Some(cursor) = cam.cursor {
            let tx = cursor.x.floor() + 0.5;
            let ty = cursor.y.floor() + 0.5;
            let ok = world.can_place(kind, tx, ty, Owner::Player).is_ok();
            let edge = if ok {
                rgb([90, 235, 120])
            } else {
                rgb([255, 90, 80])
            };
            let r = (if kind == NodeType::Settlement {
                zoom * 0.7
            } else {
                zoom * 0.5
            }
            .max(GHOST_PX))
                / zoom;
            let center = wpos(tx, ty);
            if kind == NodeType::Settlement {
                gizmos.circle_2d(center, r, edge);
            } else {
                kind_outline(kind, center, r, &mut pts);
                gizmos.lineloop_2d(pts.iter().copied(), edge);
            }
        }
    }
}

/// Leaving the game despawns the visuals (`DespawnOnExit`); drop the stale
/// entity handles so re-entering rebuilds them.
pub fn on_exit_game(mut vis: ResMut<WorldVis>) {
    vis.map_gen = u32::MAX;
    vis.terrain = None;
    vis.specks.clear();
    vis.nodes.clear();
    vis.spies.clear();
    vis.caravans.clear();
    vis.ghost = None;
    vis.roads.clear();
    vis.road_sig = 0;
}

/// The window background shows through wherever the map does not cover.
pub fn map_clear_color() -> ClearColor {
    ClearColor(rgb(MAP_BG))
}
