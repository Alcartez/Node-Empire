//! HUD: panels, overlays and the main menu, plus the world text layer.
//!
//! Panels draw from immutable state and return `Action`s that the app applies
//! afterwards, which keeps "what happened" separate from "how it looks".
//! Every glyph on screen is egui; the shapes underneath are Bevy's.

use bevy_egui::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

use super::camera::Camera;
use super::state::Tool;
use super::theme;
use crate::game::save;
use crate::game::types::*;
use crate::game::world::{EventKind, World};

/// egui colour from an `[r, g, b]` triple (the game layer's palette format).
pub fn c32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

// ---------------------------------------------------------------------------
// UI -> app communication
// ---------------------------------------------------------------------------

/// Which page the right-hand panel is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightTab {
    Realm,
    Node,
    Agents,
}

impl RightTab {
    pub const ALL: [RightTab; 3] = [RightTab::Realm, RightTab::Node, RightTab::Agents];

    pub fn label(self) -> &'static str {
        match self {
            RightTab::Realm => "Realm",
            RightTab::Node => "Node",
            RightTab::Agents => "Agents",
        }
    }
}

pub enum Action {
    SetTool(Tool),
    SetTab(RightTab),
    Select(Option<u32>),
    Build(NodeType, f32, f32),
    Road(u32, u32),
    CutRoad(usize),
    CaravanDelta(usize, i32),
    TrainWorker(u32, usize),
    DismissWorker(u32, usize),
    TrainSpy(u32),
    SetSpyOp(u32, SpyOp),
    RecallSpy(u32),
    Foment(u32),
    BuySettlement(u32),
    AdvanceAge,
    Demolish(u32),
    PayGeneralBonus(u32),
    UpgradeFortEquipment(u32),
    Save(usize),
    Load(usize),
    NewGame,
    QuitToMenu,
    SetSpeed(u8),
    TogglePause,
    ToggleHelp,
    ToggleMenu,
}

/// Immutable snapshot the panels draw from.
pub struct Hud<'a> {
    pub world: &'a World,
    pub tool: Tool,
    pub tab: RightTab,
    pub selected: Option<u32>,
    pub speed: u8,
}

// ---------------------------------------------------------------------------
// Top bar: identity, treasury, age, clock, and the slim victory strip
// ---------------------------------------------------------------------------

pub fn top_bar(ui: &mut Ui, h: &Hud) -> Vec<Action> {
    let mut acts = Vec::new();
    let w = h.world;
    let led = w.player();
    let st = w.total_stock(Owner::Player);

    ui.horizontal(|ui| {
        let fac = faction_def(w.player_faction);
        theme::swatch(ui, c32(fac.color), 13.0);
        ui.label(RichText::new(fac.name).color(theme::TEXT).strong());

        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{}g", led.gold))
                .size(19.0)
                .strong()
                .color(theme::GOLD),
        );
        let inc = led.income_shown;
        ui.label(
            RichText::new(format!("{}{:.1}/t", if inc >= 0.0 { "+" } else { "" }, inc))
                .size(11.0)
                .color(if inc >= 0.0 { theme::GOOD } else { theme::BAD }),
        );

        ui.add_space(8.0);
        theme::chip(ui, &format!("Food {}", st.food), theme::GOOD);
        theme::chip(
            ui,
            &format!("Timber {}", st.timber),
            Color32::from_rgb(196, 152, 100),
        );
        theme::chip(
            ui,
            &format!("Ore {}", st.ore),
            Color32::from_rgb(170, 182, 200),
        );
        theme::chip(ui, &format!("Goods {}", st.goods), theme::ACCENT);
        theme::chip(
            ui,
            &format!("Pop {}", w.total_pop(Owner::Player)),
            theme::NEUTRAL,
        );

        ui.add_space(8.0);
        ui.label(
            RichText::new(led.age.label().to_string())
                .color(theme::TEXT)
                .size(12.0),
        );
        ui.label(
            RichText::new(format!("{} RP", led.tech))
                .color(theme::SPY)
                .size(12.0),
        );
        if let Some(cost) = led.age.cost_to_advance() {
            let can = led.tech >= cost;
            if ui
                .add_enabled(
                    can,
                    egui::Button::new(format!("Advance · {} RP", cost))
                        .min_size(egui::vec2(0.0, 22.0)),
                )
                .on_hover_text("Advance to the next age")
                .clicked()
            {
                acts.push(Action::AdvanceAge);
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new("Menu").min_size(egui::vec2(0.0, 22.0)))
                .clicked()
            {
                acts.push(Action::ToggleMenu);
            }
            if ui
                .add(egui::Button::new("Help").min_size(egui::vec2(0.0, 22.0)))
                .on_hover_text("How to play (H)")
                .clicked()
            {
                acts.push(Action::ToggleHelp);
            }
            ui.add_space(4.0);
            for s in [4u8, 2, 1] {
                let label = format!("{}\u{d7}", s);
                if ui
                    .add(
                        egui::Button::selectable(h.speed == s, label)
                            .min_size(egui::vec2(32.0, 22.0)),
                    )
                    .clicked()
                {
                    acts.push(Action::SetSpeed(s));
                }
            }
            if ui
                .add(egui::Button::selectable(h.speed == 0, "II").min_size(egui::vec2(32.0, 22.0)))
                .on_hover_text("Pause (Space)")
                .clicked()
            {
                acts.push(Action::SetSpeed(0));
            }
            ui.label(
                RichText::new(format!("Tick {}", w.tick))
                    .color(theme::DIM)
                    .size(11.5),
            );
        });
    });

    // The victory conditions, as one slim strip.
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        let wealth = (led.gold.max(0) as f32 / w.win_gold.max(1) as f32).clamp(0.0, 1.0);
        theme::meter_row(
            ui,
            "Wealth",
            &format!("{}g / {}g", led.gold, w.win_gold),
            wealth,
            theme::GOLD,
            236.0,
        );
        let (bound, total) = w.web_progress();
        let web = if total > 0 {
            bound as f32 / total as f32
        } else {
            0.0
        };
        theme::meter_row(
            ui,
            "Web",
            &format!("{} / {}", bound, total),
            web,
            theme::SPY,
            206.0,
        );
        let p = w.settlements_of(Owner::Player).len();
        let e = w.settlements_of(Owner::Enemy).len();
        let dom = if p + e > 0 {
            p as f32 / (p + e) as f32
        } else {
            0.0
        };
        theme::meter_row(
            ui,
            "Domination",
            &format!("{} / {}", p, p + e),
            dom,
            theme::BAD,
            236.0,
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ef = faction_def(w.enemy_faction);
            theme::chip(ui, &format!("Rival · {}", ef.name), c32(ef.color));
        });
    });

    acts
}

// ---------------------------------------------------------------------------
// Left panel: tools + build palette
// ---------------------------------------------------------------------------

