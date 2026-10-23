# Node Empire — Roadmap

One roadmap. No dates, no estimates, no second phase list. If a system is not
in this file, it is not planned here.

**Thesis:** a realtime strategy game about *networks*, not tank spam. You build
roads, staff buildings, win villages by trade and influence, and hold them with
garrisons and spies. The current build is a playable single-player alpha with
two real powers.

- What exists **now**: [`README.md`](README.md) and
  [`docs/MECHANICS.md`](docs/MECHANICS.md).
- What is **design intent only**: [`docs/VISION.md`](docs/VISION.md).

---

## Status legend

| Mark | Meaning |
|---|---|
| ✅ **Shipped** | Implemented and covered by a named test |
| 🟡 **Thin** | Implemented, but untested, unbalanced, or missing a key part |
| 📝 **Designed** | Written down, not implemented |
| ⛔ **Cut** | Explicitly removed from scope |

A ✅ without a test is not allowed. If a test is deleted, the row drops to 🟡.

---

## Where the build is (v0.3.0 / M0)

| System | Status | Evidence / gap |
|---|---|---|
| Procedural 120×90 map, 200-seed viability | ✅ | `tests/spawn.rs`, `suite_invariants` |
| Roads, A* pathing, caravans | ✅ | `tests/economy.rs`, `sim_stress` |
| Production chain (farm/mine/camp/factory/market/lab) | ✅ | `tests/economy.rs` |
| Order, happiness, growth gates | ✅ | `tests/economy.rs` (5 tests) |
| Garrisons, supply links, tier caps | ✅ | `tests/economy.rs`, `sim_stress` |
| Manual worker training and skill growth | ✅ | `tests/suite_training.rs` (4 tests) |
| Auto-employment | ✅ | `tests/suite_auto_employ.rs` |
| Generals: salary, loyalty, mutiny | ✅ | `tests/suite_generals.rs` |
| Equipment tiers | ✅ | `tests/suite_equipment.rs` |
| Races as passive multipliers | ✅ | `tests/suite_demographics.rs` |
| Fealty, tiers, peaceful integration | ✅ | `tests/suite_fealty.rs` |
| Save/load and schema back-compat | ✅ | `economy`, `suite_invariants` |
| Influence, petitions, foment, purchase | 🟡 | works, no dedicated suite |
| Espionage (4 ops + detection) | 🟡 | works, no dedicated suite |
| Combat, sieges, bandits | 🟡 | stress-tested only |
| Victory conditions | 🟡 | simple predicates, no direct test |
| Faction profiles | 🟡 | six multiplier sets; no asymmetric content |
| AI opponent | 🟡 | one enemy, personality and difficulty only |
| Audio | ⛔ | not started; deliberately sequenced last |

---

## M0 — v0.3.1 Integrity *(current)*

**Theme:** make the repository tell the truth, and make CI able to prove it.

- [x] `cargo fmt --check` passes
- [x] `cargo clippy --all-targets -- -D warnings` passes
- [x] `Cargo.lock` committed; toolchain pinned in `rust-toolchain.toml`
- [x] One roadmap (this file); six superseded docs deleted
- [x] `docs/MECHANICS.md` written from the code
- [x] Fealty, auto-employment, and training suites (4 tests each)
- [x] In-game help covers auto-employment and fealty
- [x] Save timestamps labelled UTC
- [ ] CI green on Windows **and** Linux for this commit
- [x] Every ✅ row above names a test that a reviewer can run

**Done when:** a fresh clone runs the four verification commands in
`docs/TESTING.md` without a single warning, and no document claims a feature the
build does not have.

---

## M1 — v0.4 Political economy

**Theme:** depth before polish. The criticism of the current build is "broad but
shallow"; this milestone answers it before audio.

- [ ] **Fealty decay and a delivery cap.** Remove the infinite trade dividend
      money printer. Fealty should decay without contact and the dividend should
      scale with tier, not repeat forever.
- [ ] **Reputation 0–100.** A global standing that rises with fair trade and
      falls with razing, betrayal, and broken deals. Gates petitions and
      general recruitment.
- [ ] **Tax slider vs order.** Player chooses a tax rate; high tax raises gold
      and depresses order and happiness.
- [ ] **Unrest consequences.** Low order produces emigration or secession, not
      just stalled growth.
