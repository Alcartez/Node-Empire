# Node Empire — Architecture

A Seven-Kingdoms-like realtime strategy game built entirely from code: zero art
assets, every visual is a shape or a procedurally generated texture. The one
non-code file is a WESL shader (`assets/shaders/terrain.wesl`), which is source
code that happens to live in the asset folder — it is embedded into the binary
with `include_str!` and there is no runtime asset lookup.

## Layers

```
src/
  main.rs        windowed entry point -> engine::app::run()
  lib.rs         pub mod game; pub mod engine;
  game/          pure simulation — no Bevy imports, fully serializable
    types.rs     enums, constants, factions, Node/Edge/Spy value types
    map.rs       procedural map (56x44), terrain/deposits, A* pathfinding
    world.rs     World state + all player/AI commands and queries
    sim.rs       the tick pipeline (see below)
    ai.rs        enemy decision making
    save.rs      save/load slots (slot 0 = autosave every 600 ticks)
  engine/        presentation — reads game/, never mutates it behind its back
    app.rs       the Bevy App: plugins, schedules, input, action application
    state.rs     shared resources (GameState, CamView, WorldVis, Mats) + Tool
    camera.rs    tile<->screen transforms, pan, cursor-anchored zoom
    render.rs    terrain quad, Mesh2d fills, gizmo strokes, world sync
    terrain.rs   the custom WESL terrain material (ColorMaterial extension)
    theme.rs     palette + generic egui widgets (cards, meters, segments)
    ui.rs        HUD panels, overlays, main menu; emits Action enums
    errors.rs    the fallback error handler and the toast queue
tests/
  sim_stress.rs  headless simulation tests (no window, no Bevy)
  economy.rs     order, employment, growth gates and garrisons
  spawn.rs       new-game spawn guarantees, checked over 200 seeds
assets/
  shaders/terrain.wesl   the only asset: terrain shimmer, compiled in
```

The game layer is engine-agnostic: `cargo test` runs the whole simulation
without a window, and saves are plain JSON (`serde`) that survive engine
changes. The engine may be replaced wholesale (it was, twice: `backup/`) without
touching `game/`.

## Engine: Bevy 0.20

Bevy owns the window, the 2D camera, the fixed-timestep clock, the state
machine and world rendering. egui (via `bevy_egui` 0.43) owns every glyph.

### App wiring (`app.rs`)

- Plugins: `DefaultPlugins`, `EguiPlugin`, `terrain::TerrainPlugin`,
  `errors::ErrorPlugin`.
- `Screen { Menu, Game }` is a Bevy `States`. Game over and the pause menu are
  overlays driven by `world.victory` / `GameState.show_menu`, not states.
- Simulation runs in `FixedUpdate` at `Time<Fixed>::from_seconds(TICK_DT)`
  (5 Hz). Speed 0/1/2/4 is `Time<Virtual>` pause / relative speed, so pause and
  fast-forward need no changes in the game layer.
- The HUD runs in `EguiPrimaryContextPass`; world visuals sync in `PostUpdate`
  **after** the egui pass (the panels measure the map viewport) and **before**
  Bevy's camera/visibility systems:

  ```
  camera_follow, sync_world, draw_strokes
      .after(EguiPostUpdateSet::EndPass)
      .after(TransformSystems::Propagate)
      .before(CameraUpdateSystems)
      .before(VisibilitySystems::VisibilityPropagate)
  ```

  `camera_follow` writes the camera transform and projection every frame, so
  Bevy's `camera_system` (which turns the projection into the clip matrix) must
  run after it, or the map lags the egui labels by a frame.
- `tighten_schedules` (debug builds) builds every schedule with a `shuffle_seed`,
  which randomises system order while respecting constraints. That is how the
  camera/visibility ordering above was found; if a bug only appears shuffled, a
  constraint is missing. `NODE_EMPIRE_AUDIT=1` also turns on ambiguity
  reporting, which names systems that share data without an explicit order.

### Drawing split

| Layer | Who draws it |
|-------|--------------|
| Terrain | `Mesh2d` quad + the WESL terrain material |
| Node fills, deposit specks, road ribbons, caravan dots | `Mesh2d` + `ColorMaterial` |
| Node outlines, owner rings, selection, spy threads, road-build preview, build ghost | `Gizmos` (vector strokes) |
| Every glyph: node names, population, tooltips, cost labels, toasts, all panels and menus | egui, painted over the top in screen space |

World text is egui because shapes cannot carry text without assets; both layers
share one camera (`camera.rs`), which is what keeps them aligned.

Because 2D gizmos always draw in front of meshes, roads are ribbon meshes
rather than lines, and a road's caravan count is shown by one animated dot per
caravan instead of line width.

### Coordinates

The game layer works in tile units with Y pointing down (the classic screen
convention from the original egui engine); Bevy is Y-up. `render.rs::wpos`
mirrors Y at the boundary, and `camera.rs` converts tile space to viewport
pixels. Terrain is baked one texel per tile into a nearest-sampled image, so
tile edges stay crisp at every zoom.

### The terrain shader (`terrain.rs`, `assets/shaders/terrain.wesl`)

`ColorMaterial` stays the base material, so Bevy keeps owning the mesh2d vertex
pipeline, the bind group layout, tonemapping and alpha handling. The extension
replaces only the fragment stage: it re-samples the base material's texture and
multiplies in a slow two-wave "weather" drift over the map. Its single uniform
is a `vec4` (time, map size, strength) so the Rust and WGSL layouts cannot
drift apart. The clock is `Time<Virtual>`, so the shimmer freezes with the pause
menu and speeds up with fast-forward. If the shader ever fights us, reverting
`sync_world` to a plain `ColorMaterial` for the terrain quad is a two-line
change.

### Error handling (`errors.rs`)