pub fn left_panel(ui: &mut Ui, h: &Hud) -> Vec<Action> {
    let mut acts = Vec::new();
    let w = h.world;
    let gold = w.player().gold;

    theme::section(ui, "Tools");
    let sel = if matches!(h.tool, Tool::Select) { 0 } else { 1 };
    if let Some(i) = theme::segmented(
        ui,
        &["Select  V", "Road  R"],
        sel,
        ui.available_width(),
        26.0,
    ) {
        acts.push(Action::SetTool(match i {
            0 => Tool::Select,
            _ => Tool::Road { from: None },
        }));
    }

    theme::section(ui, "Build");
    let card_w = ((ui.available_width() - 8.0) * 0.5).max(70.0);
    egui::Grid::new("build_grid")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 7.0))
        .show(ui, |ui| {
            for (i, (kind, cost)) in BUILD_COSTS.iter().enumerate() {
                let active = matches!(h.tool, Tool::Build(k) if k == *kind);
                let affordable = gold >= *cost;
                let resp = build_card(ui, *kind, *cost, active, affordable, card_w);
                if resp.clicked() {
                    acts.push(Action::SetTool(Tool::Build(*kind)));
                }
                resp.on_hover_text(build_hint(*kind));
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });

    ui.add_space(8.0);
    let hint = match h.tool {
        Tool::Select => "Click a node to inspect it. Right-click deselects, drag to pan.",
        Tool::Build(NodeType::Mine) => {
            "Click grass bearing orange (copper) or silver (iron) specks."
        }
        Tool::Build(NodeType::Woodcamp) => "Click inside dark-green forest.",
        Tool::Build(NodeType::Settlement) => "Click open ground - not forest, water or mountains.",
        Tool::Build(_) => "Click the ground near your network.",
        Tool::Road { from: None } => "Click the first node of the road...",
        Tool::Road { from: Some(_) } => "...then the second. Esc cancels.",
    };
    theme::card(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new("TIP").size(9.5).strong().color(theme::FAINT));
        ui.label(RichText::new(hint).size(11.5).color(theme::DIM));
    });

    acts
}

/// A build-palette card: shape glyph, name, price.
fn build_card(
    ui: &mut Ui,
    kind: NodeType,
    cost: i32,
    active: bool,
    affordable: bool,
    width: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 42.0), Sense::click());
    let bg = if active {
        theme::CARD_ACTIVE
    } else if !affordable {
        theme::CARD.gamma_multiply(0.55)
    } else if resp.hovered() {
        theme::CARD_HOVER
    } else {
        theme::CARD
    };
    let fg = if affordable {
        theme::TEXT
    } else {
        theme::FAINT
    };
    let p = ui.painter();
    p.rect_filled(rect, 7.0, bg);
    p.rect_stroke(
        rect,
        7.0,
        Stroke::new(
            if active { 1.5 } else { 1.0 },
            if active { theme::ACCENT } else { theme::LINE },
        ),
        StrokeKind::Inside,
    );
    glyph(
        p,
        Pos2::new(rect.left() + 18.0, rect.center().y),
        9.5,
        kind,
        if affordable {
            theme::ACCENT
        } else {
            theme::FAINT
        },
    );
    p.text(
        Pos2::new(rect.left() + 32.0, rect.center().y - 7.0),
        Align2::LEFT_CENTER,
        kind.label(),
        FontId::proportional(11.5),
        fg,
    );
    p.text(
        Pos2::new(rect.left() + 32.0, rect.center().y + 7.0),
        Align2::LEFT_CENTER,
        format!("{}g", cost),
        FontId::proportional(10.5),
        if affordable {
            theme::GOLD
        } else {
            theme::FAINT
        },
    );
    resp
}

fn build_hint(kind: NodeType) -> &'static str {
    match kind {
        NodeType::Settlement => "Found a new village: population, food hub, spy school.",
        NodeType::Farm => "Feeds nearby settlements. Best beside water.",
        NodeType::Mine => "Needs copper (orange) or iron (silver) specks.",
        NodeType::Woodcamp => "Needs forest. Produces timber.",
        NodeType::Factory => "Refines ore + timber into trade goods.",
        NodeType::Market => "Sells goods for gold; boosts nearby trade.",
        NodeType::Fort => "Garrison, defense, and sieges.",
        NodeType::Laboratory => "Generates research points each tick.",
        NodeType::BanditCamp => "A bandit hold - capture it instead.",
    }
}

