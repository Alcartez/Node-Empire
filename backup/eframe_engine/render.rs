//! World rendering: terrain baked to a NEAREST-filtered texture, roads as
//! polylines colored by role, nodes as distinct shapes with owner rings,
//! spies as traveling dots, plus build ghost and road preview.

use super::app::Tool;
use super::camera::Camera;
use crate::game::map::Map;
use crate::game::types::*;
use crate::game::world::World;
use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, TextureHandle, Vec2,
};

const MAP_BG: Color32 = Color32::from_rgb(18, 20, 24);

/// Everything the renderer needs to know about interaction state.
pub struct View<'a> {
    pub camera: &'a Camera,
    pub tool: Tool,
    pub selected: Option<u32>,
    /// Pointer position in world units (tile coords).
    pub cursor: Option<Vec2>,
    /// Node under the pointer.
    pub hover: Option<u32>,
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub fn owner_color(world: &World, owner: Owner) -> Color32 {
    match owner {
        Owner::Player => rgb(faction_def(world.player_faction).color),
        Owner::Enemy => rgb(faction_def(world.enemy_faction).color),
        Owner::Neutral => Color32::from_rgb(175, 175, 165),
        Owner::Bandit => Color32::from_rgb(145, 62, 52),
    }
}

fn kind_color(kind: NodeType) -> Color32 {
    match kind {
        NodeType::Settlement => Color32::from_rgb(220, 210, 190),
        NodeType::Farm => Color32::from_rgb(122, 192, 92),
        NodeType::Mine => Color32::from_rgb(192, 142, 92),
        NodeType::Woodcamp => Color32::from_rgb(78, 142, 82),
        NodeType::Factory => Color32::from_rgb(132, 152, 182),
        NodeType::Market => Color32::from_rgb(232, 192, 82),
        NodeType::Fort => Color32::from_rgb(172, 122, 112),
        NodeType::Laboratory => Color32::from_rgb(162, 122, 212),
        NodeType::BanditCamp => Color32::from_rgb(152, 72, 62),
    }
}

fn edge_color(kind: EdgeType) -> Color32 {
    match kind {
        EdgeType::Resource => Color32::from_rgb(208, 178, 118),
        EdgeType::Supply => Color32::from_rgb(112, 192, 132),
        EdgeType::Combat => Color32::from_rgb(222, 82, 72),
        EdgeType::Siege => Color32::from_rgb(238, 152, 58),
        EdgeType::Trade => Color32::from_rgb(238, 218, 108),
    }
}

fn deposit_color(d: Deposit) -> Option<Color32> {
    match d {
        Deposit::None => None,
        Deposit::Copper => Some(Color32::from_rgb(214, 127, 50)),
        Deposit::Iron => Some(Color32::from_rgb(176, 176, 192)),
    }
}

// ---------------------------------------------------------------------------
// Terrain texture
// ---------------------------------------------------------------------------

fn terrain_color(t: Terrain, shade: f32) -> Color32 {
    let base = match t {
        Terrain::Water => [40, 75, 130],
        Terrain::Sand => [214, 196, 140],
        Terrain::Grass => [110, 150, 90],
        Terrain::Forest => [66, 110, 66],
        Terrain::Mountain => [128, 122, 118],
    };
    let s = shade.clamp(0.5, 1.5);
    let f = |v: u8| (v as f32 * s).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(f(base[0]), f(base[1]), f(base[2]))
}

/// Bake the whole map into one small texture (1 texel per tile).
pub fn bake_terrain(ctx: &egui::Context, map: &Map) -> TextureHandle {
    let cols = map.cols as usize;
    let rows = map.rows as usize;
    let mut pixels = Vec::with_capacity(cols * rows);
    for y in 0..map.rows {
        for x in 0..map.cols {
            let t = map.terrain_at(x, y);
            let shade = map.shade[map.idx(x, y)];
            pixels.push(terrain_color(t, shade));
        }
    }
    ctx.load_texture(
        "terrain",
        egui::ColorImage::new([cols, rows], pixels),
        egui::TextureOptions::NEAREST,
    )
}

// ---------------------------------------------------------------------------
// Shapes
// ---------------------------------------------------------------------------