- [ ] **Inns and recruitment.** A buildable node that offers generals and
      specialists over time, so the starter general is not the only one.
- [ ] **Balance bands.** Scripted scenarios assert realistic time-to-grow and
      time-to-integrate ranges instead of one-off numbers.

**Done when:** a scripted 30-minute scenario can be won by trade and influence
without founding a second settlement, and by conquest without going bankrupt.

---

## M2 — v0.5 Command and intrigue

**Theme:** make espionage and command dangerous, which is the game's pitch.

- [ ] **Spy cover roles.** Agents pose as workers, merchants, or soldiers and
      survive casual inspection.
- [ ] **Bribe a general.** Gold flips an enemy commander; loyalty and reputation
      both matter.
- [ ] **Coup d'état.** An embedded agent in a fort can defect it, its garrison,
      and its road nodes.
- [ ] **Counter-intelligence.** Stationed agents detect and remove foreign moles.
- [ ] **Ambition and succession.** Generals develop ambitions; a dead or
      discredited ruler can trigger a succession crisis.

**Done when:** each operation has a suite, and a coup test flips a fort in a
deterministic scenario.

---

## M3 — v0.6 Living world

**Theme:** scale the politics from a duel to a map.

- [ ] Replace `Owner` with a real `FactionId`; migrate saves.
- [ ] 3–7 AI kingdoms with distinct personalities.
- [ ] Diplomacy matrix: war, ceasefire, peace, trade pact, defensive alliance.
- [ ] Attractiveness and migration: prosperous settlements drain struggling
      neighbours along roads; soft annexation is a real path.
- [ ] Fog of war, scouts, and watchtowers.
- [ ] Monoliths / greater powers as late-game objectives.

**Done when:** a deterministic run with at least three AI factions completes and
a save from M2 migrates without data loss.

---

## M4 — v0.7 Presentation

**Theme:** feel. Deliberately after the politics, because the current gap is
depth, not polish.

- [ ] Audio: ambient bed, placement clicks, coin chimes, battle drums, mutiny
      horns.
- [ ] Visible caravans, marching workers, and soldiers on roads.
- [ ] Combat dust, siege arrows, day/night or seasonal tint.
- [ ] UI animation: meter fills, toast transitions, panel easing.

**Done when:** the manual playtest checklist in
[`docs/PLAYTESTING.md`](docs/PLAYTESTING.md) passes with sound enabled.

---

## M5 — v1.0 Release

- [ ] Data-driven factions, buildings, and equipment in TOML with hot reload.
- [ ] Localization and accessibility (scaling, colour-blind palette, rebinding).
- [ ] Scenarios and a map/seed editor.
- [ ] Lockstep multiplayer (co-op first), or an explicit decision to ship
      single-player only.
- [ ] Reproducible release builds and packaging.

**Done when:** a clean machine can install, play, save, reload, and finish a
scenario; release artefacts are reproducible from a tagged commit.

---

## Cut / not planned

- ⛔ **12 asymmetric factions at launch.** Six multiplier profiles exist; unique
  unit and building rosters are a post-1.0 question, not a launch promise.
- ⛔ **Replacing Bevy or rewriting the engine again.** The simulation is already
  engine-agnostic; another port buys nothing.
- ⛔ **Live-service or Steam-first features.** Achievements, cloud saves, and
  store packaging are M5 concerns only.
- ⛔ **Provenance claims beyond "independent implementation".** No code or
  assets are copied; see the licence section in `README.md`.

---

## Open decisions

These are genuinely undecided; do not treat either option as promised.

1. **Victory threshold.** Keep a flat 5000 gold for every difficulty, or scale
   it with map size and AI count?
2. **How many AI kingdoms in M3.** Three is tractable; seven is the fantasy.
   Pick the smallest number that makes diplomacy interesting.
3. **Multiplayer scope.** Deterministic lockstep co-op is feasible; competitive
   netcode is a much larger commitment.
4. **Configuration format.** Move tunables to TOML in M5, or keep them as Rust
   constants for compile-time safety?
5. **Localization priority.** Ship English-only at 1.0, or block on a
   translation pipeline?

---

## How to propose work

Open an issue using the templates in `.github/ISSUE_TEMPLATE/`. A proposal that
adds a milestone item should say which milestone it belongs to and what test
will prove it. Features that cannot be tested headlessly need a playtest step in
`docs/PLAYTESTING.md` instead.
