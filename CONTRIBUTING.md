# Contributing to Node Empire 👑

Thank you for your interest in contributing to **Node Empire**! We are building a deep, zero-asset, procedural RTS game inspired by *Seven Kingdoms*, combining the frictionless satisfaction of an idle graph economy with the high-stakes personal drama of military commanders, species tensions, and deep espionage.

Whether you are an experienced Rust veteran, a game designer, a sound composer, or someone looking to make their first open-source pull request, **you are warmly welcome here.**

---

## 🌟 Where Can You Help?

You don't need to be a graphics guru or a Bevy wizard to contribute. The project is split into distinct layers:

| Focus Area | Typical Tasks | Required Skills |
| :--- | :--- | :--- |
| 🧠 **Core Simulation** (`src/game/`) | Economy formulas, AI logic, pathfinding, general mutiny triggers, spy covert ops. | Pure Rust, data structures, algorithms. **(No Bevy knowledge needed!)** |
| 🖥️ **Presentation & UI** (`src/engine/ui.rs`) | HUD panels, general cards, demographic info sheets, hotkey ergonomics. | Rust, `egui` |
| 🎨 **Visuals & Shaders** (`src/engine/render.rs`, `assets/`) | WESL terrain shaders, vector gizmo effects, dynamic road animations. | Bevy 2D, WGSL/WESL, math. |
| 🔊 **Audio & Sound** | Ambient procedural audio, placement sound effects, battle horns. | Audio engineering, `bevy_audio`/`rodio`/`kira`. |
| ⚖️ **Balance & Testing** (`tests/`) | Scenario stress tests, edge-case invariants, seed viability, economy calibration. | Rust, testing, strategy game theory. |
| 📖 **Docs & Lore** (`docs/`) | Court histories, gameplay tutorials, architectural documentation. | Technical writing, Markdown, worldbuilding. |

---

## 🚀 Quickstart for Developers

### 1. Prerequisites
- **Rust 1.85+** (Stable). Install via [rustup](https://rustup.rs/).  
  👉 **Step-by-step OS guide**: See [`docs/technical/SETUP_AND_PLAYTESTING.md`](docs/technical/SETUP_AND_PLAYTESTING.md) for detailed Windows, Linux, and macOS instructions.
- An OS with Vulkan/DirectX 12/Metal support (Windows, Linux, macOS).

### 2. Clone & Run Headless Tests First
Because the game engine is headless-first, you can verify your environment in 2 seconds without launching a window:
```bash
git clone https://github.com/Alcartez/Node-Empire.git
cd Node-Empire
cargo test
```
*If all 30+ tests pass, your environment is 100% verified!*

### 3. Launch the Game
```bash
cargo run --release
```
*(Tip: Always run with `--release` or check `[profile.dev]` — Bevy debug builds without optimization can run slowly.)*

---

## 🏛️ Architectural Invariants (Must-Read)

Before making PRs, keep these core principles in mind:

1. **Simulation Purity**:
   - Code inside `src/game/` must **never import Bevy**. It must remain 100% pure, deterministic, and serializable (`serde_json`).
   - If a feature can be tested headlessly, write a test in `tests/` for it!
2. **Unidirectional UI Data Flow**:
   - Panels in `src/engine/ui.rs` receive an immutable `Hud` snapshot and emit `Action` enums.
   - Panels never mutate `World` directly; actions are resolved centrally in `engine::app::apply_action`.
3. **Save Compatibility**:
   - If you add fields to serializable structs in `game/types.rs`, always add `#[serde(default)]` so existing player saves keep loading.
4. **Zero Art Assets**:
   - Do not commit PNGs, JPEGs, or 3D models. All visual elements are procedural meshes, gizmos, or code-embedded WESL shaders.

---

## 🛠️ Contribution Workflow

1. **Find or Open an Issue**:
   - Check the [Issues](https://github.com/Alcartez/Node-Empire/issues) tab. Look for tags like `good first issue` or `help wanted`.
   - Discuss major gameplay or architecture changes in an issue before writing large PRs.
2. **Fork and Branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Format & Lint**:
   Ensure your code matches project standards:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
4. **Submit a Pull Request**:
   - Reference the issue number (`Closes #123`).
   - Describe what changed and include a screenshot/GIF if UI changes were made.
5. **Code Review**:
   - A maintainer will review your code promptly. We focus on constructive, friendly, and learning-oriented reviews!

---

## 🎖️ Contributor Recognition & Licensing

- Every contributor is permanently credited in [`CONTRIBUTORS.md`](CONTRIBUTORS.md) and inside the in-game Credits screen.
- All code contributed to Node Empire is licensed under the **GNU General Public License v3.0 (or later)** (`GPL-3.0-or-later`).
- See [`CONTRIBUTORS.md`](CONTRIBUTORS.md) for the contributor terms and project leadership details.

---

## 💬 Community & Questions

- **Discord**: Join our community and dev team at [discord.gg/TquJzY24Ea](https://discord.gg/TquJzY24Ea).
- **GitHub Discussions**: For questions, game balance debates, and feature brainstorming.
- **Found a bug?** Open an issue using our [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.md).
- **Need help?** Ping `@Alcartez` or fellow maintainers in the PR/issue thread. We're happy to guide you!
