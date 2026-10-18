//! HUD panels and overlays. Every panel draws from immutable state and
//! returns `Action`s that the App applies afterwards — this keeps the
//! borrow checker happy and separates "what happened" from "how it looks".

use super::app::Tool;
use super::render::rgb;
use crate::game::save;
use crate::game::types::*;
use crate::game::world::{EventKind, World};
use eframe::egui::{self, Color32, RichText, Ui};

// ---------------------------------------------------------------------------
// UI → App communication
// ---------------------------------------------------------------------------

pub enum Action {
    SetTool(Tool),
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
    pub selected: Option<u32>,
    pub speed: u8,
}

// ---------------------------------------------------------------------------
// Top bar
// ---------------------------------------------------------------------------

pub fn top_bar(ui: &mut Ui, h: &Hud) -> Vec<Action> {
    let mut acts = Vec::new();
    let w = h.world;

    ui.horizontal_wrapped(|ui| {
        let fac = faction_def(w.player_faction);
        ui.colored_label(rgb(fac.color), RichText::new(fac.name).strong());

        ui.separator();
        let led = w.player();
        ui.label(RichText::new(format!("{}g", led.gold)).size(16.0).strong());
        let inc = led.income_shown;
        let txt = if inc >= 0.0 {
            format!("+{:.1}/t", inc)
        } else {
            format!("{:.1}/t", inc)
        };
        ui.colored_label(
            if inc >= 0.0 { Color32::LIGHT_GREEN } else { Color32::LIGHT_RED },
            txt,
        );

        ui.separator();
        let st = w.total_stock(Owner::Player);
        ui.label(format!(
            "Food {} · Timber {} · Ore {} · Goods {} · Pop {}",
            st.food,
            st.timber,
            st.ore,
            st.goods,
            w.total_pop(Owner::Player)
        ));

        ui.separator();
        ui.label(format!("{} · {} RP", led.age.label(), led.tech));
        if let Some(cost) = led.age.cost_to_advance() {
            let can = led.tech >= cost;
            if ui
                .add_enabled(can, egui::Button::new(format!("Advance ({} RP)", cost)))
                .on_hover_text("Advance to the next age")
                .clicked()
            {
                acts.push(Action::AdvanceAge);
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Menu").clicked() {
                acts.push(Action::ToggleMenu);
            }
            if ui.button("Help (H)").clicked() {
                acts.push(Action::ToggleHelp);
            }
            if ui.selectable_label(h.speed == 0, "Pause").clicked() {
                acts.push(Action::SetSpeed(0));
            }
            for s in [1u8, 2, 4] {
                if ui.selectable_label(h.speed == s, format!("{}\u{d7}", s)).clicked() {
                    acts.push(Action::SetSpeed(s));
                }
            }
            ui.label(format!("Tick {}", w.tick));
        });
    });

    ui.horizontal(|ui| {
        let wealth = (w.player().gold.max(0) as f32 / w.win_gold as f32).clamp(0.0, 1.0);
        ui.add(
            egui::ProgressBar::new(wealth)
                .desired_width(170.0)
                .text(format!("Wealth {}/{}", w.player().gold, w.win_gold)),
        );
        let (bound, total) = w.web_progress();
        let web = if total > 0 { bound as f32 / total as f32 } else { 0.0 };
        ui.add(
            egui::ProgressBar::new(web)
                .desired_width(150.0)
                .text(format!("Web {}/{}", bound, total)),
        );
        let p = w.settlements_of(Owner::Player).len();
        let e = w.settlements_of(Owner::Enemy).len();
        let dom = if p + e > 0 { p as f32 / (p + e) as f32 } else { 0.0 };
        ui.add(
            egui::ProgressBar::new(dom)
                .desired_width(160.0)
                .text(format!("Domination {}/{}", p, p + e)),
        );
        ui.label(format!(
            "Rival: {}",
            faction_def(w.enemy_faction).name
        ));
    });

    acts
}

// ---------------------------------------------------------------------------
// Left panel: tools + build palette
// ---------------------------------------------------------------------------

