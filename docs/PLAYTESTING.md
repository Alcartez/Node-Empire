# Node Empire — Playtesting Checklist

Manual QA for things the headless suite cannot see. Run it before a release or
after UI changes, and record the build/commit you tested.

## Controls

| Action | Control |
|---|---|
| Select / place | Left click |
| Cancel tool or deselect | Right click |
| Pan | `WASD`, arrow keys, drag, or middle-mouse drag |
| Zoom | Mouse wheel at the cursor |
| Pause / resume | `Space` |
| Speed 1x / 2x / 4x | `1` / `2` / `3` |
| Select tool / road tool | `V` / `R` |
| Quick-save to slot 1 | `0` |
| Help | `H` |
| Back / pause menu | `Esc` |

Main menu: `Enter` or `Space` starts, `1`–`6` picks a court, `H` opens help.

## 1. Startup and court selection

- [ ] `cargo run --release` opens a window and renders terrain without errors.
- [ ] All six courts appear with their motto and six stat bars.
- [ ] `1`–`6` change the selected court; the blurb and bars update.
- [ ] Easy / Medium / Hard select correctly.
- [ ] **New Game** starts a realm; **Continue** is disabled with no save.

## 2. Start state

- [ ] Capital sits on passable land with ~30 population, ~60 food, 300 gold.
- [ ] A farm, woodcamp, and mine are roaded to the capital; each has one worker.
- [ ] A fort is roaded to the capital with a Supply road.
- [ ] A general is stationed in the fort with leadership/combat/loyalty visible.
- [ ] The Realm tab shows "Order" and the posts-filled count, and the Node tab
      explains growth when it is stalled.

## 3. Roads and caravans

- [ ] `R` shows a route preview that avoids water and mountains.
- [ ] Placing a road deducts gold and draws a ribbon, not a flat line.
- [ ] Caravan dots travel the road and stock moves producer → consumer.
- [ ] Cutting a road stops deliveries; a farm no longer feeds that settlement.
- [ ] A road that is too long or crosses water is refused with a readable toast.

## 4. Growth, employment, and garrisons

- [ ] A brand-new idle realm sits around 34 order / 34 happiness and does not
      grow.
- [ ] Filling workable posts raises order above 40 and growth begins once
      happiness passes 55.
- [ ] A fort alone leaves order just short of the growth gate (about 54).
- [ ] A connected production building hires a novice roughly every second
      (20 ticks) until its posts are full, without spending gold.
- [ ] Population never drops below 5 to staff a building.
- [ ] The fort garrison respects the tier cap (8/12/16/20) and returns surplus
      population when the village shrinks.

## 5. Fealty and peaceful integration

- [ ] A Resource road to a neutral village produces deliveries and the village's
      Fealty readout rises.
- [ ] At 25 fealty the tier changes to Trading Partner and gold ticks up on
      qualifying deliveries.
- [ ] At 50 the tier changes to Allied Protector.
- [ ] At 85 the village joins the realm, its connected farm flips owner, and the
      chronicle records the integration.
- [ ] A rival road to the same neutral erodes your fealty.

## 6. Generals and mutiny

- [ ] The fort's general is paid 15 gold every 100 ticks.
- [ ] Spending the treasury to zero produces an "unpaid" warning and loyalty
      falls by 6 per cycle.
- [ ] Low loyalty produces a critical warning below 20.
- [ ] Below 10 loyalty the general mutinies: the fort becomes a bandit
      stronghold and the chronicle reports it.

## 7. Espionage

- [ ] A spy can be hired for 75 gold from a settlement with 4+ population.
- [ ] The agent travels, then shows as embedded at the target.
- [ ] Switching between Infiltrate / Siphon / Sabotage / Incite changes the
      operation and produces visible effects (gold, order, loyalty, sabotage).
- [ ] Detection removes the agent and logs the capture.
- [ ] Recall sends the agent home and removes it from the roster on arrival.

## 8. War and equipment

- [ ] A fort with a garrison can attack an adjacent hostile fort or bandit camp.
- [ ] Sieges mark the settlement and suppress its growth.
- [ ] Upgrading a fort's equipment deducts gold and goods and changes its tier
      label; an enemy fort cannot be upgraded.
- [ ] Bandit camps raid nearby settlements, stealing food and population.

## 9. Victory and defeat

- [ ] The top strip tracks Wealth, Web, and Domination progress.
- [ ] Reaching 5000 gold triggers the Wealth end screen.
- [ ] Taking every enemy settlement triggers Domination.
- [ ] Losing the last settlement triggers Defeat.

## 10. Save, load, and quit

- [ ] `0` quick-saves; the toast confirms.
- [ ] Change the world, then load slot 1: tick, positions, stocks, generals, and
      spies match.
- [ ] Autosave appears in slot 0 after 600 ticks.
- [ ] Quit and relaunch: **Continue** loads the autosave.
- [ ] A save from an older build still loads.

## Diagnostics

```bash
# Schedule ambiguity / ordering audit (debug builds)
NODE_EMPIRE_AUDIT=1 cargo run

# Long headless stability run with output
cargo test --test sim_stress -- --nocapture
```

Report a playtest failure with: commit hash, OS, what you did, what happened,
what you expected, and the relevant lines from the chronicle panel.