/// Draw a node kind's shape as a small HUD glyph.
fn glyph(p: &Painter, center: Pos2, r: f32, kind: NodeType, color: Color32) {
    use std::f32::consts::{FRAC_PI_2, TAU};
    let stroke = Stroke::new(1.3, color);
    let fill = color.gamma_multiply(0.22);
    let ngon = |sides: usize, phase: f32| -> Vec<Pos2> {
        (0..sides)
            .map(|i| {
                let a = phase + TAU * i as f32 / sides as f32;
                Pos2::new(center.x + a.cos() * r, center.y + a.sin() * r)
            })
            .collect()
    };
    // Mirrors `render::kind_shape` so the card matches the map.
    match kind {
        NodeType::Settlement | NodeType::Market | NodeType::Laboratory => {
            p.circle(center, r * 0.85, fill, stroke);
        }
        NodeType::Farm => {
            p.add(egui::Shape::convex_polygon(
                ngon(3, -FRAC_PI_2),
                fill,
                stroke,
            ));
        }
        NodeType::Mine => {
            p.add(egui::Shape::convex_polygon(ngon(4, 0.0), fill, stroke));
        }
        NodeType::Woodcamp => {
            p.add(egui::Shape::convex_polygon(
                ngon(4, std::f32::consts::FRAC_PI_4),
                fill,
                stroke,
            ));
        }
        NodeType::Factory => {
            p.add(egui::Shape::convex_polygon(ngon(6, 0.0), fill, stroke));
        }
        NodeType::Fort => {
            p.add(egui::Shape::convex_polygon(
                ngon(5, -FRAC_PI_2),
                fill,
                stroke,
            ));
        }
        NodeType::BanditCamp => {
            p.add(egui::Shape::convex_polygon(
                ngon(3, FRAC_PI_2),
                fill,
                stroke,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Right panel: Realm | Node | Agents
// ---------------------------------------------------------------------------

pub fn right_panel(ui: &mut Ui, h: &Hud) -> Vec<Action> {
    let mut acts = Vec::new();
    let w = h.world;

    let sel = RightTab::ALL.iter().position(|t| *t == h.tab).unwrap_or(0);
    let labels: Vec<&str> = RightTab::ALL.iter().map(|t| t.label()).collect();
    if let Some(i) = theme::segmented(ui, &labels, sel, ui.available_width(), 28.0) {
        acts.push(Action::SetTab(RightTab::ALL[i]));
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| match h.tab {
            RightTab::Realm => realm_tab(ui, w),
            RightTab::Node => match h.selected.and_then(|id| w.node(id)) {
                Some(n) => inspector(ui, w, n, &mut acts),
                None => {
                    theme::section(ui, "Node");
                    ui.label(
                        RichText::new("Nothing selected. Click a node on the map to inspect it.")
                            .italics()
                            .size(11.5)
                            .color(theme::FAINT),
                    );
                }
            },
            RightTab::Agents => agents(ui, w, &mut acts),
        });

    acts
}

/// The realm at a glance: holdings, treasury, victory, rival.
fn realm_tab(ui: &mut Ui, w: &World) {
    let led = w.player();

    theme::section(ui, "Holdings");
    let towns = w.settlements_of(Owner::Player);
    if towns.is_empty() {
        ui.label(
            RichText::new("None - you are a wanderer.")
                .italics()
                .color(theme::BAD),
        );
    } else {
        egui::Grid::new("realm_towns")
            .num_columns(3)
            .spacing(egui::vec2(8.0, 3.0))
            .show(ui, |ui| {
                for n in &towns {
                    ui.label(RichText::new(n.name.as_str()).color(theme::TEXT).size(12.0));
                    ui.label(
                        RichText::new(format!("pop {}", n.population))
                            .color(theme::DIM)
                            .size(11.0),
                    );
                    let (mood, col) = if n.besieged {
                        ("besieged", theme::BAD)
                    } else if n.sabotage > 0 {
                        ("sabotaged", theme::SPY)
                    } else if n.happiness < 45.0 {
                        ("restless", theme::WARN)
                    } else if n.loyalty < 40 {
                        ("wavering", theme::WARN)
                    } else {
                        ("loyal", theme::GOOD)
                    };
                    ui.label(RichText::new(mood).color(col).size(10.5));
                    ui.end_row();
                }
            });
    }

    theme::section(ui, "Treasury");
    let inc = led.income_shown;
    egui::Grid::new("realm_treasury")
        .num_columns(2)
        .spacing(egui::vec2(12.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new("Gold").color(theme::DIM).size(11.5));
            ui.label(
                RichText::new(format!("{}g", led.gold))
                    .color(theme::GOLD)
                    .strong(),
            );
            ui.end_row();
            ui.label(RichText::new("Income").color(theme::DIM).size(11.5));
            ui.label(
                RichText::new(format!("{}{:.1}/t", if inc >= 0.0 { "+" } else { "" }, inc))
                    .color(if inc >= 0.0 { theme::GOOD } else { theme::BAD }),
            );
            ui.end_row();
            ui.label(RichText::new("Research").color(theme::DIM).size(11.5));
            ui.label(RichText::new(format!("{} RP", led.tech)).color(theme::SPY));
            ui.end_row();
            ui.label(RichText::new("Age").color(theme::DIM).size(11.5));
            ui.label(RichText::new(led.age.label()).color(theme::TEXT));
            ui.end_row();
            ui.label(RichText::new("Population").color(theme::DIM).size(11.5));
            ui.label(format!("{}", w.total_pop(Owner::Player)));
            ui.end_row();
            ui.label(RichText::new("Nodes").color(theme::DIM).size(11.5));
            ui.label(format!("{}", w.nodes_owned_by(Owner::Player).len()));
            ui.end_row();
        });

    let st = w.total_stock(Owner::Player);
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        theme::chip(ui, &format!("Food {}", st.food), theme::GOOD);
        theme::chip(
            ui,
            &format!("Timber {}", st.timber),
            Color32::from_rgb(196, 152, 100),
        );
        theme::chip(
            ui,
            &format!("Ore {}", st.ore),
            Color32::from_rgb(170, 182, 200),
        );
        theme::chip(ui, &format!("Goods {}", st.goods), theme::ACCENT);
    });

    theme::section(ui, "Victory");
    let wealth = (led.gold.max(0) as f32 / w.win_gold.max(1) as f32).clamp(0.0, 1.0);
    theme::meter_row(
        ui,
        "Wealth",
        &format!("{}g / {}g", led.gold, w.win_gold),
        wealth,
        theme::GOLD,
        286.0,
    );
    let (bound, total) = w.web_progress();
    theme::meter_row(
        ui,
        "Web",
        &format!("{} / {}", bound, total),
        if total > 0 {
            bound as f32 / total as f32
        } else {
            0.0
        },
        theme::SPY,
        286.0,
    );
    let p = w.settlements_of(Owner::Player).len();
    let e = w.settlements_of(Owner::Enemy).len();
    theme::meter_row(
        ui,
        "Domination",
        &format!("{} / {}", p, p + e),
        if p + e > 0 {
            p as f32 / (p + e) as f32
        } else {
            0.0
        },
        theme::BAD,
        286.0,
    );
    ui.add_space(3.0);
    ui.label(
        RichText::new(w.victory_progress())
            .size(11.5)
            .color(theme::DIM),
    );
    ui.label(
        RichText::new(
            "Wealth: hoard gold. Web: an infiltrating agent plus 70 influence in every rival \
             settlement. Domination: hold every enemy settlement.",
        )
        .italics()
        .size(10.5)
        .color(theme::FAINT),
    );

    theme::section(ui, "Rival");
    let ef = faction_def(w.enemy_faction);
    let e_led = w.enemy();
    ui.horizontal(|ui| {
        theme::swatch(ui, c32(ef.color), 12.0);
        ui.label(RichText::new(ef.name).color(theme::TEXT).size(12.0));
    });
    ui.label(
        RichText::new(format!(
            "{}g \u{b7} {} RP \u{b7} {} settlements \u{b7} {}",
            e_led.gold,
            e_led.tech,
            e,
            e_led.age.label()
        ))
        .color(theme::DIM)
        .size(11.0),
    );
    ui.label(
        RichText::new(ef.blurb)
            .italics()
            .size(10.5)
            .color(theme::FAINT),
    );
}

/// Plain-language reasons a settlement's growth roll is blocked.
///
/// Order draws on employment (fillable posts) and a fort supply link; growth
/// then needs happiness > 55 and order > 40. This spells the gap out instead
/// of leaving a motionless population bar unexplained.
fn growth_notes(w: &World, n: &crate::game::world::Node) -> Vec<String> {
    if !n.is_settlement() || n.owner != Owner::Player {
        return Vec::new();
    }
    let (filled, posts) = w.employment_of(Owner::Player);
    let fort = w.has_fort_supply(n.id);
    let mut notes = vec![format!(
        "Order {:.0}% \u{b7} {}/{} posts filled \u{b7} {}",
        n.public_order,
        filled,
        posts,
        if fort {
            "fort supply"
        } else {
            "no fort supply"
        }
    )];

    let starving = n.stock.food < (n.population / 10).max(1);
    if starving {
        notes.push("Growth stalled: the settlement is starving".into());
    } else if n.happiness <= GROWTH_MIN_HAPPINESS || n.public_order <= GROWTH_MIN_ORDER {
        notes.push(format!(
            "Growth stalled: happiness {:.0} (needs {:.0}) \u{b7} order {:.0} (needs {:.0})",
            n.happiness, GROWTH_MIN_HAPPINESS, n.public_order, GROWTH_MIN_ORDER
        ));
    }
    notes
}

fn inspector(ui: &mut Ui, w: &World, n: &crate::game::world::Node, acts: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(n.name.as_str())
                .size(17.0)
                .strong()
                .color(theme::TEXT),
        );
        if n.owner == Owner::Player {
            theme::chip(ui, "yours", theme::GOOD);
        } else if n.besieged {
            theme::chip(ui, "under siege", theme::BAD);
        }
    });
    ui.label(
        RichText::new(format!("{} \u{b7} {}", n.kind.label(), owner_text(n)))
            .color(theme::DIM)
            .size(11.5),
    );
    ui.add_space(2.0);

    if n.is_settlement() {
        let tier = ["Village", "Town", "City", "Metropolis"][n.tier.min(3) as usize];
        egui::Grid::new("node_city")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 3.0))
            .show(ui, |ui| {
                if let Some(race) = n.dominant_race {
                    ui.label(RichText::new("Demographics").color(theme::DIM).size(11.0));
                    ui.horizontal(|ui| {
                        theme::chip(ui, race.label(), theme::ACCENT);
                    });
                    ui.end_row();
                }
                ui.label(RichText::new("Population").color(theme::DIM).size(11.0));
                ui.label(
                    RichText::new(format!("{} \u{b7} {}", n.population, tier)).color(theme::TEXT),
                );
                ui.end_row();
                ui.label(RichText::new("Happiness").color(theme::DIM).size(11.0));
                ui.label(
                    RichText::new(format!("{:.0}%", n.happiness)).color(mood_color(n.happiness)),
                );
                ui.end_row();
                ui.label(RichText::new("Order").color(theme::DIM).size(11.0));
                ui.label(
                    RichText::new(format!("{:.0}%", n.public_order))
                        .color(mood_color(n.public_order)),
                );
                ui.end_row();
                ui.label(RichText::new("Loyalty").color(theme::DIM).size(11.0));
                ui.label(RichText::new(format!("{}", n.loyalty)).color(theme::NEUTRAL));
                ui.end_row();
                if n.owner == Owner::Neutral {
                    let fealty = n.fealty.get(&Owner::Player).copied().unwrap_or(0.0);
                    let tier = n.fealty_tier(Owner::Player);
                    ui.label(RichText::new("Fealty").color(theme::GOLD).size(11.0));
                    ui.label(
                        RichText::new(format!("{:.0}% \u{b7} {}", fealty, tier.label()))
                            .color(theme::GOLD)
                            .strong(),
                    );
                    ui.end_row();
                    if let Some(spec) = n.specialty {
                        ui.label(RichText::new("Specialty").color(theme::DIM).size(11.0));
                        ui.horizontal(|ui| {
                            theme::chip(ui, spec.label(), theme::GOLD);
                        });
                        ui.end_row();
                    }
                }
            });
        ui.add_space(3.0);
        ui.horizontal_wrapped(|ui| {
            theme::chip(ui, &format!("Food {}", n.stock.food), theme::GOOD);
            theme::chip(
                ui,
                &format!("Timber {}", n.stock.timber),
                Color32::from_rgb(196, 152, 100),
            );
            theme::chip(
                ui,
                &format!("Ore {}", n.stock.ore),
                Color32::from_rgb(170, 182, 200),
            );
            theme::chip(ui, &format!("Goods {}", n.stock.goods), theme::ACCENT);
        });
        if let Some(race) = n.dominant_race {
            let race_blurb = match race {
                Race::Human => "Human: versatile baseline, balanced civic and martial order",
                Race::Beetlefolk => "Beetlefolk: +20% combat, +35% fort defense, -15% growth",
                Race::HiveSwarm => "Hive Swarm: rapid expansion +45% growth, +30% appetite",
                Race::SilkSpinner => "Silk Spinner: artisan lineage, +5% growth, efficient food",
                Race::Reptilian => {
                    "Reptilian: cold-blooded endurance, -25% food, +15% combat valor"
                }
            };
            ui.label(
                RichText::new(race_blurb)
                    .italics()
                    .size(10.5)
                    .color(theme::FAINT),
            );
        }
        if let Some(spec) = n.specialty {
            ui.label(
                RichText::new(format!("Specialty: {}", spec.bonus_description()))
                    .italics()
                    .size(10.5)
                    .color(theme::GOLD),
            );
        }
        if n.besieged {
            ui.colored_label(theme::BAD, "Under siege!");
        }
        if n.sabotage > 0 {
            ui.colored_label(theme::SPY, "Sabotaged production!");
        }
        // Why the population isn't moving: say it in plain words.
        for note in growth_notes(w, n) {
            ui.label(RichText::new(note).size(10.5).color(theme::FAINT));
        }
    } else {
        egui::Grid::new("node_prod")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 3.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Workers").color(theme::DIM).size(11.0));
                ui.label(format!("{} / {} slots", n.worker_count(), n.job_slots()));
                ui.end_row();
                ui.label(RichText::new("Garrison").color(theme::DIM).size(11.0));
                if n.kind == NodeType::Fort {
                    let cap = w.garrison_cap_at(n.id);
                    ui.label(format!(
                        "{} / {} \u{b7} from its village",
                        n.population, cap
                    ));
                } else {
                    ui.label(format!("{}", n.population));
                }
                ui.end_row();
            });

        if n.kind == NodeType::Fort {
            theme::section(ui, "Armory & Equipment");
            ui.horizontal(|ui| {
                theme::chip(ui, n.equipment.label(), theme::ACCENT);
                let (c_mult, d_mult) = (n.equipment.combat_mult(), n.equipment.defense_mult());
                ui.label(
                    RichText::new(format!(
                        "+{:.0}% cbt \u{b7} +{:.0}% def",
                        (c_mult - 1.0) * 100.0,
                        (d_mult - 1.0) * 100.0
                    ))
                    .size(11.0)
                    .color(theme::DIM),
                );
            });
            if n.owner == Owner::Player {
                if let Some(next) = n.equipment.next() {
                    let gold_cost = (n.equipment.upgrade_cost() * 5).max(10);
                    let can_afford = w.player().gold >= gold_cost;
                    if ui
                        .add_enabled(
                            can_afford,
                            egui::Button::new(format!("Forge {} ({}g)", next.label(), gold_cost))
                                .min_size(egui::vec2(0.0, 22.0)),
                        )
                        .on_hover_text("Upgrade fort garrison weapons and siege defenses")
                        .clicked()
                    {
                        acts.push(Action::UpgradeFortEquipment(n.id));
                    }
                } else {
                    ui.label(
                        RichText::new("Maximum armory tier achieved.")
                            .size(10.5)
                            .color(theme::FAINT),
                    );
                }
            }

            theme::section(ui, "Commanding General");
            let gen_opt = w.generals.iter().find(|g| g.fort_id == Some(n.id));
            if let Some(gen) = gen_opt {
                theme::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(gen.name.as_str()).strong().color(theme::TEXT));
                        theme::chip(ui, gen.race.label(), theme::ACCENT);
                    });
                    ui.label(
                        RichText::new(format!(
                            "Ldr: {} \u{b7} Cbt: {} \u{b7} Kills: {}",
                            gen.leadership, gen.combat, gen.kills
                        ))
                        .size(11.0)
                        .color(theme::DIM),
                    );
                    let loyalty_color = if gen.loyalty >= 70 {
                        theme::GOOD
                    } else if gen.loyalty >= 30 {
                        theme::WARN
                    } else {
                        theme::BAD
                    };
                    theme::meter_row(
                        ui,
                        "Loyalty",
                        &format!("{}%", gen.loyalty),
                        gen.loyalty as f32 / 100.0,
                        loyalty_color,
                        240.0,
                    );
                    if gen.loyalty < 30 {
                        ui.colored_label(theme::BAD, "Warning: General nearing mutiny!");
                    }
                    if n.owner == Owner::Player {
                        let gold = w.player().gold;
                        if ui
                            .add_enabled(
                                gold >= 50 && gen.loyalty < 100,
                                egui::Button::new("Bestow 50g Bonus (+15 Loyalty)")
                                    .min_size(egui::vec2(0.0, 21.0)),
                            )
                            .on_hover_text("Reward general from royal treasury to increase loyalty")
                            .clicked()
                        {
                            acts.push(Action::PayGeneralBonus(gen.id));
                        }
                    }
                });
            } else {
                ui.label(
                    RichText::new("No general stationed here.")
                        .italics()
                        .size(11.0)
                        .color(theme::FAINT),
                );
            }
        }
    }

    // Foreign settlements: the influence game.
    if n.is_settlement() && n.owner != Owner::Player {
        theme::section(ui, "Influence");
        let inf = w.influence(Owner::Player, n.id);
        theme::meter_row(
            ui,
            "",
            &format!("{:.0}/100", inf),
            inf / 100.0,
            theme::SPY,
            286.0,
        );
        let gold = w.player().gold;
        if ui
            .add_enabled(
                inf >= 61.0 && gold >= 100,
                egui::Button::new("Foment dissent (100g)").min_size(egui::vec2(0.0, 22.0)),
            )
            .on_hover_text("Stir unrest: halves loyalty and sparks a crisis")
            .clicked()
        {
            acts.push(Action::Foment(n.id));
        }
        if ui
            .add_enabled(
                inf >= 81.0 && gold >= 200,
                egui::Button::new("Buy allegiance (200g)").min_size(egui::vec2(0.0, 22.0)),
            )
            .on_hover_text("Needs 81 influence")
            .clicked()
        {
            acts.push(Action::BuySettlement(n.id));
        }
        if ui
            .add_enabled(
                gold >= SPY_TRAIN_COST,
                egui::Button::new(format!("Send spy ({}g)", SPY_TRAIN_COST))
                    .min_size(egui::vec2(0.0, 22.0)),
            )
            .on_hover_text("Recruit an agent at home; slip it into this court")
            .clicked()
        {
            acts.push(Action::TrainSpy(n.id));
        }
    }

    // Own production: crew management.
    let trainable = n.owner == Owner::Player
        && (n.is_production() || n.kind == NodeType::Fort || n.kind == NodeType::Laboratory);
    if trainable {
        theme::section(ui, "Crew");
        if n.workers.is_empty() {
            ui.label(
                RichText::new("No crew yet.")
                    .italics()
                    .size(11.0)
                    .color(theme::FAINT),
            );
        }
        for (i, wk) in n.workers.iter().enumerate() {
            ui.push_id(("crew", i), |ui| {
                ui.horizontal(|ui| {
                    let train = if wk.ready() {
                        String::new()
                    } else {
                        format!(" ({}t)", wk.training_left)
                    };
                    ui.label(
                        RichText::new(format!(
                            "{} \u{b7} {} \u{b7} skill {}{}",
                            wk.name,
                            wk.job.label(),
                            wk.skill,
                            train
                        ))
                        .size(11.5)
                        .color(theme::TEXT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("Dismiss")
                            .on_hover_text("Send this worker home")
                            .clicked()
                        {
                            acts.push(Action::DismissWorker(n.id, i));
                        }
                    });
                });
            });
        }
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            let names = ["Novice", "Adept", "Master"];
            for (t, cost) in TRAIN_COSTS.iter().enumerate() {
                let gold = w.player().gold;
                if ui
                    .add_enabled(
                        gold >= *cost,
                        egui::Button::new(format!("{} {}g", names[t], cost))
                            .min_size(egui::vec2(0.0, 21.0)),
                    )
                    .on_hover_text(format!("Starting skill {}", TRAIN_SKILLS[t]))
                    .clicked()
                {
                    acts.push(Action::TrainWorker(n.id, t));
                }
            }
        });
    }

    // Roads.
    theme::section(ui, "Roads");
    let edges: Vec<(usize, u32, EdgeType, Owner, i32)> = w
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.from == n.id || e.to == n.id)
        .map(|(i, e)| (i, e.other(n.id), e.kind, e.by, e.caravans))
        .collect();
    if edges.is_empty() {
        ui.label(
            RichText::new("None yet. Road to neighbours to trade, supply or attack.")
                .italics()
                .size(11.0)
                .color(theme::FAINT),
        );
    }
    for (idx, other, kind, by, caravans) in edges {
        let oname = w
            .node(other)
            .map(|x| x.name.clone())
            .unwrap_or_else(|| "?".to_string());
        ui.push_id(("road", idx), |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} \u{b7} {}", oname, kind.label()))
                        .size(11.5)
                        .color(theme::TEXT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if by == Owner::Player && ui.small_button("Cut").clicked() {
                        acts.push(Action::CutRoad(idx));
                    }
                    if by == Owner::Player && matches!(kind, EdgeType::Resource | EdgeType::Trade) {
                        if ui.small_button("-").clicked() {
                            acts.push(Action::CaravanDelta(idx, -1));
                        }
                        ui.label(
                            RichText::new(format!("{}", caravans))
                                .size(11.0)
                                .color(theme::DIM),
                        );
                        if ui.small_button("+").clicked() {
                            acts.push(Action::CaravanDelta(idx, 1));
                        }
                    }
                });
            });
        });
    }

    if n.owner == Owner::Player {
        ui.add_space(6.0);
        if ui
            .add(egui::Button::new("Demolish").min_size(egui::vec2(0.0, 22.0)))
            .on_hover_text("Tear down and recover part of the cost")
            .clicked()
        {
            acts.push(Action::Demolish(n.id));
        }
    }
}

