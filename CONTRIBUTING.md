# Contributing to Node Empire

Node Empire is a Rust + Bevy realtime strategy game about road networks,
villages, garrisons, and espionage. Contributions of code, tests, balance, docs,
and playtesting are welcome.

Read [`ROADMAP.md`](ROADMAP.md) for what is planned, and
[`docs/MECHANICS.md`](docs/MECHANICS.md) for what exists. A change that adds a
feature should point at a milestone; a change that fixes a lie in the docs is
always welcome.

## Where to help

| Area | Path | Skills |
|---|---|---|
| Simulation, economy, AI, espionage | `src/game/` | Rust, algorithms; no Bevy needed |
| HUD and panels | `src/engine/ui.rs`, `theme.rs` | Rust, egui |
| Rendering, shaders | `src/engine/render.rs`, `assets/` | Bevy 2D, WGSL/WESL |
| Balance and tests | `tests/` | Rust, testing, strategy game theory |
| Docs and lore | `docs/` | Technical writing, Markdown |
| Audio | not started | Bevy audio or a compatible crate |

## Setup

Install Rust 1.98 via [rustup](https://rustup.rs/). Windows needs the MSVC C++
build tools. Full per-OS instructions are in [`docs/SETUP.md`](docs/SETUP.md).

```bash
git clone https://github.com/Alcartez/Node-Empire.git
cd Node-Empire
cargo test
cargo run --release
```

If `cargo test` compiles and runs, your environment is ready. Do not treat a
test count in an old document as a target; run the command and read the summary.

## Architectural invariants

1. **The simulation is pure.**
   `src/game/` must never import Bevy, and every state type must be
   serializable. If a feature can be tested headlessly, add a test in `tests/`.
2. **UI data flows one way.**
   Panels receive an immutable `Hud` snapshot and emit `Action` values. They
   never mutate `World`; `engine::app::apply_action` does that centrally.
3. **Saves stay compatible.**
   New fields on serializable structs need `#[serde(default)]`, and the change
   needs a load test.
4. **No runtime art assets.**
   Visuals are meshes, gizmos, and code-embedded shaders. Screenshots in
   `docs/img/` are documentation, not runtime assets.
5. **No copied Seven Kingdoms code or assets.**
   `scratch/` holds GPL-2.0 reference clones for reading only. They are not part
   of the build. See the licence section of the README.
6. **Tuning lives in `src/game/types.rs`.**
   Do not scatter balance constants through the engine.

## Workflow

1. Open or find an issue. For anything larger than a bug fix, describe the
   mechanic and the test that will prove it before writing code.
2. Branch:
   ```bash
   git checkout -b feature/your-feature
   ```
3. Make the change, with a test if it touches `src/game/`.
4. Run the full gate:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
5. Open a pull request. Reference the issue (`Closes #123`), describe the
   behaviour change, and note any manual playtest step.

## What a good PR contains

- A focused change, not a mixed refactor.
- A test named as a sentence describing the rule
  (`a_fort_alone_is_one_point_short_of_growth`).
- An update to `docs/MECHANICS.md` if a rule or constant changed.
- An update to `docs/PLAYTESTING.md` if the change can only be judged by hand.
- A note in `docs/ARCHITECTURE.md` if the tick pipeline or layers changed.

## Licensing and credit

Contributions are accepted under **GPL-3.0-or-later**, the same licence as the
project (inbound = outbound). There is no separate contributor licence
agreement. Contributors are credited in [`CONTRIBUTORS.md`](CONTRIBUTORS.md)
and in the in-game credits where applicable.

## Questions

Use GitHub Issues for bugs (the bug template asks for OS, commit, and
reproduction steps) and GitHub Discussions for design questions. For a bug,
include the chronicle panel text and whether `cargo test` still passes.