fn poly(center: Pos2, r: f32, sides: usize, rot: f32) -> Vec<Pos2> {
    (0..sides)
        .map(|i| {
            let a = rot + i as f32 * std::f32::consts::TAU / sides as f32;
            Pos2::new(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect()
}

fn draw_shape(painter: &Painter, center: Pos2, r: f32, kind: NodeType, fill: Color32, stroke: Stroke) {
    match kind {
        NodeType::Settlement | NodeType::Market | NodeType::Laboratory => {
            painter.circle_filled(center, r, fill);
            painter.circle_stroke(center, r, stroke);
        }
        NodeType::Farm => {
            let pts = poly(center, r, 3, -std::f32::consts::FRAC_PI_2);
            painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        NodeType::Mine => {
            let pts = poly(center, r, 4, 0.0);
            painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        NodeType::Woodcamp => {
            let pts = poly(center, r, 4, std::f32::consts::FRAC_PI_4);
            painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        NodeType::Factory => {
            let pts = poly(center, r, 6, 0.0);
            painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        NodeType::Fort => {
            let pts = poly(center, r, 5, -std::f32::consts::FRAC_PI_2);
            painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        NodeType::BanditCamp => {
            let pts = poly(center, r, 3, std::f32::consts::FRAC_PI_2);
            painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
    }
}

// ---------------------------------------------------------------------------
// Main draw
// ---------------------------------------------------------------------------

pub fn draw(painter: &Painter, rect: Rect, world: &World, terrain: &TextureHandle, view: &View) {
    let cam = view.camera;
    let map = &world.map;

    // World rect in screen space.
    let tl = cam.world_to_screen(rect, Vec2::ZERO);
    let br = cam.world_to_screen(rect, Vec2::new(map.cols as f32, map.rows as f32));
    let world_rect = Rect::from_two_pos(tl, br);

    painter.rect_filled(rect, 0.0, MAP_BG);
    painter.image(
        terrain.id(),
        world_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    painter.rect_stroke(
        world_rect,
        0.0,
        Stroke::new(1.5_f32, Color32::from_rgb(70, 74, 82)),
        egui::StrokeKind::Outside,
    );

    // Mineral deposits, when zoomed in enough to be useful.
    if cam.zoom >= 9.0 {
        let r = (cam.zoom * 0.14).max(1.5);
        for y in 0..map.rows {
            for x in 0..map.cols {
                let Some(c) = deposit_color(map.deposit_at(x, y)) else { continue };
                let p = cam.world_to_screen(rect, Vec2::new(x as f32 + 0.5, y as f32 + 0.5));
                if rect.expand(cam.zoom).contains(p) {
                    painter.circle_filled(p, r, c);
                }
            }
        }
    }

    // Roads.
    for e in &world.edges {
        let pts: Vec<Pos2> = e
            .path
            .iter()
            .map(|&(x, y)| cam.world_to_screen(rect, Vec2::new(x as f32 + 0.5, y as f32 + 0.5)))
            .collect();
        if pts.is_empty() {
            continue;
        }
        let carry = matches!(e.kind, EdgeType::Resource | EdgeType::Trade);
        let w = if carry {
            1.6 + (e.caravans.max(0) as f32) * 0.45
        } else {
            2.2
        };
        let col = edge_color(e.kind);
        painter.line(pts.clone(), Stroke::new(w, col));

        // Caravan marker crawling along the road.
        if carry && e.caravans > 0 && pts.len() >= 2 {
            let period = (e.length() * TILE_TRAVEL_TICKS / e.caravans as f32).max(2.0);
            let frac = (e.progress / period).clamp(0.0, 1.0);
            let seg = frac * (pts.len() - 1) as f32;
            let i = seg.floor() as usize;
            let t = seg - i as f32;
            if i + 1 < pts.len() {
                let a = pts[i];
                let b = pts[i + 1];
                let p = Pos2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
                painter.circle_filled(p, (w * 0.75).max(2.0), Color32::from_rgb(250, 245, 225));
            }
        }
    }

    // Faint spy lines home → target.
    for s in &world.spies {
        if let (Some(h), Some(t)) = (world.node(s.home), world.node(s.target)) {
            let a = cam.world_to_screen(rect, Vec2::new(h.x, h.y));
            let b = cam.world_to_screen(rect, Vec2::new(t.x, t.y));
            let c = owner_color(world, s.owner);
            painter.line_segment(
                [a, b],
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 70)),
            );
        }
    }

    // Nodes.
    for n in world.nodes.values() {
        let center = cam.world_to_screen(rect, Vec2::new(n.x, n.y));
        let r = (n.radius() * cam.zoom * 0.5).max(4.0);
        if !rect.expand(r + 40.0).contains(center) {
            continue;
        }
        let owner = owner_color(world, n.owner);
        let fill = if n.kind == NodeType::Settlement {
            owner
        } else {
            kind_color(n.kind)
        };
        let stroke_w = if n.owner == Owner::Player || n.owner == Owner::Enemy {
            2.2_f32
        } else {
            1.4_f32
        };
        let stroke = Stroke::new(stroke_w, owner);
        draw_shape(painter, center, r, n.kind, fill, stroke);

        // Inner details at readable zoom.
        if cam.zoom >= 14.0 && r >= 9.0 {
            match n.kind {
                NodeType::Settlement => {
                    let inner =
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 90));
                    if n.tier >= 1 {
                        painter.circle_stroke(center, r * 0.66, inner);
                    }
                    if n.tier >= 3 {
                        painter.circle_stroke(center, r * 0.36, inner);
                    }
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        n.population.to_string(),
                        FontId::proportional((r * 0.7).clamp(9.0, 15.0)),
                        Color32::WHITE,
                    );
                }
                NodeType::Fort => {
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        n.population.to_string(),
                        FontId::proportional((r * 0.7).clamp(9.0, 14.0)),
                        Color32::WHITE,
                    );
                }
                _ => {
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        n.kind.short(),
                        FontId::proportional(9.0),
                        Color32::from_rgb(25, 25, 30),
                    );
                }
            }
        }

        // Siege / sabotage indicators.
        if n.besieged {
            painter.circle_stroke(
                center,
                r + 4.0,
                Stroke::new(2.0_f32, Color32::from_rgb(255, 90, 70)),
            );
        } else if n.sabotage > 0 {
            painter.circle_stroke(
                center,
                r + 4.0,
                Stroke::new(1.5_f32, Color32::from_rgb(200, 120, 255)),
            );
        }

        // Labels.
        let show_label = cam.zoom >= 11.0 || view.selected == Some(n.id) || view.hover == Some(n.id);
        if show_label && n.is_settlement() {
            let name_c = if n.owner == Owner::Player {
                Color32::from_rgb(255, 250, 235)
            } else {
                Color32::from_rgb(225, 225, 220)
            };
            painter.text(
                Pos2::new(center.x, center.y - r - 4.0),
                Align2::CENTER_BOTTOM,
                n.name.clone(),
                FontId::proportional(12.0),
                name_c,
            );
        } else if cam.zoom >= 16.0 && view.hover == Some(n.id) {
            painter.text(
                Pos2::new(center.x, center.y - r - 4.0),
                Align2::CENTER_BOTTOM,
                n.name.clone(),
                FontId::proportional(11.0),
                Color32::from_rgb(225, 225, 220),
            );
        }
    }

    // Spy dots.
    for s in &world.spies {
        let c = owner_color(world, s.owner);
        let (pos, done) = match (world.node(s.home), world.node(s.target)) {
            (Some(h), Some(t)) => {
                let (hx, hy, tx, ty) = (h.x, h.y, t.x, t.y);
                match s.status {
                    SpyStatus::Embedded => (Vec2::new(tx, ty), false),
                    SpyStatus::Traveling => {
                        let f = s.progress.clamp(0.0, 1.0);
                        (Vec2::new(hx + (tx - hx) * f, hy + (ty - hy) * f), false)
                    }
                    SpyStatus::Returning => {
                        let f = s.progress.clamp(0.0, 1.0);
                        (Vec2::new(tx + (hx - tx) * f, ty + (hy - ty) * f), false)
                    }
                }
            }
            _ => continue,
        };
        let _ = done;
        let p = cam.world_to_screen(rect, pos);
        if !rect.expand(20.0).contains(p) {
            continue;
        }
        if s.status == SpyStatus::Embedded {
            painter.circle_stroke(p, 7.0, Stroke::new(2.0_f32, c));
            painter.circle_filled(p, 3.0, c);
        } else {
            painter.circle_filled(p, 4.0, c);
            painter.circle_stroke(p, 4.0, Stroke::new(1.0_f32, Color32::WHITE));
        }
    }

    // Hover + selection rings.
    if let Some(id) = view.hover {
        if view.selected != Some(id) {
            if let Some(n) = world.node(id) {
                let c = cam.world_to_screen(rect, Vec2::new(n.x, n.y));
                let r = (n.radius() * cam.zoom * 0.5).max(4.0);
                painter
                    .circle_stroke(c, r + 4.0, Stroke::new(1.5_f32, Color32::from_rgb(255, 255, 255)));
            }
        }
    }
    if let Some(id) = view.selected {
        if let Some(n) = world.node(id) {
            let c = cam.world_to_screen(rect, Vec2::new(n.x, n.y));
            let r = (n.radius() * cam.zoom * 0.5).max(4.0);
            painter
                .circle_stroke(c, r + 7.0, Stroke::new(2.2_f32, Color32::from_rgb(255, 220, 80)));
        }
    }

    // Tool overlays.
    match view.tool {
        Tool::Select => {}
        Tool::Build(kind) => {
            if let Some(cursor) = view.cursor {
                let tx = cursor.x.floor();
                let ty = cursor.y.floor();
                let snap = Vec2::new(tx + 0.5, ty + 0.5);
                let p = cam.world_to_screen(rect, snap);
                let ok = world.can_place(kind, snap.x, snap.y, Owner::Player).is_ok();
                let tint = if ok {
                    Color32::from_rgba_unmultiplied(140, 255, 160, 150)
                } else {
                    Color32::from_rgba_unmultiplied(255, 110, 100, 150)
                };
                let edge = if ok {
                    Color32::from_rgb(90, 235, 120)
                } else {
                    Color32::from_rgb(255, 90, 80)
                };
                let ghost_r = (cam.zoom * if kind == NodeType::Settlement { 0.7 } else { 0.5 }).max(6.0);
                draw_shape(
                    painter,
                    p,
                    ghost_r,
                    kind,
                    tint,
                    Stroke::new(1.8_f32, edge),
                );
                if !ok {
                    if let Err(msg) = world.can_place(kind, snap.x, snap.y, Owner::Player) {
                        painter.text(
                            Pos2::new(p.x, p.y - ghost_r - 6.0),
                            Align2::CENTER_BOTTOM,
                            msg,
                            FontId::proportional(11.0),
                            Color32::from_rgb(255, 130, 120),
                        );
                    }
                }
            }
        }
        Tool::Road { from } => {
            let (Some(from_id), Some(cursor)) = (from, view.cursor) else {
                if from.is_none() {
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "Click a node to start the road (Esc cancels)",
                        FontId::proportional(14.0),
                        Color32::from_rgb(255, 240, 180),
                    );
                }
                return;
            };
            let Some(from_n) = world.node(from_id) else { return };
            let a = cam.world_to_screen(rect, Vec2::new(from_n.x, from_n.y));
            let b = cam.world_to_screen(rect, cursor);

            // Prefer the hovered node as the endpoint.
            let target = view.hover.filter(|&h| h != from_id);
            let (line_color, preview_path, cost) = match target {
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
                        kind.map(edge_color).unwrap_or(Color32::from_rgb(255, 120, 110)),
                        path,
                        cost,
                    )
                }
                None => (Color32::from_rgb(240, 240, 230), None, None),
            };

            if let Some(path) = preview_path {
                let pts: Vec<Pos2> = path
                    .iter()
                    .map(|&(x, y)| cam.world_to_screen(rect, Vec2::new(x as f32 + 0.5, y as f32 + 0.5)))
                    .collect();
                painter.line(pts, Stroke::new(3.0_f32, line_color));
            } else {
                painter.line_segment([a, b], Stroke::new(2.0_f32, line_color));
            }

            // Endpoint marker + cost.
            painter.circle_stroke(b, 8.0, Stroke::new(2.0_f32, line_color));
            let label = match (target, cost) {
                (Some(_), Some(c)) => format!("{}g", c),
                (Some(_), None) => "no route".to_string(),
                (None, _) => String::new(),
            };
            if !label.is_empty() {
                painter.text(
                    Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5 - 8.0),
                    Align2::CENTER_BOTTOM,
                    label,
                    FontId::proportional(13.0),
                    line_color,
                );
            }
        }
    }
}