fn owner_text(n: &crate::game::world::Node) -> String {
    match n.owner {
        Owner::Player => "Yours".to_string(),
        Owner::Enemy => format!(
            "Enemy ({})",
            n.faction.map(|f| faction_def(f).name).unwrap_or("?")
        ),
        Owner::Neutral => "Neutral".to_string(),
        Owner::Bandit => "Bandits".to_string(),
    }
}

fn mood_color(v: f32) -> Color32 {
    if v >= 70.0 {
        theme::GOOD
    } else if v >= 45.0 {
        theme::WARN
    } else {
        theme::BAD
    }
}

fn agents(ui: &mut Ui, w: &World, acts: &mut Vec<Action>) {
    theme::section(ui, "Generals");
    let my_generals: Vec<&General> = w
        .generals
        .iter()
        .filter(|g| g.owner == Owner::Player)
        .collect();
    if my_generals.is_empty() {
        ui.label(
            RichText::new("No generals in your realm.")
                .italics()
                .size(11.5)
                .color(theme::FAINT),
        );
    } else {
        for gen in my_generals {
            theme::card(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(gen.name.as_str()).strong().color(theme::TEXT));
                    theme::chip(ui, gen.race.label(), theme::ACCENT);
                });
                let fort_name = gen
                    .fort_id
                    .and_then(|fid| w.node(fid))
                    .map(|n| n.name.clone())
                    .unwrap_or_else(|| "Unassigned".to_string());
                ui.label(
                    RichText::new(format!(
                        "Fort: {} \u{b7} Ldr: {} \u{b7} Cbt: {} \u{b7} Kills: {}",
                        fort_name, gen.leadership, gen.combat, gen.kills
                    ))
                    .size(11.0)
                    .color(theme::DIM),
                );
                let loyalty_color = if gen.loyalty >= 70 {
                    theme::GOOD
                } else if gen.loyalty >= 30 {
                    theme::WARN
                } else {
                    theme::BAD
                };
                theme::meter_row(
                    ui,
                    "Loyalty",
                    &format!("{}%", gen.loyalty),
                    gen.loyalty as f32 / 100.0,
                    loyalty_color,
                    ui.available_width(),
                );
                if gen.loyalty < 30 {
                    ui.colored_label(theme::BAD, "Warning: General nearing mutiny!");
                }
                ui.horizontal(|ui| {
                    if let Some(fid) = gen.fort_id {
                        if ui.small_button("View Fort").clicked() {
                            acts.push(Action::Select(Some(fid)));
                            acts.push(Action::SetTab(RightTab::Node));
                        }
                    }
                    let gold = w.player().gold;
                    if ui
                        .add_enabled(
                            gold >= 50 && gen.loyalty < 100,
                            egui::Button::new("Pay 50g Bonus (+15)")
                                .min_size(egui::vec2(0.0, 20.0)),
                        )
                        .on_hover_text(
                            "Bribe general with royal treasury gold to secure their loyalty",
                        )
                        .clicked()
                    {
                        acts.push(Action::PayGeneralBonus(gen.id));
                    }
                });
            });
            ui.add_space(2.0);
        }
    }

    ui.add_space(4.0);
    theme::section(ui, "Spies");
    let mine: Vec<&Spy> = w
        .spies
        .iter()
        .filter(|s| s.owner == Owner::Player)
        .collect();
    if mine.is_empty() {
        ui.label(
            RichText::new("No agents. Select a foreign settlement and send a spy to recruit one.")
                .italics()
                .size(11.5)
                .color(theme::FAINT),
        );
        return;
    }
    for s in mine {
        let tname = w.node(s.target).map(|n| n.name.clone()).unwrap_or_default();
        let (st, col) = match s.status {
            SpyStatus::Traveling => ("traveling", theme::WARN),
            SpyStatus::Embedded => ("embedded", theme::SPY),
            SpyStatus::Returning => ("returning", theme::ACCENT),
        };
        theme::card(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(s.name.as_str()).strong().color(theme::TEXT));
                ui.label(
                    RichText::new(format!("\u{2192} {}", tname))
                        .size(11.5)
                        .color(theme::DIM),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.push_id(("recall", s.id), |ui| {
                        if ui.small_button("Recall").clicked() {
                            acts.push(Action::RecallSpy(s.id));
                        }
                    });
                    theme::chip(ui, st, col);
                    ui.label(
                        RichText::new(format!("skill {}", s.skill))
                            .size(11.0)
                            .color(theme::DIM),
                    );
                });
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                for op in [
                    SpyOp::Infiltrate,
                    SpyOp::Siphon,
                    SpyOp::Sabotage,
                    SpyOp::Incite,
                ] {
                    if ui
                        .add(
                            egui::Button::selectable(s.op == op, op.label())
                                .min_size(egui::vec2(0.0, 20.0)),
                        )
                        .on_hover_text(op.hint())
                        .clicked()
                    {
                        acts.push(Action::SetSpyOp(s.id, op));
                    }
                }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Bottom panel: chronicle ticker