pub fn left_panel(ui: &mut Ui, h: &Hud) -> Vec<Action> {
    let mut acts = Vec::new();
    let w = h.world;

    ui.heading("Tools");
    ui.horizontal(|ui| {
        if ui
            .selectable_label(matches!(h.tool, Tool::Select), "Select (V)")
            .clicked()
        {
            acts.push(Action::SetTool(Tool::Select));
        }
        if ui
            .selectable_label(matches!(h.tool, Tool::Road { .. }), "Road (R)")
            .clicked()
        {
            acts.push(Action::SetTool(Tool::Road { from: None }));
        }
    });

    ui.separator();
    ui.heading("Build");
    let gold = w.player().gold;
    for (kind, cost) in BUILD_COSTS {
        let active = matches!(h.tool, Tool::Build(k) if k == kind);
        let label = format!("{} — {}g", kind.label(), cost);
        if ui
            .add_enabled(gold >= cost, egui::Button::selectable(active, label))
            .clicked()
        {
            acts.push(Action::SetTool(Tool::Build(kind)));
        }
    }

    ui.separator();
    let hint = match h.tool {
        Tool::Select => "Click a node to inspect it.\nRight-click deselects.",
        Tool::Build(NodeType::Mine) => "Click grass bearing orange (copper)\nor silver (iron) specks.",
        Tool::Build(NodeType::Woodcamp) => "Click inside dark-green forest.",
        Tool::Build(NodeType::Settlement) => "Click open ground (not forest,\nnot water or mountains).",
        Tool::Build(_) => "Click the ground near your network.",
        Tool::Road { from: None } => "Click the first node\u{2026}",
        Tool::Road { from: Some(_) } => "\u{2026}then the second node.\nEsc cancels.",
    };
    ui.label(RichText::new(hint).italics().color(Color32::LIGHT_GRAY));

    acts
}

// ---------------------------------------------------------------------------
// Right panel: inspector + agents
// ---------------------------------------------------------------------------

pub fn right_panel(ui: &mut Ui, h: &Hud) -> Vec<Action> {
    let mut acts = Vec::new();
    let w = h.world;

    egui::ScrollArea::vertical().show(ui, |ui| {
        match h.selected.and_then(|id| w.node(id)) {
            None => overview(ui, w),
            Some(n) => inspector(ui, w, n, &mut acts),
        }
        ui.separator();
        agents(ui, w, &mut acts);
    });

    acts
}

fn overview(ui: &mut Ui, w: &World) {
    ui.heading("Your Realm");
    ui.separator();
    for n in w.settlements_of(Owner::Player) {
        ui.label(format!("{} — pop {}", n.name, n.population));
    }
    ui.separator();
    ui.label(RichText::new("Victory").strong());
    ui.label(w.victory_progress());
    ui.label(
        RichText::new(
            "Wealth: hoard gold.\nWeb: an Infiltrate agent + influence 70 in every rival settlement.\nDomination: hold every enemy settlement.",
        )
        .italics()
        .color(Color32::LIGHT_GRAY),
    );
}

