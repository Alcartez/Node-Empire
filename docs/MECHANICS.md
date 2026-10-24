# Node Empire — Implemented Mechanics

This file describes **only what the build currently does**. Every number here
comes from `src/game/` and can be changed in one place. If a rule is not in this
file, it is not implemented.

- Tunables: `src/game/types.rs`
- Tick pipeline and rules: `src/game/sim.rs`
- World state and commands: `src/game/world.rs`
- Tests that pin each system are named in the sections below.

Design intent that is **not** implemented lives in [`VISION.md`](VISION.md) and
[`../ROADMAP.md`](../ROADMAP.md).

---

## 1. Time and determinism

| Fact | Value | Source |
|---|---|---|
| Simulation rate | 5 ticks per second (`TICK_DT = 0.2s`) | `types.rs::TICKS_PER_SEC` |
| Speeds | paused, 1x, 2x, 4x (`Time<Virtual>`) | `engine/app.rs` |
| Randomness | one serializable xorshift64* stream per `World`, seeded from the map seed | `types.rs::Rng` |
| Determinism | same seed + same input sequence = same world; the RNG state is saved | `world.rs`, `save.rs` |
| Headless | `src/game/` has no Bevy imports and runs under `cargo test` | `Cargo.toml`, `tests/` |

---

## 2. Map

- **Size: 120 x 90 tiles** (`World::new` → `Map::generate(seed, 120, 90)`).
- Terrain: `Water`, `Sand`, `Grass`, `Forest`, `Mountain`. Water and mountains
  are impassable and unbuildable; sand, grass and forest are buildable.
- Deposits: `None`, `Copper`, `Iron`, drawn per tile and required by mines.
- Generation is fully procedural and seed-driven; there are no map files.

### Spawn guarantees (`World::place_world`)

- Player capital in the north-west, enemy capital in the south-east.
- **16 neutral settlements** scattered across the middle, at least 12 tiles
  apart.
- Resource nodes: **24 farms, 18 woodcamps, 16 mines**.
- **6 bandit camps** (`Owner::Bandit`), 20–39 population each.
- Every settlement — capitals and neutral villages alike — gets a starter ring:
  one farm, one woodcamp, one mine, free roads to each, one novice posted at
  each. The ring belongs to the settlement's owner.
- Each capital gets a starter fort, a founding general, and a free Supply road.

Pinned by `tests/spawn.rs` (200 seeds) and
`tests/suite_invariants.rs::test_procedural_generation_invariants_across_seeds`.

---

## 3. Nodes and roads

| Building | Cost (gold) | Job | Slots |
|---|---:|---|---:|
| Settlement | 200 | — (employs indirectly) | 0 counted |
| Farm | 50 | Farmer | 4 |
| Mine | 75 | Miner | 4 |
| Woodcamp | 40 | Lumberjack | 4 |
| Factory | 100 | Manufacturer | 6 |
| Market | 100 | Manufacturer | 6 |
| Fort | 125 | Soldier | 8 |
| Laboratory | 150 | Scholar | 4 |
| Bandit Camp | not buildable | — | 4 |

Slots come from `Node::job_slots`; settlement slots are excluded from employment
on purpose (`Node::employable_slots`).

### Road rules

- A road must be **1.5 to 22 tiles** apart in straight-line distance; A* may
  wander up to **2.6x** that distance; terrain must be passable.
- Cost = `5 + number of points in the A* path` gold (`World::road_cost`).
- One road per node pair. Roads are classified automatically:
  - `Resource` — production ↔ consumer links
  - `Supply` — settlement ↔ your fort
  - `Combat` / `Siege` — fort ↔ hostile fort or bandit camp
  - `Trade` — trade routes (count toward influence)
- Each road carries **1–6 caravans**; more caravans shorten the delivery period.

---

## 4. Production

A node produces only when its owner's road network reaches it. Neutral
producers also work when linked by any faction, when part of their own village
network, or when roaded to a village.