// ---------------------------------------------------------------------------

pub fn log_panel(ui: &mut Ui, w: &World) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("CHRONICLE")
                .size(9.5)
                .strong()
                .color(theme::FAINT),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{} entries", w.log.len()))
                    .size(9.5)
                    .color(theme::FAINT),
            );
        });
    });
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .max_height(62.0)
        .show(ui, |ui| {
            let tail: Vec<&crate::game::world::LogEvent> = w.log.iter().rev().take(40).collect();
            for ev in tail.iter().rev() {
                let c = match ev.kind {
                    EventKind::Good => theme::GOOD,
                    EventKind::Bad => theme::BAD,
                    EventKind::War => theme::WARN,
                    EventKind::Spy => theme::SPY,
                    EventKind::Neutral => theme::NEUTRAL,
                };
                ui.label(
                    RichText::new(format!("{:>6}  {}", ev.tick, ev.text))
                        .size(10.5)
                        .color(c.gamma_multiply(0.85)),
                );
            }
        });
}

// ---------------------------------------------------------------------------
// Overlays
// ---------------------------------------------------------------------------

pub fn help_window(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("How to Play")
        .open(open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(560.0);
            theme::section(ui, "Victory");
            ui.label(
                RichText::new(
                    "Wealth: hold the gold threshold. Web: an infiltrating agent plus 70 \
                     influence in every rival settlement. Domination: hold every enemy \
                     settlement. You lose when your last settlement falls.",
                )
                .size(11.5)
                .color(theme::TEXT),
            );

            theme::section(ui, "Economy");
            ui.label(
                RichText::new(
                    "Farms feed settlements. Mines need ore specks, Woodcamps need forest. \
                     Factories refine ore and timber into goods, Markets sell them, \
                     Laboratories make research. Roads ship between nodes - add caravans to \
                     speed the flow.",
                )
                .size(11.5)
                .color(theme::TEXT),
            );

            theme::section(ui, "Growth & order");
            ui.label(
                RichText::new(format!(
                    "Public order comes from two things only: how many of your workable posts \
                     are actually staffed (farms, camps, mines, factories, markets, forts and \
                     laboratories \u{2014} settlements do not count), and a Supply road from a \
                     settlement to one of your forts. A settlement grows while happiness is \
                     above {:.0} and order above {:.0}: an idle realm sits at 34/34, a fort \
                     alone reaches 54 \u{2014} a deliberate point short \u{2014} and a fort \
                     plus one staffed post clears the bar. Starving zeroes order; sieges, \
                     bandit raids and unsettled realms push it down.",
                    GROWTH_MIN_HAPPINESS, GROWTH_MIN_ORDER
                ))
                .size(11.5)
                .color(theme::TEXT),
            );
            ui.label(
                RichText::new(
                    "A fort's garrison is fed from the settlement behind it: at most a fifth \
                     of the people who support it (8, 12, 16 or 20 spears, from village up to \
                     metropolis, by tier). When the village shrinks, loses the road, or is \
                     taken, the surplus marches home rather than draining the survivors.",
                )
                .size(11.5)
                .color(theme::TEXT),
            );

            theme::section(ui, "Workers & auto-employment");
            ui.label(
                RichText::new(
                    "Connected farms, woodcamps, mines, factories, markets and laboratories \
                     hire one novice every second (20 ticks) for free, as long as the village \
                     keeps at least 5 people. Manual training still buys higher starting skill \
                     (10/40/80) and workers improve by +1 skill every 30 ticks on the job.",
                )
                .size(11.5)
                .color(theme::TEXT),
            );

            theme::section(ui, "Fealty & integration");
            ui.label(
                RichText::new(
                    "Roads to a neutral village build fealty: +1.2 per food delivery and +2.5 \
                     for manufactured goods, while a rival's deliveries erode yours. At 25 the \
                     village becomes a Trading Partner and pays +3 gold per delivery; at 50 an \
                     Allied Protector; at 85 it joins your realm peacefully and brings its \
                     connected farm with it.",
                )
                .size(11.5)
                .color(theme::TEXT),
            );

            theme::section(ui, "War & intrigue");
            ui.label(
                RichText::new(
                    "Forts with a garrison fight and lay siege. Trade roads to rivals grant \
                     influence (and risk). Spend influence to foment dissent or buy a \
                     settlement outright. Agents infiltrate, siphon, sabotage or incite.",
                )
                .size(11.5)
                .color(theme::TEXT),
            );

            theme::section(ui, "Controls");
            egui::Grid::new("help_keys")
                .num_columns(2)
                .spacing(egui::vec2(16.0, 3.0))
                .striped(true)
                .show(ui, |ui| {
                    let rows = [
                        ("LMB", "select a node / place a building"),
                        ("RMB", "cancel tool or deselect"),
                        ("Drag \u{b7} MMB", "pan the map"),
                        ("Wheel", "zoom at the cursor"),
                        ("W A S D", "pan the map"),
                        ("Space", "pause / resume"),
                        ("1 \u{b7} 2 \u{b7} 3", "speed 1x / 2x / 4x"),
                        ("V \u{b7} R", "select tool / road tool"),
                        ("0", "quick-save to slot 1"),
                        ("H", "this help"),
                        ("Esc", "back / pause menu"),
                    ];
                    for (k, a) in rows {
                        ui.label(RichText::new(k).size(11.0).strong().color(theme::ACCENT));
                        ui.label(RichText::new(a).size(11.5).color(theme::DIM));
                        ui.end_row();
                    }
                });
        });
}

