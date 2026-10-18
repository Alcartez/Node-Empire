# Open Questions

These questions need answers before development can proceed on specific features.

---

## MVP Questions

| # | Question | Options |
|---|----------|---------|
| 1 | **Which humanoid faction for MVP?** | Beetle Soldiers (defense), Mantis Hunters (assassins), Silk Spinners (spies), Hive Swarms (swarm) |
| 2 | **MVP AI scope?** | Passive bandits only, or simple AI opponent that can be defeated? |
| 3 | **Unit types in MVP?** | Soldiers only from settlements? Or scouts, workers as separate unit types? |
| 4 | **Equipment tiers (Wood→Steel)** | Per-settlement upgrade (all soldiers get better gear) or per-unit (individual soldier equipment)? |
| 5 | **Research cost structure** | Gold cost? Time (turns/ticks)? Production slots consumed? Combination? |
| 6 | **Gold victory threshold** | Fixed amount (e.g., 10,000 gold)? Configurable before each game? Percentage of all players? |
| 7 | **Starting conditions** | 1 settlement + how much starting gold/food? Multiple settlements? |

---

## UI Questions

| # | Question | Options |
|---|----------|---------|
| 8 | **Main Menu options** | Start Game, Load Game, Options (audio/video), Quit? Anything else? |
| 9 | **Map viewport controls** | WASD/arrow keys to pan? Scroll wheel to zoom? Middle-click to pan? |
| 10 | **Settlement panel style** | Slide-in side panel? Centered popup? Dock at bottom? |
| 11 | **Production queue scope** | One queue per settlement OR global queue affecting all settlements? |
| 12 | **Tech tree display** | Visual tree with branches? Flat list of available upgrades? Tab in settlement panel? |
| 13 | **Context menu items** | Right-click: Build, Produce, Upgrade, View Info, Cancel? What else? |
| 14 | **Minimap** | Include in MVP? Top-right corner? Toggleable? |
| 15 | **Save slots** | How many? 5? 10? Unlimited? |
| 16 | **Settings options** | Audio (volume sliders)? Video (resolution, fullscreen)? Keybindings? |

---

## Gameplay Questions

| # | Question | Options |
|---|----------|---------|
| 17 | **Research points generation** | Per research building? Flat rate? Scales with population? |
| 18 | **Public order formula** | Crime = -15% when no jobs/forts. What are the positive modifiers? Forts add +X%? Happiness adds +Y%? |
| 19 | **Gold from settlements** | Flat generation rate? Based on population? Based on trade edges? |
| 20 | **Soldier recruitment** | Costs gold? Food? Population? All three? |
| 21 | **Soldier maintenance** | Ongoing cost per soldier per turn? Or one-time cost? |
| 22 | **Combat resolution speed** | Real-time (ticks)? Turn-based? Hybrid (commands then resolve)? |
| 23 | **Fort defense bonus** | What percentage? Does it stack with unit defense? |
| 24 | **Siege mechanics** | Wall health? Garrison casualties over time? Surrender threshold? |
| 25 | **Victory condition tracking** | Gold: Real-time counter? Military: Capital markers on map? |

---

## Technical Questions

| # | Question | Options |
|---|----------|---------|
| 26 | **Godot + Bevy integration approach** | Godot loads Bevy as GDExtension? Separate processes with IPC? Other? |
| 27 | **Map seed format** | JSON file? Binary? What data is stored (terrain, nodes, edges, resources)? |
| 28 | **Save file structure** | JSON? Binary? Include full game state or diff-based? |
| 29 | **Network architecture** | Authoritative server? Peer-to-peer? Relay server? |
| 30 | **Target frame rate for game logic** | Fixed tick rate (e.g., 60 TPS)? Variable? |

---

## Visuals Questions

| # | Question | Options |
|---|----------|---------|
| 31 | **Terrain colors** | Fixed palette? Procedurally varied? What colors = what terrain? |
| 32 | **Resource icon style** | Pixel art? Vector icons? Colored shapes? |
| 33 | **Settlement visual scaling** | How much larger is City vs Village? 2x? 3x? |
| 34 | **Edge visual style** | Solid lines? Dotted? Animated flow? Color-coded by resource type? |
| 35 | **Impassable terrain** | Dark grey as specified. Any variation (mountains vs obstacles)? |

---

## Priority

Mark these as **P0** (must answer before starting), **P1** (before that phase), or **P2** (can decide later):

| # | Priority | Question |
|---|----------|----------|
| 1 | ? | Which humanoid faction for MVP? |
| 2 | ? | MVP AI scope? |
| 4 | ? | Equipment tiers - per-settlement or per-unit? |
| 6 | ? | Gold victory threshold? |
| 26 | ? | Godot + Bevy integration approach? |
| 8 | ? | Main Menu options? |
| 9 | ? | Map viewport controls? |
| 10 | ? | Settlement panel style? |

---

*Last Updated: 2026-04-06*