| Node | Output per tick | Notes |
|---|---|---|
| Farm | `5 + 1.5 x ready farmers` food | base feeds ~50 pop |
| Woodcamp | `2 + 1.5 x ready lumberjacks` timber | needs forest |
| Mine | `2 + 0.06 x average miner skill` ore | needs a deposit |
| Factory | consumes 1 ore + 1 timber → 2 goods | fractional carry |
| Market | sells goods at 10 gold each, ~2/tick before multipliers | × trade multiplier |
| Laboratory | `1 + 1.2 x scholars + 0.04 x skill` research | |
| Settlement | tax `pop x 0.02 x clamp(happiness/70, 0.4, 1.6)` gold | |
| Bandit Camp | none | |

Age and faction multipliers apply to production (see §13 and §14).

---

## 5. Caravans and transfers

Each Resource/Trade road accumulates progress every tick:

```
period = max(path_length x 2.0 / caravans, 2)   # ticks per delivery
```

On delivery the road moves **one stock type** in the producer→consumer
direction, or reverses if that fails:

| From → To | Resource | Reserve kept |
|---|---|---|
| Farm → Settlement | food | 0 |
| Mine → Factory / Market | ore | 0 |
| Woodcamp → Factory | timber | 0 |
| Factory → Settlement / Market | goods | 0 |
| Settlement → Market | goods | 0 |
| Settlement → Settlement | food | 15 ticks of eating (`pop/10 x 15`) |

A farm always feeds its **hungriest** roaded settlement first, so a passing army
cannot strip a village's food. Ownership gate: you may feed your own or neutral
nodes, never a rival's.

Note: `Edge::length()` is the number of points in the path, not Euclidean
distance. Harness-built two-point roads therefore deliver every 4 ticks; real
A* roads are longer and slower.

---

## 6. Fealty and peaceful integration

Neutral settlements track fealty per faction (`Node::fealty`). A delivery that
arrives at a neutral settlement awards fealty to the delivering faction:

| Delivery | Fealty |
|---|---:|
| Food / raw stock | **+1.2** |
| Manufactured goods from a factory | **+2.5** |

The rival faction's fealty on that settlement is eroded by **20% of the gain**.
Fealty is capped at 100.

### Tiers (`AllegianceTier`)

| Fealty | Tier | Effect |
|---:|---|---|
| 0–24 | Wary Neutral | normal trade |
| 25–49 | Trading Partner | **+3 gold** per qualifying delivery |
| 50–84 | Allied Protector | event only (no levy is spawned yet) |
| 85–100 | Integrated Domain | village joins the delivering faction; its connected neutral production ring flips too; +50 gold |

Integration happens immediately on the delivery that crosses 85. Only
Player/Enemy deliveries award fealty; a neutral village's own internal roads
award nothing.

Pinned by `tests/suite_fealty.rs` (4 tests).

**Known gap:** the +3 dividend has no decay, cooldown, or cost, so a mature
trade road is a slow money printer. Tracked in `ROADMAP.md` (M1).

---

## 7. Governance: order, happiness, growth, rebellion

Computed per settlement every tick (`Sim::governance`):

```
civic          = 32 if fed, else 0
target_order   = civic
               + 0.6 x employment %
               + fort_order_bonus (20, or general-scaled)
               - 25 if besieged
order         += (target_order - order) x 0.05 + 0.1
target_happy   = order x 0.6 + happiness x 0.4 + faction happiness
```

- **Employment** = filled posts / workable posts across everything the owner
  holds. Settlements do not contribute slots.
- A settlement **grows** only while `happiness > 55` **and** `order > 40`:
  `chance = 0.02 x growth multiplier x (happiness/100 + 0.3)` per tick.
- Feeding: each settlement eats `max(1, round(pop/10 x race food multiplier))`
  food per tick. Starving settlements lose 1 pop and 4 happiness per tick and
  set civic order to zero.
- Empty settlements with food or a food road can regain settlers (5% chance).
- **Rebellion:** if `loyalty < 35` and `happiness < 30`, a non-neutral
  settlement has a 2% chance per tick to go neutral.