Systems return `Result` and Bevy hands unhandled errors to
`FallbackErrorHandler`. The default panics; ours logs and pushes the message
onto a bounded queue that the HUD drains into the toast line. So a failed save,
a corrupt file or a lookup error is visible on screen instead of killing the
process. `errors::engine_error` tags game-layer messages as `Severity::Error`
so they never escalate to a panic.

### HUD (`ui.rs`, `theme.rs`)

- Panels: top bar (identity, treasury, stock chips, age, clock/speed, help) with
  a slim victory strip (Wealth / Web / Domination meters); left tools + build
  palette; right panel as three tabs (Realm / Node / Agents); slim chronicle
  along the bottom; world text layer.
- Panels draw from an immutable `Hud` snapshot and return `Vec<Action>`; the app
  applies them afterwards, so nothing mutably borrows the world while drawing.
- Selecting a node on the map switches the right panel to its Node tab.
- `theme.rs` holds the palette and the generic widgets (card, section, rule,
  meter row, segmented control, chip, swatch, micro bar); anything specific to a
  game concept stays in `ui.rs`.

## Simulation tick (sim.rs)

One tick = 0.2 s of game time (`TICK_DT`). Order of operations:

1. **Production** at nodes their faction's network reaches (neutral producers
   also work when roaded to a village or linked by any faction)
2. **Caravans** ship bulk stock along Resource/Trade roads; a farm feeds its
   hungriest roaded settlement first, so a passing empire cannot starve a
   village by out-pulling its farm
3. **Supply** — settlements push 1 pop into roaded forts every 6 ticks: a
   garrison settles at a fifth of the settlement feeding it (ceiling 8/12/16/20
   by tier), excess marches home when the village shrinks, and a garrison whose
   fort loses its Supply road drifts home
4. **Governance** — food, happiness, public order, population growth (see below)
5. **Combat** — fort battles, sieges, bandit camps
6. **Espionage** — spy travel, embedded operations (Infiltrate, Siphon,
   Sabotage, Incite), counter-espionage
7. **Influence** — trade routes to rivals grant influence; petitions
8. **Bandits** — camps spawn and raid
9. **Training** — workers/skills progress over ticks
10. **AI** — enemy builds, offense (forts with pop > 6 attack)
11. **Victory check** — Economic (gold), Domination (all settlements),
    Web (Infiltrate spy + 70 influence on every enemy settlement), Defeat

Autosave (slot 0) fires inside `tick()` every 600 ticks; `World::autosave`
turns it off (headless tests set it so they cannot clobber a real save).

## Settlement growth, order and garrisons

The whole economy hangs off one ladder, and it is deliberately short enough to
hold in your head. All of it lives in `game/types.rs` as constants:

```
order target = civic base (32 when fed, 0 when starving)
             + 0.6 x employment %
             + 20 if a Supply road links the settlement to your fort
             - 25 while besieged
order drifts toward that target by 5% a tick, plus +0.1 of civic inertia
happiness follows order exactly (no drift of its own)
growth roll fires while happiness > 55 and order > 40
```

- **Employment** is the share of *workable* posts that are staffed: farms,
  woodcamps, mines, factories, markets, forts and laboratories. Settlements
  cannot host a crew, so their slots are excluded — counting them (6 + 4 per
  tier) would cap a realm below the gate forever, which is exactly what used to
  happen.
- **Where it lands**: an idle realm sits at 34/34 (stable, no growth); five of
  twelve posts staffed reaches 59/59 and grows; a fort alone reaches 54/54 — one
  point short, on purpose, so a garrison is never a substitute for working
  posts; a fort plus one staffed post clears the gate; a besieged realm of the
  same size stalls at 44.
- **Garrisons** are fed from the village behind them: the split settles at 80/20
  between village and fort (`garrison_cap` divides the combined pool by
  `1 + share`), so a village of 30 keeps 25 and its fort holds 5, capped by
  tier at 8/12/16/20. Because the cap follows the *current* village, a raid that
  halves the village calls the surplus home instead of draining survivors.

## New-game spawn guarantees

`place_world` must produce a playable map for any seed, and
`tests/spawn.rs` checks 200 of them:

- Capitals are placed with `Map::nearest_passable`, which never falls back to
  water or an off-map tile (an unreachable capital starved and could never
  recover).
- Every settlement — capitals *and* independent villages — is bootstrapped with
  a farm (force-placed at the nearest buildable tile if the ring fails), a
  woodcamp, a mine, free roads to each, and one novice posted at each. The ring
  belongs to the settlement's owner, so a fresh realm starts with non-zero
  employment and can reach the growth gate.
- Roads fall back to `Map::straight_path` when A\* cannot route, so a starter
  link is never silently lost.
- A settlement at zero population with a working food road lets settlers drift
  in, so an emptied node is recoverable rather than permanently dead.

## Controls

LMB select / place · RMB cancel tool or deselect · drag or MMB pan · wheel zoom
at the cursor · WASD pan · Space pause · 1/2/3 speed 1x/2x/4x · V select tool ·
R road tool · 0 quick-save · H help · Esc back (help → tool → pause menu).
The main menu is also keyboard driven: Enter/Space start a new game, 1..6 pick
a court, H opens help.

## Key invariants

- `World::can_place` is the single source of truth for placement rules; the
  renderer calls it for the ghost and the HUD shows its message.
- UI actions are applied centrally (`apply_action`), which toasts command
  results and failures; panels never touch `World` mutably.
- Save schema stays backward compatible: any new field carries
  `#[serde(default)]` so saves written by an older build keep loading (slot 0 is
  the autosave, 1 the quicksave on key `0`, 2-3 manual).
- Nothing in `engine/` may change `game/` semantics; all tuning lives in
  `game/types.rs`.
