# Seven Kingdoms — Mechanics Study

A comparison between the mechanics of *Seven Kingdoms* (7K) and Node Empire's
road-graph design. It records what was studied, what was translated, and what is
deliberately different. It is **not** a parity checklist: Node Empire is its own
game, not a remake.

**Method and licensing.** The reference is the freely published GPL-2.0 source
of *Seven Kingdoms* / *Seven Kingdoms II* (7KAA and 7K-Ambition). Those clones
live only in the gitignored `scratch/` directory, are read-only, and are never
compiled or linked. Nothing in this repository is copied, ported, or adapted
from them — no code, strings, data, or assets. The notes below are observations
about game mechanics, written from scratch. Because no code is shared, the
GPL-2.0 reference does not affect Node Empire's GPL-3.0-or-later licence.

## The translation rule

> **In 7K, proximity is physical distance. In Node Empire, proximity is a road
> connection.**

Anything 7K does with a radius, Node Empire should do with an edge or a path.
That single rule explains most of the design differences below.

## Status key

| Mark | Meaning |
|---|---|
| ✅ | Implemented and tested |
| 🟡 | Implemented, shallow or untested |
| 📝 | Designed, not implemented |
| — | Not planned |

## Mechanic-by-mechanic

| 7K mechanic | Node Empire today | Status | Where it goes |
|---|---|---|---|
| **Vast living map** | 120×90 procedural map, 16 neutral villages, 6 bandit camps, player and enemy capitals in opposite corners. Much smaller than 7K's continents, and there is no exploration layer. | 🟡 | M3 (fog, scouts, larger maps if playtests justify them) |
| **Open building ground** | Terrain is generated per seed; buildability is checked by `World::can_place`. Density is seed-dependent and not tuned for wide-open plains. | 🟡 | M3 |
| **Peasants find work automatically** | Connected production buildings hire one novice every 20 ticks at no gold cost, keeping a reserve of 5 population. Manual training remains available for higher skill. | ✅ | Shipped |
| **On-the-job experience** | Workers gain +1 skill every 30 ticks, capped at 100. | ✅ | Shipped |
| **Villages with identity** | Neutral villages have names, fealty per faction, and four specialty types (`NeutralSpecialty`). Specialties are labels only; they do not yet change behaviour. | 🟡 | M1/M3 |
| **Reputation / honour** | No global reputation. Attacking neutrals has no diplomatic cost. | 📝 | M1 |
| **Deep espionage** | Four operations (infiltrate, siphon, sabotage, incite) with per-tick detection risk. No cover identities, bribery, coups, or counter-intelligence. | 🟡 | M2 |
| **General loyalty and mutiny** | Salaries (flat 15), loyalty drift, unpaid decay (−6/cycle), mutiny below 10. No ambition, succession, or civil war. | 🟡 | M2 |
| **Multi-ethnic demographics** | Five races with growth, food, combat, and defence multipliers, plus a per-settlement dominant race. No population mix or ethnic friction. | 🟡 | M3 |
| **Inns and recruitment** | Only the starting general per capital. No inns or wandering talent. | 📝 | M1 |
| **Fort levies and garrisons** | Supply roads move population into forts, capped by tier and by the supporting village. No soldier identity, morale, or per-unit equipment. | ✅ | Shipped (equipment is per fort) |
| **Markets and trade** | Factories consume ore and timber to make goods; markets sell goods for gold with a faction trade multiplier. No foreign demand, tariffs, or blockades. | 🟡 | M1 |
| **Tax vs loyalty** | Taxes are a fixed function of population and happiness. No player-controlled rate. | 📝 | M1 |
| **Civil unrest and rebellion** | Starvation collapses order; low loyalty plus low happiness can flip a settlement neutral. No emigration or organised secession. | 🟡 | M1 |
| **Multi-faction geopolitics** | Two real powers (`Player`, `Enemy`) plus neutrals and bandits. No alliances, treaties, or coalitions. | 📝 | M3 |
| **Visible unit flow** | Caravans are animated dots on road ribbons. Workers and soldiers do not march. | 🟡 | M4 |
| **Research unlocks** | Laboratories generate points; ages cost 120/300/650 and grant production and combat multipliers. Points do not unlock buildings or doctrines. | 🟡 | M2/M3 |
| **Greater powers / monoliths** | None. | 📝 | M3 |
| **Fog of war and scouts** | None; the whole map is visible. | 📝 | M3 |
| **Migration and soft annexation** | No migration. Peaceful annexation exists through fealty (trade) and influence (petition/foment/purchase). | 🟡 | M1/M3 |
| **Royal succession** | No ruler entity; the player is abstract. | 📝 | M2 |

## What was deliberately not copied

- **The 7K interface and presentation.** Node Empire is a graph with ribbons and
  egui panels, not an isometric sprite RTS.
- **Race rosters and lore.** The five Node Empire races are original multipliers;
  names and traits are not lifted from 7K.
- **The firm/building catalogue.** Node Empire has eight buildable node types
  chosen for the graph economy, not a reproduction of 7K's building list.
- **Difficulty through AI cheating only.** Node Empire's harder settings make
  the AI think more often and grants it income, but does not change player rules.

## Reading order for contributors

1. [`MECHANICS.md`](MECHANICS.md) — what exists.
2. [`../ROADMAP.md`](../ROADMAP.md) — what is scheduled, and in which milestone.
3. [`VISION.md`](VISION.md) — why the pillars are shaped this way.

If a proposal here is not in the roadmap, it is not planned. Add it to the
roadmap with a milestone and a test, or leave it in this file as research.