- **Loyalty aura:** every 100 ticks a general posted to the settlement's Supply
  fort restores `max(1, leadership / 25)` loyalty (up to 100).
- Tiers: village < 100 pop, town 100–499, city 500–1999, metropolis 2000+.

Reference points from `tests/economy.rs`:

- idle realm: **34 order / 34 happiness** — stable, no growth
- fort alone: **54 / 54** — one point short, on purpose
- fort + one staffed post: clears both gates
- besieged equivalent: stalls around 44

---

## 8. Garrisons

- A Supply road between a settlement and a same-owner fort pushes **1 population
  into the fort every 6 ticks**.
- Garrison cap: `pool x 0.20 / 1.20`, clamped to `1..` a tier ceiling of
  **8 / 12 / 16 / 20** for village → metropolis (`types.rs::garrison_cap`).
- The settlement keeps at least 80% of the combined pool. When the village
  shrinks, loses the road, or is captured, surplus soldiers march home.

Pinned by `tests/economy.rs` (garrison tests) and
`tests/sim_stress.rs::supply_feeding_moves_pop_into_fort`.

---

## 9. Workers, training, auto-employment

### Manual training

| Tier | Cost | Starting skill |
|---|---:|---:|
| Basic | 10 | 10 |
| Specialist | 40 | 40 |
| Expert | 90 | 80 |

Training pulls one population from one of the payer's settlements. Workers gain
**+1 skill every 30 ticks** on the job (cap 100). Each ready worker adds to
production and combat. Ownership, building type, and free slots are checked
*before* gold is spent, so a refused order never costs the player. Pinned by
`tests/suite_training.rs` (4 tests).

### Auto-employment

Every **20 ticks** (`AUTO_EMPLOY_INTERVAL`), a connected production building
with a free slot and a same-owner road to a player/enemy settlement hires
**one novice (skill 5)** — no gold cost — as long as the settlement keeps
`MIN_SETTLEMENT_POP_RESERVE = 5` people. Foreign buildings are never staffed.

Pinned by `tests/suite_auto_employ.rs` (4 tests).

---

## 10. Generals

| Stat | Effect |
|---|---|
| `leadership` | fort order bonus: `20 x (0.8 + 0.4 x leadership/100)`, then x1.15 if the general shares the settlement's dominant race, x0.90 otherwise; also adds `leadership/100 x 0.20` to fort defense |
| `combat` | battle multiplier: `1 + combat/100 x 0.5` |
| `loyalty` | 0–100; adds `loyalty/100 x 0.10` to fort defense; warned below 20, mutiny below 10 |
| `salary` | **flat 15 gold**, never varies |

A general assigned to a fort (`fort.general_id`) therefore does three things:

- **Defense:** multiplies the fort's defense by
  `1 + 0.20 x leadership/100 + 0.10 x loyalty/100` (max **+30%**). Defense
  divides incoming losses, so the commander's troops last longer.
- **Order:** scales the fort's order bonus to its supplied settlement.
- **Loyalty aura:** every 100 ticks, adds `max(1, leadership / 25)` loyalty to
  the settlement that fort supplies.

Upkeep runs every **100 ticks**:

- Paid: salary deducted; loyalty below 80 rises by 1.
- Unpaid: loyalty **drops by 6** and the player gets a warning.
- `loyalty < 10`: the general mutinies, takes their fort, and both become
  `Owner::Bandit`.

Pinned by `tests/suite_generals.rs` (7 tests) and
`tests/seven_kingdoms_features.rs`.

---

## 11. Espionage

- Train an agent at a settlement with 4+ population for **75 gold**; it consumes
  1 population.
- Travel time is `distance x 4` ticks per tile. Recalling sends the agent home
  and removes it from the roster on arrival.
- Embedded agents run one operation per tick against a non-owned target.

| Operation | Effect per tick | Base detection chance/tick |
|---|---|---:|
| Infiltrate | −0.15 target order | 0.8% |
| Siphon | steal `1 + skill/30 + embed/100` gold (or food) | 1.8% |
| Sabotage | sets `sabotage = 40`, −0.5 happiness | 2.5% |
| Incite | −(1 + skill/40) loyalty, −0.7 happiness, −0.9 order | 2.2% |