pub fn pause_menu(ctx: &egui::Context, w: &World) -> Vec<Action> {
    let mut acts = Vec::new();
    egui::Window::new("Paused")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(460.0);
            ui.label(
                RichText::new(format!(
                    "Tick {} \u{b7} {}g \u{b7} {} pop \u{b7} {}",
                    w.tick,
                    w.player().gold,
                    w.total_pop(Owner::Player),
                    w.player().age.label()
                ))
                .size(11.5)
                .color(theme::DIM),
            );
            ui.add_space(6.0);
            if ui
                .add(
                    egui::Button::new(RichText::new("Resume").strong())
                        .min_size(egui::vec2(110.0, 28.0)),
                )
                .clicked()
            {
                acts.push(Action::ToggleMenu);
            }

            theme::section(ui, "Saves");
            egui::Grid::new("save_grid")
                .num_columns(4)
                .spacing(egui::vec2(10.0, 4.0))
                .striped(true)
                .show(ui, |ui| {
                    for head in ["Slot", "Saved", "", ""] {
                        ui.label(RichText::new(head).size(10.0).color(theme::FAINT));
                    }
                    ui.end_row();
                    for slot in 0..4usize {
                        let name = if slot == 0 {
                            "Autosave".to_string()
                        } else {
                            format!("Slot {}", slot)
                        };
                        ui.label(RichText::new(name).size(11.5).color(theme::TEXT));
                        let summ = save::summary(slot);
                        match &summ {
                            Some(s) => ui.label(
                                RichText::new(format!(
                                    "t{} \u{b7} {}g \u{b7} {}",
                                    s.tick, s.gold, s.player_faction
                                ))
                                .size(10.5)
                                .color(theme::DIM),
                            ),
                            None => ui.label(
                                RichText::new("empty")
                                    .italics()
                                    .size(10.5)
                                    .color(theme::FAINT),
                            ),
                        };
                        if ui
                            .add_enabled(
                                slot != 0,
                                egui::Button::new("Save").min_size(egui::vec2(52.0, 20.0)),
                            )
                            .on_hover_text("Autosave is written every 600 ticks")
                            .clicked()
                        {
                            acts.push(Action::Save(slot));
                        }
                        if ui
                            .add_enabled(
                                summ.is_some(),
                                egui::Button::new("Load").min_size(egui::vec2(52.0, 20.0)),
                            )
                            .clicked()
                        {
                            acts.push(Action::Load(slot));
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(6.0);
            if ui
                .add(egui::Button::new("Quit to Main Menu").min_size(egui::vec2(0.0, 24.0)))
                .clicked()
            {
                acts.push(Action::QuitToMenu);
            }
        });
    acts
}

pub fn game_over(ctx: &egui::Context, w: &World) -> Vec<Action> {
    let mut acts = Vec::new();
    let Some(v) = w.victory else { return acts };
    let won = !matches!(v, Victory::Defeat);
    egui::Window::new(v.title())
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(420.0);
            ui.label(RichText::new(v.blurb()).italics().size(12.0).color(if won {
                theme::GOOD
            } else {
                theme::BAD
            }));
            theme::section(ui, "Chronicle of the war");
            egui::Grid::new("over_stats")
                .num_columns(2)
                .spacing(egui::vec2(16.0, 3.0))
                .show(ui, |ui| {
                    let rows = [
                        ("Ticks elapsed", format!("{}", w.tick)),
                        ("Battles won", format!("{}", w.stats.battles_won)),
                        (
                            "Settlements taken",
                            format!("{}", w.stats.settlements_taken),
                        ),
                        ("Agents born", format!("{}", w.stats.spies_born)),
                        ("Agents caught", format!("{}", w.stats.spies_caught)),
                        ("Gold earned", format!("{}", w.stats.gold_earned)),
                        ("Goods sold", format!("{}", w.stats.goods_sold)),
                    ];
                    for (k, val) in rows {
                        ui.label(RichText::new(k).size(11.0).color(theme::DIM));
                        ui.label(RichText::new(val).size(11.5).color(theme::TEXT));
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("New Game").strong())
                            .min_size(egui::vec2(120.0, 28.0)),
                    )
                    .clicked()
                {
                    acts.push(Action::NewGame);
                }
                if ui
                    .add(egui::Button::new("Main Menu").min_size(egui::vec2(120.0, 28.0)))
                    .clicked()
                {
                    acts.push(Action::QuitToMenu);
                }
            });
        });
    acts
}

