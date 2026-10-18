# Node Empire

A realtime strategy game of roads, villages, garrisons and spies — a graph
instead of a grid of tanks. You start with one village and a few fields, and you
win by weaving the map together: claim nodes, staff them, keep the peace, and
take the rest by influence or by force.

Built on [Bevy](https://bevyengine.org) 0.20 with **zero art assets**. Every
visual on screen is a shape drawn in code or a procedural texture generated at
startup; the only file in `assets/` is a WESL shader that is compiled into the
binary with `include_str!`. All text is drawn by egui.

```
cargo run --release     # play
cargo test              # the whole simulation, headless, no window
```

Rust 1.85+ (developed on 1.98). Saves live in `saves/` next to the executable's
working directory.

## The game in one screen

- **Nodes and roads.** The map is a graph: settlements, farms, woodcamps, mines,
  factories, markets, laboratories and forts, joined by roads that caravans walk.
  A producer only works if a road reaches it, which is the whole claim mechanic.
- **Workers and jobs.** Post workers at a building to boost its output; their
  skill grows on the job. Workers are drawn from settlement population.
- **Public order, growth and garrisons.** Order comes from *employment* (how many
  of your workable posts are actually staffed) and from a Supply road to one of
  your forts. A settlement grows while happiness is above 55 and order above 40.
  An idle realm sits at 34/34 and stagnates; a fort alone reaches 54 — one point
  short, on purpose. Garrisons take at most a fifth of the village feeding them
  (8/12/16/20 by tier) and march home when the village shrinks.
- **Influence and petitions.** Trade roads and a healthy treasury win hearts; a
  well-regarded court can absorb independent villages without a fight.
- **Espionage** is the signature system: train agents, then infiltrate, siphon,
  sabotage or incite. Victory condition *The Silent Web* needs an agent embedded
  in every rival settlement plus deep influence there.
- **Six courts** — Silk Spinners, Breplid Clans and four more — trade espionage,
  combat, production, defense, trade and growth bonuses.
- **Victory:** Wealth (gold), Domination (every enemy settlement), The Silent Web
  (espionage), or lose when your last settlement falls.

The in-game **H** help window covers the same rules, and
[`docs/technical/ARCHITECTURE.md`](docs/technical/ARCHITECTURE.md) has the engine tour, the exact order/growth
formulas and the tick pipeline.

## Documentation

Full documentation is organized into subdirectories in [`docs/`](docs/):

### Technical ([`docs/technical/`](docs/technical/))
* [`docs/technical/ARCHITECTURE.md`](docs/technical/ARCHITECTURE.md) — Engine layers, tick pipeline, economy formulas, and headless invariants.
* [`docs/technical/TESTING.md`](docs/technical/TESTING.md) — Developer testing guide, `ScenarioHarness` fixture builder, and test suite commands.
* [`docs/technical/IMPLEMENTATION.md`](docs/technical/IMPLEMENTATION.md) — Historical engineering implementation log.

### Planning ([`docs/planning/`](docs/planning/))
* [`docs/planning/changes.md`](docs/planning/changes.md) — *Seven Kingdoms* & Idle Strategy overhaul roadmap and feature status.
* [`docs/planning/MVP.md`](docs/planning/MVP.md) — Minimum viable product scope and milestone criteria.
* [`docs/planning/questions.md`](docs/planning/questions.md) — Open design decisions and future considerations.

### Game Design & Lore ([`docs/design/`](docs/design/))
* [`docs/design/design_notes.md`](docs/design/design_notes.md) — Core mechanics design, automated road logistics, and economic calibrations.
* [`docs/design/factions.md`](docs/design/factions.md) — Lore, species traits, and stat modifiers for the six courts.
* [`backup/README.md`](backup/README.md) — The two retired engines kept for reference.

## Roadmap

1. **Economy and spawns** — order/employment/growth/garrison rules and
   guaranteed, playable starts. *(done)*
2. **Scale** — selectable map sizes (Small 72x56, Standard 112x84, Large
   160x120) with content that scales by area.
3. **Travel** — workers and peasants visibly walk the roads; producers can be
   claimed by walking in; soldiers march to the front.
4. **Calibration** — firms of many workers, taxes that trade loyalty for gold,
   salaries and rebellious loyalty floors, at this game's own (larger) scale
   than its ancestors.

## License

Copyright (C) 2026 the Node Empire authors.

Node Empire is free and open-source software licensed under the terms of the
**GNU General Public License, version 3** (or any later version) as published by
the Free Software Foundation. See [`LICENSE`](LICENSE) for the full text.

All author credits, contributions, and licensing agreements are documented in
[`CONTRIBUTORS.md`](CONTRIBUTORS.md). Under GPL-3.0 Section 5(d), attribution and
interactive credits must be preserved in all derivative distributions.

### A note on Seven Kingdoms (attribution, not endorsement)

Node Empire is *inspired by* the mechanics of **Seven Kingdoms / Seven Kingdoms
II: The Fryhtan Wars**, the classic 1997 realtime strategy game by Trevor Chan
(Enlight Software, later distributed freely as GPL-2.0 source by the community at
[7kfans.com](https://www.7kfans.com)).

- This is a **clean-room implementation**. No 7KAA source code, no strings, no
  artwork, no data files and no other assets were copied, ported or translated
  into this project. What was taken is *mechanics* — ideas such as firms with a
  handful of workers, villages that need food, garrisons that eat population,
  and influence over independent settlements — and game mechanics are not
  copyrightable.
- Node Empire has its own identity, its own scale and its own balance; it is not
  a clone, not a remake, and it is **not affiliated with, endorsed by, or
  supported by** Trevor Chan, Enlight Software, or the 7kfans project.
- "Seven Kingdoms" and "Seven Kingdoms II" are the trademarks of their owners
  and are used here only to describe where the design inspiration came from.

### Third-party dependencies

Bevy, egui and the other crates in `Cargo.lock` are used under their own
licences (Bevy and egui are dual MIT / Apache-2.0). Both are permissive and
compatible with distributing Node Empire under the GPL-3.0.
