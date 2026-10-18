# Node Empire: Documentation Library

Welcome to the *Node Empire* documentation directory. The documentation is organized into three specialized categories:

* **[Technical](#1-technical-documentation)**: Architecture, engine pipeline, testing guide, and implementation log.
* **[Planning](#2-planning--roadmap)**: Seven Kingdoms feature overhaul roadmap, MVP scope, and design questions.
* **[Design](#3-game-design--worldbuilding)**: Economic philosophy, mechanics calibration, court lore, and species traits.

---

## 1. Technical Documentation (`docs/technical/`)

Specifications and guides for engine architecture, headless simulation invariants, and testing suites:

| Document | Purpose & Contents |
| :--- | :--- |
| **[`technical/ARCHITECTURE.md`](technical/ARCHITECTURE.md)** | Engine architecture, Bevy presentation vs headless simulation layer, tick pipeline order of operations, and mathematical growth/order formulas. |
| **[`technical/TESTING.md`](technical/TESTING.md)** | Headless test suite guide, `ScenarioHarness` fixture builder, invariant validation rules, and testing commands. |
| **[`technical/IMPLEMENTATION.md`](technical/IMPLEMENTATION.md)** | Historical engineering implementation milestones and commit logs. |

---

## 2. Planning & Roadmap (`docs/planning/`)

Strategic roadmaps, milestone criteria, and architectural questions:

| Document | Purpose & Contents |
| :--- | :--- |
| **[`planning/changes.md`](planning/changes.md)** | *Seven Kingdoms* & Idle Strategy overhaul roadmap: starter forts, named generals, mutiny mechanics, racial demographics, and equipment tiers. |
| **[`planning/MVP.md`](planning/MVP.md)** | Minimum viable product scope, initial release deliverables, and milestone checklists. |
| **[`planning/questions.md`](planning/questions.md)** | Open design questions, mechanics calibration inquiries, and long-term roadmap discussions. |

---

## 3. Game Design & Worldbuilding (`docs/design/`)

Game mechanics, macro demographic design, economic calibrations, and faction lore:

| Document | Purpose & Contents |
| :--- | :--- |
| **[`design/design_notes.md`](design/design_notes.md)** | Core gameplay design philosophies, automated road logistics, macro demographic management, and economic calibrations. |
| **[`design/factions.md`](design/factions.md)** | Deep lore, species traits, demographic alignments, and visual color palettes for the six ruling courts. |

---

## Contributing to Documentation

When contributing new mechanics, systems, or lore:
* **Engine/Testing changes**: Document in [`technical/ARCHITECTURE.md`](technical/ARCHITECTURE.md) and [`technical/TESTING.md`](technical/TESTING.md).
* **Roadmap/Feature progress**: Update [`planning/changes.md`](planning/changes.md).
* **Factions/Species lore**: Document in [`design/factions.md`](design/factions.md).
