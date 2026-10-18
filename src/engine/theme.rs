//! The one dark RTS look: palette, egui style, and the small widgets every
//! panel is built from (cards, section rules, meters, segmented controls,
//! chips, swatches).
//!
//! Nothing here knows about the game — it is pure presentation, so the HUD and
//! the menus stay consistent by construction.

use bevy_egui::egui::{
    self, Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

/// Map surround, behind the world.
pub const WINDOW: Color32 = Color32::from_rgb(13, 15, 19);
/// Panel chrome.
pub const PANEL: Color32 = Color32::from_rgb(20, 22, 28);
/// Inset wells (segment groups, meters, scrolling lists).
pub const SUNKEN: Color32 = Color32::from_rgb(15, 17, 21);
/// Raised cards.
pub const CARD: Color32 = Color32::from_rgb(31, 35, 43);
pub const CARD_HOVER: Color32 = Color32::from_rgb(41, 46, 57);
pub const CARD_ACTIVE: Color32 = Color32::from_rgb(36, 58, 88);
pub const LINE: Color32 = Color32::from_rgb(45, 50, 62);

pub const TEXT: Color32 = Color32::from_rgb(228, 232, 240);
pub const DIM: Color32 = Color32::from_rgb(140, 148, 162);
pub const FAINT: Color32 = Color32::from_rgb(96, 103, 118);

pub const ACCENT: Color32 = Color32::from_rgb(98, 176, 255);
pub const GOLD: Color32 = Color32::from_rgb(255, 206, 100);
pub const GOOD: Color32 = Color32::from_rgb(118, 210, 142);
pub const BAD: Color32 = Color32::from_rgb(236, 104, 94);
pub const WARN: Color32 = Color32::from_rgb(240, 168, 72);
pub const SPY: Color32 = Color32::from_rgb(200, 150, 255);
pub const NEUTRAL: Color32 = Color32::from_rgb(184, 190, 200);

// ---------------------------------------------------------------------------
// egui style
// ---------------------------------------------------------------------------

/// Apply the Node Empire style to the egui context. Cheap and idempotent.
pub fn set_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.visuals = visuals();
        style.spacing.item_spacing = egui::vec2(7.0, 6.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.spacing.interact_size.y = 22.0;
        style.spacing.slider_width = 120.0;
    });
}

fn visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.window_corner_radius = CornerRadius::same(10);
    v.extreme_bg_color = SUNKEN;
    v.faint_bg_color = SUNKEN;
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(160),
    };
    v.popup_shadow = v.window_shadow;

    let r = CornerRadius::same(6);
    let w = &mut v.widgets;
    w.noninteractive.corner_radius = r;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, DIM);

    w.inactive.corner_radius = r;
    w.inactive.bg_fill = CARD;
    w.inactive.weak_bg_fill = CARD;
    w.inactive.bg_stroke = Stroke::new(1.0, LINE);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);

    w.hovered.corner_radius = r;
    w.hovered.bg_fill = CARD_HOVER;
    w.hovered.weak_bg_fill = CARD_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));
    w.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    w.active.corner_radius = r;
    w.active.bg_fill = CARD_ACTIVE;
    w.active.weak_bg_fill = CARD_ACTIVE;
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
    w.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    w.open.corner_radius = r;
    w.open.bg_fill = CARD_HOVER;
    w.open.weak_bg_fill = CARD_HOVER;
    w.open.bg_stroke = Stroke::new(1.0, LINE);
    w.open.fg_stroke = Stroke::new(1.0, TEXT);

    v
}

// ---------------------------------------------------------------------------
// Widgets
// ---------------------------------------------------------------------------

/// A raised card to group related controls.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::default()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(9))
        .show(ui, add)
}

/// A small-caps heading with a hairline rule under it.
pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(3.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.0)
            .color(DIM)
            .strong(),
    );
    rule(ui);
    ui.add_space(2.0);
}

/// A full-width hairline divider.
pub fn rule(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, LINE);
}

/// A slim labelled progress row: `Label ▓▓▓░░ value`.
pub fn meter_row(
    ui: &mut Ui,
    label: &str,
    value: &str,
    frac: f32,
    color: Color32,
    width: f32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 15.0), Sense::hover());
    let p = ui.painter();
    let label_w = 62.0;
    let value_w = 62.0;
    let bar = Rect::from_min_max(
        Pos2::new(rect.left() + label_w, rect.top() + 4.0),
        Pos2::new(rect.right() - value_w, rect.bottom() - 4.0),
    );
    p.text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        DIM,
    );
    p.rect_filled(bar, 3.0, SUNKEN);
    let inner = bar.shrink(1.0);
    let filled = Rect::from_min_size(
        inner.min,
        Vec2::new(inner.width() * frac.clamp(0.0, 1.0), inner.height()),
    );
    p.rect_filled(filled, 2.0, color.gamma_multiply(0.9));
    p.rect_stroke(bar, 3.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
    p.text(
        rect.right_center(),
        Align2::RIGHT_CENTER,
        value,
        FontId::proportional(11.0),
        color,
    );
}

/// A segmented control. Returns the index the user picked this frame.
pub fn segmented(
    ui: &mut Ui,
    items: &[&str],
    selected: usize,
    width: f32,
    height: f32,
) -> Option<usize> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let n = items.len().max(1);
    let seg = rect.width() / n as f32;
    let hover_pos = resp.hover_pos();

    let mut picked = None;
    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let i = ((pos.x - rect.left()) / seg).floor();
            picked = Some(i.clamp(0.0, n as f32 - 1.0) as usize);
        }
    }

    let p = ui.painter();
    p.rect_filled(rect, 7.0, SUNKEN);
    p.rect_stroke(rect, 7.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
    for (i, item) in items.iter().enumerate() {
        let cell = Rect::from_min_size(
            Pos2::new(rect.left() + seg * i as f32, rect.top()),
            Vec2::new(seg, rect.height()),
        )
        .shrink2(Vec2::new(2.0, 2.0));
        let is_sel = i == selected;
        let hovered = hover_pos.map(|q| cell.contains(q)).unwrap_or(false);
        if is_sel {
            p.rect_filled(cell, 5.0, CARD_ACTIVE);
            p.rect_stroke(cell, 5.0, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
        } else if hovered {
            p.rect_filled(cell, 5.0, CARD_HOVER);
        }
        p.text(
            cell.center(),
            Align2::CENTER_CENTER,
            *item,
            FontId::proportional(11.5),
            if is_sel { TEXT } else { DIM },
        );
    }
    picked
}

/// A rounded label pill: `[ text ]` tinted by `color`.
pub fn chip(ui: &mut Ui, text: &str, color: Color32) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(11.0), color);
    let size = galley.size() + Vec2::new(14.0, 7.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 999.0, color.gamma_multiply(0.14));
    p.rect_stroke(
        rect,
        999.0,
        Stroke::new(1.0, color.gamma_multiply(0.40)),
        StrokeKind::Inside,
    );
    p.galley(
        rect.center() - galley.size() * 0.5,
        galley,
        color.gamma_multiply(1.0),
    );
    resp
}

/// A small colour swatch, for faction identity.
pub fn swatch(ui: &mut Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, color);
    p.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_black_alpha(90)),
        StrokeKind::Inside,
    );
}

/// A thin horizontal bar in a mini stat row (no labels).
pub fn micro_bar(ui: &mut Ui, frac: f32, color: Color32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 5.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, SUNKEN);
    let filled = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * frac.clamp(0.0, 1.0), rect.height()),
    );
    p.rect_filled(filled, 2.0, color);
}