Detection risk is scaled by the target faction's spy multiplier and the agent's
skill; a caught agent is removed. At zero loyalty, Incite has a 35% chance to
flip the settlement to the spy's owner (population halved).

There is currently **no test suite for espionage**; it is exercised only by the
long stress run. Listed as thin in `README.md` and `ROADMAP.md`.

---

## 12. Influence, petitions, foment, purchase

Influence is 0–100 (`World::influence`):

```
influence = 22
          + (gold rising ? +8 : gold falling ? -8 : 0)   # King Rating
          + total food x 0.06
          + (average happiness - 50) x 0.35
          + 8 per Trade road from the viewer to the target
          + 25 if the viewer has an embedded spy there
          - 0.9 x distance to the viewer's nearest settlement
```

Clamped to 0–100. It applies only to settlements the viewer does not own.

| Action | Requirement | Cost | Result |
|---|---|---:|---|
| Petition (automatic) | influence ≥ **41** | — | 1% chance per tick to join peacefully |
| Foment | influence ≥ **61** | 100 gold | flips owner, loyalty 45, −15 happiness |
| Buy settlement | influence ≥ **81** | 200 gold | flips owner outright |

No dedicated tests; covered indirectly by the stress run. Listed as thin.

---

## 13. Combat, sieges, equipment

War is fought entirely over nodes: there are no marching columns and no direct
unit control. Combat happens only along `Combat` and `Siege` roads between
hostile owners, resolved once per tick.

### Fort battles (`Combat` road)

Each side computes an **attack** and a **defense**:

```
attack = (1 + 0.08 x garrison + 0.02 x soldier skill)
         x (0.5 + loyalty/200)            # morale
         x (0.75 if the fort is unsupplied)
         x faction combat
         x race combat multiplier
         x equipment combat multiplier
         x general combat multiplier

defense = 1.15
          x race defense multiplier
          x equipment defense multiplier
          x general fort defense multiplier   # 1 + 0.20 x Ldr/100 + 0.10 x Loy/100
```

Losses on each side are `1.1 x attack_other / defense_self`, symmetric, so
neither end of the road gets a free bonus. Because defense divides incoming
losses, a commander protects their own garrison instead of adding offense. A
fort at 0 population is captured; bandit camps become neutral, and bandit camps
fight forts like any other hostile node.

### Sieges (`Siege` road, fort to hostile settlement)

- The settlement is marked `besieged`, taking the -25 order penalty and
  stalling growth.
- **Siege investment** (`Node.siege_progress`, 0–100) grows each tick by
  `max(1, siege power / town defense)`:

```
siege power = garrison x 0.10
              x (1 + 0.02 x soldier skill)
              x equipment combat
              x general combat
              x (1 + 0.25 x leadership/100)
              x (1.00 supplied, 0.70 unsupplied)

town defense = (1 + 0.01 x loyalty + tier bonus)
               x faction defense
               x race defense multiplier
```

- Casualties are **pulsed every 5 ticks** (`damage` population, -1 happiness,
  -1 loyalty), so a siege captures a town rather than erasing it. The
  besieging fort loses `max(1, town pop / 150)` every 3 ticks to urban
  resistance.
- Capture occurs at **100 investment** or when population hits zero. The town
  is preserved: at least 10 townsfolk remain, the occupying garrison (up to 20)
  moves in, loyalty resets to 40, and investment clears.
- A broken siege (road removed or no longer hostile) loses 1 investment per
  tick until it reaches zero.

### Supply

A fort is **supplied** when it has a `Supply` road to any friendly node. An
unsupplied fort fights at 75% attack and sieges at 70% power, so cutting the
road is a real war aim.

### Equipment tiers (per fort)

