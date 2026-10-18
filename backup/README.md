# Archived / Discarded Code

Snapshots kept from earlier rounds of development. Nothing in this folder is
referenced by the current build; it is here so the reasoning behind each
rewrite stays readable.

## eframe_engine/

The whole presentation layer as it stood before the Bevy 0.20 migration: a
custom immediate-mode engine on `eframe`/`egui`, where egui both drew the HUD
and painted the map (baked terrain texture plus shapes) and kept the camera by
hand. Superseded by `src/engine/` — see `../ARCHITECTURE.md`. The game layer
(`src/game/`) crossed the migration unchanged, which is what that design was
for.

| File | What it was |
|------|-------------|
| `app.rs` | `eframe::App` implementation: event handling, hotkeys, action application, toast/status handling, screen switching |
| `camera.rs` | Tile↔screen transforms, cursor-anchored zoom and pan for the egui canvas |
| `render.rs` | Map rendering: baked terrain texture, node shapes, roads, deposit specks, spies, build ghosts, selection rings |
| `ui.rs` | The HUD, panels, main menu, pause and game-over overlays, world text |
| `mod.rs` | Module wiring and re-exports (`run`, `Screen`, `Tool`) |
| `main_eframe.rs` | The old `src/main.rs` entry point (window setup, 1400x900, dark theme) |
| `Cargo_eframe.toml` | The old manifest: `eframe`/`egui` 0.32, no Bevy |

Early versions of that engine had no `theme.rs` or `state.rs` — the palette was
inlined in `ui.rs` and the shared state lived in the `App` struct. Both were
split out only in the final months, so the archive keeps `ui.rs` as one file.

## legacy_src/

| File | What it was |
|------|-------------|
| `main_v1_node_empire.rs` | The original monolithic 1,468-line single-file game (all logic + UI + rendering in one `main.rs`) |
| `main_backup.rs` | Mid-session backup copy of v1 (near-identical, 59 KB) |
| `main_broken.rs` | Snapshot taken while debugging a compile error |
| `main_fixed.rs` | Snapshot after the fix |
| `main_new.rs` | Earlier snapshot from the same session |
| `main_test.rs` | Duplicate of the fixed snapshot |

These accumulated during iterative development in April 2026, before the game
layer was split out into `src/game/`.

## legacy_saves/

Old JSON save files (v1 schema). Incompatible with the rewritten game state
schema — kept for reference only.

## legacy_docs/

| File | What it was |
|------|-------------|
| `plan_godot_bevy.md` | The original Godot 4 + Bevy hybrid engine plan, superseded by the eframe/egui custom-engine architecture, which in turn gave way to Bevy 0.20 |