fn inspector(ui: &mut Ui, w: &World, n: &crate::game::world::Node, acts: &mut Vec<Action>) {
    ui.heading(n.name.clone());
    let owner_txt = match n.owner {
        Owner::Player => "Yours".to_string(),
        Owner::Enemy => format!(
            "Enemy ({})",
            n.faction.map(|f| faction_def(f).name).unwrap_or("?")
        ),
        Owner::Neutral => "Neutral".to_string(),
        Owner::Bandit => "Bandits".to_string(),
    };
    ui.label(format!("{} \u{b7} {}", n.kind.label(), owner_txt));

    if n.is_settlement() {
        let tier = ["Village", "Town", "City", "Metropolis"][n.tier.min(3) as usize];
        ui.label(format!(
            "Pop {} \u{b7} {} \u{b7} Happiness {:.0}% \u{b7} Order {:.0}% \u{b7} Loyalty {}",
            n.population, tier, n.happiness, n.public_order, n.loyalty
        ));
        ui.label(format!(
            "Local stock: food {} \u{b7} ore {} \u{b7} timber {} \u{b7} goods {}",
            n.stock.food, n.stock.ore, n.stock.timber, n.stock.goods
        ));
        if n.besieged {
            ui.colored_label(Color32::LIGHT_RED, "Under siege!");
        }
        if n.sabotage > 0 {
            ui.colored_label(Color32::from_rgb(200, 150, 255), "Sabotaged production!");
        }
    } else {
        ui.label(format!(
            "Workers {} / {} slots \u{b7} Garrison {}",
            n.worker_count(),
            n.job_slots(),
            n.population
        ));
    }

    // Foreign settlements: influence game.
    if n.is_settlement() && n.owner != Owner::Player {
        let inf = w.influence(Owner::Player, n.id);
        ui.add(
            egui::ProgressBar::new((inf / 100.0).clamp(0.0, 1.0))
                .desired_width(180.0)
                .text(format!("Influence {:.0}/100", inf)),
        );
        let gold = w.player().gold;
        if ui
            .add_enabled(
                inf >= 61.0 && gold >= 100,
                egui::Button::new("Foment dissent (100g)"),
            )
            .on_hover_text("Stir unrest: halves loyalty and sparks a crisis")
            .clicked()
        {
            acts.push(Action::Foment(n.id));
        }
        if ui
            .add_enabled(
                inf >= 81.0 && gold >= 200,
                egui::Button::new("Buy allegiance (200g)"),
            )
            .clicked()
        {
            acts.push(Action::BuySettlement(n.id));
        }
        if ui
            .add_enabled(
                w.player().gold >= SPY_TRAIN_COST,
                egui::Button::new(format!("Send spy ({}g)", SPY_TRAIN_COST)),
            )
            .on_hover_text("Recruit an agent at home; slip it into this court")
            .clicked()
        {
            acts.push(Action::TrainSpy(n.id));
        }
    }

    // Own production: crew management.
    let trainable =
        n.owner == Owner::Player && (n.is_production() || n.kind == NodeType::Fort || n.kind == NodeType::Laboratory);
    if trainable {
        ui.separator();
        ui.label(RichText::new("Crew").strong());
        for (i, wk) in n.workers.iter().enumerate() {
            ui.horizontal(|ui| {
                let train = if wk.ready() {
                    String::new()
                } else {
                    format!(" ({}t)", wk.training_left)
                };
                ui.label(format!(
                    "{} \u{b7} {} \u{b7} skill {}{}",
                    wk.name,
                    wk.job.label(),
                    wk.skill,
                    train
                ));
                if ui.small_button(format!("Dismiss##dis{}", i)).clicked() {
                    acts.push(Action::DismissWorker(n.id, i));
                }
            });
        }
        ui.horizontal(|ui| {
            let names = ["Novice", "Adept", "Master"];
            for (t, cost) in TRAIN_COSTS.iter().enumerate() {
                let gold = w.player().gold;
                if ui
                    .add_enabled(
                        gold >= *cost,
                        egui::Button::new(format!("{} ({}g)", names[t], cost)),
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
    ui.separator();
    ui.label(RichText::new("Roads").strong());
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
                .color(Color32::LIGHT_GRAY),
        );
    }
    for (idx, other, kind, by, caravans) in edges {
        let oname = w
            .node(other)
            .map(|x| x.name.clone())
            .unwrap_or_else(|| "?".to_string());
        ui.horizontal(|ui| {
            ui.label(format!("{} \u{b7} {}", oname, kind.label()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if by == Owner::Player
                    && ui.small_button(format!("Cut##cut{}", idx)).clicked()
                {
                    acts.push(Action::CutRoad(idx));
                }
                if by == Owner::Player
                    && matches!(kind, EdgeType::Resource | EdgeType::Trade)
                {
                    if ui.small_button(format!("-##m{}", idx)).clicked() {
                        acts.push(Action::CaravanDelta(idx, -1));
                    }
                    ui.label(format!("{}", caravans));
                    if ui.small_button(format!("+##p{}", idx)).clicked() {
                        acts.push(Action::CaravanDelta(idx, 1));
                    }
                }
            });
        });
    }

    // Demolish.
    if n.owner == Owner::Player {
        ui.separator();
        if ui.button("Demolish").clicked() {
            acts.push(Action::Demolish(n.id));
        }
    }
}

fn agents(ui: &mut Ui, w: &World, acts: &mut Vec<Action>) {
    ui.heading("Agents");
    let mine: Vec<&Spy> = w.spies.iter().filter(|s| s.owner == Owner::Player).collect();
    if mine.is_empty() {
        ui.label(
            RichText::new("No agents. Select a foreign settlement to recruit one.")
                .italics()
                .color(Color32::LIGHT_GRAY),
        );
        return;
    }
    for s in mine {
        let tname = w
            .node(s.target)
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let st = match s.status {
            SpyStatus::Traveling => "traveling",
            SpyStatus::Embedded => "embedded",
            SpyStatus::Returning => "returning",
        };
        ui.horizontal(|ui| {
            ui.label(format!("{} \u{2192} {} ({}, sk {})", s.name, tname, st, s.skill));
            if ui.small_button(format!("Recall##r{}", s.id)).clicked() {
                acts.push(Action::RecallSpy(s.id));
            }
        });
        ui.horizontal(|ui| {
            for op in [SpyOp::Infiltrate, SpyOp::Siphon, SpyOp::Sabotage, SpyOp::Incite] {
                let sel = s.op == op;
                if ui
                    .selectable_label(sel, op.label())
                    .on_hover_text(op.hint())
                    .clicked()
                {
                    acts.push(Action::SetSpyOp(s.id, op));
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Bottom panel: chronicle
// ---------------------------------------------------------------------------

pub fn log_panel(ui: &mut Ui, w: &World) {
    ui.label(RichText::new("Chronicle").strong());
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .max_height(86.0)
        .show(ui, |ui| {
            for ev in w.log.iter().rev().take(40).rev() {
                let c = match ev.kind {
                    EventKind::Good => Color32::LIGHT_GREEN,
                    EventKind::Bad => Color32::LIGHT_RED,
                    EventKind::War => Color32::from_rgb(255, 170, 90),
                    EventKind::Spy => Color32::from_rgb(205, 155, 255),
                    EventKind::Neutral => Color32::LIGHT_GRAY,
                };
                ui.colored_label(c, format!("[{}] {}", ev.tick, ev.text));
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
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                "Win by Wealth (hold the gold threshold), by Web (an Infiltrate agent \
                 plus 70 influence in every rival settlement), or by Domination \
                 (hold every enemy settlement). You lose when your last settlement falls.",
            );
            ui.separator();
            ui.label(
                "Economy: Farms feed settlements; Mines need ore specks; Woodcamps need \
                 forest; Factories refine ore+timber into goods; Markets sell goods; \
                 Laboratories generate research. Roads ship goods between nodes \u{2014} \
                 add caravans to speed the flow.",
            );
            ui.separator();
            ui.label(
                "War & intrigue: Forts with a garrison fight and siege. Trade routes to \
                 rivals grant influence (and risk). Spend influence to foment dissent or \
                 buy a settlement outright. Spies infiltrate, siphon, sabotage or incite.",
            );
            ui.separator();
            ui.label(
                RichText::new(
                    "Controls: LMB select/place \u{b7} RMB cancel \u{b7} drag or MMB pan \u{b7} \
                     wheel zoom \u{b7} WASD pan \u{b7} Space pause \u{b7} 1/2/3 speed \u{b7} \
                     V select \u{b7} R road \u{b7} H help \u{b7} Esc menu",
                )
                .italics(),
            );
        });
}

pub fn pause_menu(ctx: &egui::Context, w: &World) -> Vec<Action> {
    let mut acts = Vec::new();
    egui::Window::new("Paused")
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("Tick {} \u{b7} {}g", w.tick, w.player().gold));
            ui.separator();
            if ui.button("Resume").clicked() {
                acts.push(Action::ToggleMenu);
            }
            ui.separator();
            ui.label("Save to slot:");
            ui.horizontal(|ui| {
                for slot in 1..=3usize {
                    if ui.button(format!("{}", slot)).clicked() {
                        acts.push(Action::Save(slot));
                    }
                }
            });
            ui.label("Load slot:");
            ui.horizontal(|ui| {
                for slot in 1..=3usize {
                    let summ = save::summary(slot);
                    let txt = match &summ {
                        Some(s) => format!("{} (t{})", slot, s.tick),
                        None => format!("{} (empty)", slot),
                    };
                    if ui.add_enabled(summ.is_some(), egui::Button::new(txt)).clicked() {
                        acts.push(Action::Load(slot));
                    }
                }
            });
            if save::exists(0) {
                if ui.button("Load autosave").clicked() {
                    acts.push(Action::Load(0));
                }
            }
            ui.separator();
            if ui.button("Quit to Main Menu").clicked() {
                acts.push(Action::QuitToMenu);
            }
        });
    acts
}

pub fn game_over(ctx: &egui::Context, w: &World) -> Vec<Action> {
    let mut acts = Vec::new();
    let Some(v) = w.victory else { return acts };
    egui::Window::new(v.title())
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(RichText::new(v.blurb()).italics());
            ui.separator();
            ui.label(format!("Ticks elapsed: {}", w.tick));
            ui.label(format!(
                "Battles won: {} \u{b7} Settlements taken: {}",
                w.stats.battles_won, w.stats.settlements_taken
            ));
            ui.label(format!(
                "Agents born: {} \u{b7} Caught: {} \u{b7} Gold earned: {}",
                w.stats.spies_born, w.stats.spies_caught, w.stats.gold_earned
            ));
            ui.separator();
            if ui.button("New Game").clicked() {
                acts.push(Action::NewGame);
            }
            if ui.button("Main Menu").clicked() {
                acts.push(Action::QuitToMenu);
            }
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

    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(RichText::new("NODE EMPIRE").size(44.0).strong());
        ui.label(
            RichText::new("a network-architect strategy of roads, gold and whispered words")
                .italics()
                .size(14.0)
                .color(Color32::LIGHT_GRAY),
        );
        ui.add_space(22.0);

        ui.label(RichText::new("Choose your court").size(16.0).strong());
        ui.add_space(4.0);
        for def in FACTIONS.iter() {
            let selected = state.faction == def.id;
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, rgb(def.color));
                if ui
                    .selectable_label(selected, format!("{} \u{2014} \u{201c}{}\u{201d}", def.name, def.motto))
                    .clicked()
                {
                    cmd = Some(MenuCmd::SetFaction(def.id));
                }
            });
        }

        ui.add_space(10.0);
        let def = faction_def(state.faction);
        ui.label(RichText::new(def.blurb).italics());
        ui.label(format!(
            "Combat \u{d7}{:.2} \u{b7} Production \u{d7}{:.2} \u{b7} Spies \u{d7}{:.2} \u{b7} Defense \u{d7}{:.2} \u{b7} Trade \u{d7}{:.2}",
            def.combat, def.production, def.spy, def.defense, def.trade
        ));

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label("Difficulty:");
            for d in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
                if ui
                    .selectable_label(state.difficulty == d, d.label())
                    .clicked()
                {
                    cmd = Some(MenuCmd::SetDifficulty(d));
                }
            }
        });

        ui.add_space(16.0);
        if ui
            .add(egui::Button::new("New Game").min_size(egui::vec2(180.0, 30.0)))
            .clicked()
        {
            cmd = Some(MenuCmd::Start);
        }
        ui.add_space(4.0);
        let has_save = save::exists(0) || save::exists(1) || save::exists(2) || save::exists(3);
        if ui
            .add_enabled(has_save, egui::Button::new("Continue (autosave)").min_size(egui::vec2(180.0, 26.0)))
            .clicked()
        {
            cmd = Some(MenuCmd::Continue);
        }
        ui.add_space(4.0);
        if ui.button("How to Play").clicked() {
            cmd = Some(MenuCmd::Help);
        }
        ui.add_space(4.0);
        if ui.button("Quit").clicked() {
            cmd = Some(MenuCmd::Quit);
        }
    });

    cmd
}