| Tier | Combat | Defense | Upgrade cost |
|---|---:|---:|---|
| Militia | 1.00 | 1.00 | 50 gold + 15 goods |
| Bronze | 1.25 | 1.15 | 125 gold + 40 goods |
| Iron-Clad | 1.60 | 1.35 | 250 gold + 80 goods |
| Steel & Siege | 2.00 | 1.60 | - |

Upgrades require **both** gold and goods. The goods are drawn from the
friendly settlement feeding the fort by Supply road (the best-stocked one when
several are linked); with no Supply road, or too few goods, the upgrade is
refused and nothing is charged. Only the fort's owner may upgrade it.

Pinned by `tests/suite_combat.rs` (3 tests) and `tests/suite_equipment.rs`
(5 tests).

---

## 14. Ages and research

| Age | Tech to advance | Production | Combat |
|---|---:|---:|---:|
| Wood | 120 | +0% | +0% |
| Copper | 300 | +10% | +15% |
| Iron | 650 | +20% | +30% |
| Steel | — | +30% | +45% |

Research points come from Laboratories. Advancing is a ledger operation; the
points do not yet unlock buildings or doctrines.

---

## 15. Races

Races are **passive multipliers only** — there are no unique units or buildings.

| Race | Growth | Food use | Combat | Defense |
|---|---:|---:|---:|---:|
| Human | 1.00 | 1.00 | 1.00 | 1.00 |
| Beetlefolk | 0.85 | 1.00 | 1.20 | 1.35 |
| Hive Swarm | 1.45 | 1.30 | 0.85 | 0.85 |
| Silk Spinner | 1.05 | 0.95 | 0.90 | 1.00 |
| Reptilian | 0.90 | 0.75 | 1.15 | 1.15 |

Pinned by `tests/suite_demographics.rs` (2 tests).

---

## 16. Victory and defeat

Checked every tick (`Sim::check_victory`):

| Condition | Trigger |
|---|---|
| **Wealth** | player gold ≥ `win_gold` (default **5000**) |
| **Domination** | enemy holds zero settlements and player holds at least one |
| **The Silent Web** | 100% of enemy settlements are "bound": embedded spy **and** influence ≥ 70 |
| **Defeat** | player holds zero settlements, or enemy gold reaches the threshold |

The AI also wins if its treasury reaches the threshold.

There is currently **no dedicated victory test**; the conditions are simple
predicates over state that other suites exercise.

---

## 17. Difficulty

| Difficulty | AI decision interval | Free AI gold per interval | AI aggression |
|---|---:|---:|---:|
| Easy | 60 ticks | 0 | 0.6 |
| Medium | 35 ticks | 15 | 1.0 |
| Hard | 18 ticks | 40 | higher |

Difficulty changes the AI only; player rules are identical.

---

## 18. Saves

- JSON (`serde`), schema version **2** (`save.rs::SAVE_VERSION`).
- Slots: `0` autosave (every 600 ticks), `1` quicksave (`0` key), `2–3` manual.
- Saved fields include the full `World`: map, nodes, edges, spies, generals,
  ledgers, RNG state, tick, victory, stats.
- Older saves load because new fields use `#[serde(default)]`.
- Newer-version saves are refused.
- `saved_at` is an **HH:MM:SS UTC** string. The UI does not yet convert it to
  local time.

Pinned by `tests/economy.rs` (two save tests) and
`tests/suite_invariants.rs::test_save_load_roundtrip_with_new_features`.

---

## 19. What is *not* implemented

For honesty, these are frequently assumed to exist and do not:

- Asymmetric factions (unique units, buildings, or abilities). The six courts
  are multiplier profiles only.
- More than two real powers. `Owner` is a four-variant enum
  (`Player`, `Enemy`, `Neutral`, `Bandit`).
- Diplomacy, treaties, alliances, reputation, or a tax slider.
- Population migration between settlements, inns, or recruitable generals
  beyond the starting one per capital.
- Spy cover identities, bribing generals, coups, or counter-intelligence
  networks.
- Audio of any kind.
- Fog of war, scouts, or map monoliths.
- Multiplayer.

These are sequenced in [`../ROADMAP.md`](../ROADMAP.md).