// ---------------------------------------------------------------------------
// Main menu
// ---------------------------------------------------------------------------

pub struct MenuState {
    pub faction: Faction,
    pub difficulty: Difficulty,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            faction: Faction::SilkSpinners,
            difficulty: Difficulty::Medium,
        }
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self::new()
    }
}

pub enum MenuCmd {
    Start,
    Continue,
    Quit,
    Help,
    SetFaction(Faction),
    SetDifficulty(Difficulty),
}

pub fn main_menu(ui: &mut Ui, state: &mut MenuState) -> Option<MenuCmd> {
    let mut cmd = None;
    let avail = ui.available_size();

    ui.vertical_centered(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(avail.x.min(760.0), avail.y),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                ui.add_space((avail.y * 0.05).clamp(10.0, 60.0));
                ui.label(
                    RichText::new("NODE EMPIRE")
                        .size(46.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new("roads \u{b7} gold \u{b7} whispered words")
                        .italics()
                        .size(13.0)
                        .color(theme::DIM),
                );
                ui.add_space(16.0);

                theme::card(ui, |ui| {
                    ui.set_min_width(668.0);
                    theme::section(ui, "Choose your court");
                    egui::Grid::new("faction_grid")
                        .num_columns(3)
                        .spacing(egui::vec2(10.0, 10.0))
                        .show(ui, |ui| {
                            for (i, def) in FACTIONS.iter().enumerate() {
                                let selected = state.faction == def.id;
                                if faction_card(ui, def, selected).clicked() {
                                    cmd = Some(MenuCmd::SetFaction(def.id));
                                }
                                if i % 3 == 2 {
                                    ui.end_row();
                                }
                            }
                        });

                    ui.add_space(8.0);
                    theme::rule(ui);
                    ui.add_space(4.0);
                    let def = faction_def(state.faction);
                    ui.label(
                        RichText::new(def.name)
                            .size(14.0)
                            .strong()
                            .color(c32(def.color)),
                    );
                    ui.label(
                        RichText::new(def.blurb)
                            .italics()
                            .size(11.5)
                            .color(theme::DIM),
                    );
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        for (label, v, col) in [
                            ("Combat", def.combat, theme::BAD),
                            ("Production", def.production, theme::GOOD),
                            ("Spies", def.spy, theme::SPY),
                            ("Defense", def.defense, theme::ACCENT),
                            ("Trade", def.trade, theme::GOLD),
                            ("Growth", def.growth, theme::NEUTRAL),
                        ] {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(label).size(9.5).color(theme::FAINT));
                                theme::micro_bar(ui, (v / 1.6).clamp(0.0, 1.0), col, 74.0);
                                ui.label(
                                    RichText::new(format!("\u{d7}{:.2}", v))
                                        .size(9.5)
                                        .color(col),
                                );
                            });
                        }
                    });
                });

                ui.add_space(14.0);
                ui.label(
                    RichText::new("DIFFICULTY")
                        .size(9.5)
                        .strong()
                        .color(theme::FAINT),
                );
                let dsel = match state.difficulty {
                    Difficulty::Easy => 0,
                    Difficulty::Medium => 1,
                    Difficulty::Hard => 2,
                };
                if let Some(i) = theme::segmented(
                    ui,
                    &[
                        Difficulty::Easy.label(),
                        Difficulty::Medium.label(),
                        Difficulty::Hard.label(),
                    ],
                    dsel,
                    320.0,
                    30.0,
                ) {
                    let d = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard][i];
                    cmd = Some(MenuCmd::SetDifficulty(d));
                }

                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("New Game").size(15.0).strong())
                            .fill(theme::ACCENT.gamma_multiply(0.55))
                            .min_size(egui::vec2(240.0, 36.0)),
                    )
                    .clicked()
                {
                    cmd = Some(MenuCmd::Start);
                }

                ui.add_space(6.0);
                let slot = [0usize, 1, 2, 3].into_iter().find(|s| save::exists(*s));
                let summ = slot.and_then(save::summary);
                let cont_label = match &summ {
                    Some(s) => format!("Continue \u{b7} {} (t{})", s.player_faction, s.tick),
                    None => "Continue".to_string(),
                };
                if ui
                    .add_enabled(
                        summ.is_some(),
                        egui::Button::new(cont_label).min_size(egui::vec2(240.0, 28.0)),
                    )
                    .clicked()
                {
                    cmd = Some(MenuCmd::Continue);
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new("How to Play").min_size(egui::vec2(116.0, 26.0)))
                        .clicked()
                    {
                        cmd = Some(MenuCmd::Help);
                    }
                    if ui
                        .add(egui::Button::new("Quit").min_size(egui::vec2(116.0, 26.0)))
                        .clicked()
                    {
                        cmd = Some(MenuCmd::Quit);
                    }
                });

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Node Empire \u{b7} an engine of shapes and code")
                        .size(9.5)
                        .color(theme::FAINT),
                );
            },
        );
    });

    cmd
}

fn faction_card(ui: &mut Ui, def: &FactionDef, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(212.0, 88.0), Sense::click());
    let bg = if selected {
        theme::CARD_ACTIVE
    } else if resp.hovered() {
        theme::CARD_HOVER
    } else {
        theme::CARD
    };
    let p = ui.painter();
    p.rect_filled(rect, 9.0, bg);
    p.rect_stroke(
        rect,
        9.0,
        Stroke::new(
            if selected { 1.6 } else { 1.0 },
            if selected { theme::ACCENT } else { theme::LINE },
        ),
        StrokeKind::Inside,
    );
    // Identity stripe.
    p.rect_filled(
        Rect::from_min_max(
            Pos2::new(rect.left() + 1.0, rect.top() + 3.0),
            Pos2::new(rect.left() + 5.0, rect.bottom() - 3.0),
        ),
        2.0,
        c32(def.color),
    );
    p.text(
        Pos2::new(rect.left() + 14.0, rect.top() + 16.0),
        Align2::LEFT_CENTER,
        def.name,
        FontId::proportional(13.0),
        theme::TEXT,
    );
    p.text(
        Pos2::new(rect.left() + 14.0, rect.top() + 33.0),
        Align2::LEFT_CENTER,
        def.motto,
        FontId::proportional(10.0),
        theme::FAINT,
    );

    // Mini stat strip.
    let stats = [
        ("Combat", def.combat, theme::BAD),
        ("Prod", def.production, theme::GOOD),
        ("Spy", def.spy, theme::SPY),
        ("Def", def.defense, theme::ACCENT),
        ("Trade", def.trade, theme::GOLD),
    ];
    let mut x = rect.left() + 14.0;
    for (label, v, col) in stats {
        p.text(
            Pos2::new(x, rect.top() + 52.0),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(9.0),
            theme::FAINT,
        );
        let bar = Rect::from_min_size(Pos2::new(x, rect.top() + 60.0), Vec2::new(30.0, 4.0));
        p.rect_filled(bar, 2.0, theme::SUNKEN);
        p.rect_filled(
            Rect::from_min_size(bar.min, Vec2::new(30.0 * (v / 1.6).clamp(0.0, 1.0), 4.0)),
            2.0,
            col,
        );
        x += 36.0;
    }

    resp
}

// ---------------------------------------------------------------------------
// World text layer
//
// Bevy draws the shapes (see `render.rs`); egui paints every glyph on top, in
// screen space, so the two always agree as long as both use the same camera.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn map_labels(
    painter: &Painter,
    rect: Rect,
    world: &World,
    cam: &Camera,
    tool: Tool,
    cursor: Option<Vec2>,
    hover: Option<u32>,
    selected: Option<u32>,
    toast: Option<(&str, f32)>,
) {
    let zoom = cam.zoom;

    for n in world.nodes.values() {
        let center = cam.world_to_screen(rect, Vec2::new(n.x, n.y));
        let r = (n.radius() * zoom * 0.5).max(4.0);
        if !rect.expand(r + 60.0).contains(center) {
            continue;
        }

        // Inner details at readable zoom.
        if zoom >= 14.0 && r >= 9.0 {
            match n.kind {
                NodeType::Settlement | NodeType::Fort => {
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        n.population.to_string(),
                        FontId::proportional((r * 0.7).clamp(9.0, 15.0)),
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

        // Labels.
        let show_label = zoom >= 11.0 || selected == Some(n.id) || hover == Some(n.id);
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
        } else if zoom >= 16.0 && hover == Some(n.id) {
            painter.text(
                Pos2::new(center.x, center.y - r - 4.0),
                Align2::CENTER_BOTTOM,
                n.name.clone(),
                FontId::proportional(11.0),
                Color32::from_rgb(225, 225, 220),
            );
        }
    }

    // Tool overlays.
    match tool {
        Tool::Select => {}
        Tool::Build(kind) => {
            if let Some(cursor) = cursor {
                let tx = cursor.x.floor();
                let ty = cursor.y.floor();
                let snap = Vec2::new(tx + 0.5, ty + 0.5);
                if let Err(msg) = world.can_place(kind, snap.x, snap.y, Owner::Player) {
                    let p = cam.world_to_screen(rect, snap);
                    let ghost_r = (zoom
                        * if kind == NodeType::Settlement {
                            0.7
                        } else {
                            0.5
                        })
                    .max(6.0);
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
        Tool::Road { from } => match (from, cursor) {
            (Some(from_id), Some(cursor)) => {
                let Some(from_n) = world.node(from_id) else {
                    return;
                };
                let a = cam.world_to_screen(rect, Vec2::new(from_n.x, from_n.y));
                let b = cam.world_to_screen(rect, cursor);
                let target = hover.filter(|&h| h != from_id);
                let (color, cost) = match target {
                    Some(tid) => {
                        let kind = world.classify_edge(from_id, tid);
                        let c = match kind {
                            Some(EdgeType::Resource) => Color32::from_rgb(208, 178, 118),
                            Some(EdgeType::Supply) => Color32::from_rgb(112, 192, 132),
                            Some(EdgeType::Combat) => Color32::from_rgb(222, 82, 72),
                            Some(EdgeType::Siege) => Color32::from_rgb(238, 152, 58),
                            Some(EdgeType::Trade) => Color32::from_rgb(238, 218, 108),
                            None => Color32::from_rgb(255, 120, 110),
                        };
                        (c, world.road_cost(from_id, tid))
                    }
                    None => (Color32::from_rgb(240, 240, 230), None),
                };
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
                        color,
                    );
                }
            }
            (None, _) => {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Click a node to start the road (Esc cancels)",
                    FontId::proportional(14.0),
                    Color32::from_rgb(255, 240, 180),
                );
            }
            _ => {}
        },
    }

    // Command feedback, as a pill above the bottom edge.
    if let Some((msg, alpha)) = toast {
        toast_pill(painter, rect, msg, alpha);
    }
}

fn toast_pill(painter: &Painter, rect: Rect, msg: &str, alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    if a <= 0.01 {
        return;
    }
    let galley = painter.layout_no_wrap(msg.to_owned(), FontId::proportional(13.5), theme::TEXT);
    let size = galley.size() + Vec2::new(34.0, 14.0);
    let center = Pos2::new(rect.center().x, rect.bottom() - 20.0 - size.y * 0.5);
    let pill = Rect::from_center_size(center, size);
    painter.rect_filled(
        pill,
        999.0,
        Color32::from_rgba_unmultiplied(13, 15, 20, (235.0 * a) as u8),
    );
    painter.rect_stroke(
        pill,
        999.0,
        Stroke::new(1.0, theme::ACCENT.gamma_multiply(0.75 * a)),
        StrokeKind::Inside,
    );
    painter.circle_filled(
        Pos2::new(pill.left() + 14.0, pill.center().y),
        3.5,
        theme::ACCENT.gamma_multiply(a),
    );
    painter.galley(
        Pos2::new(pill.left() + 26.0, pill.center().y - galley.size().y * 0.5),
        galley,
        theme::TEXT.gamma_multiply(a),
    );
}
