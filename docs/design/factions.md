# The Six Courts

> This file documents what the build actually uses. A court is a **profile of
> multipliers** — it does not add unique units, buildings, or abilities. Six
> more courts are sketched as future candidates at the bottom and are **not
> implemented**.

Source of truth: `src/game/types.rs` → `FACTIONS` and `Faction::native_race`.

## How a court works

Each profile supplies:

- **Combat** — battle power multiplier.
- **Production** — output multiplier for farms, camps, mines, factories, markets,
  and laboratories.
- **Spies** — espionage multiplier. It reduces detection risk (a higher value is
  better).
- **Defense** - multiplier applied to a settlement's defense in siege
  resolution; it does not apply to fort-vs-fort field battles.
- **Trade** — gold multiplier when markets sell goods.
- **Growth** — population growth multiplier.
- **Happiness** — a flat modifier applied to every settlement's target happiness.
- **Native race** — the race associated with the court. It does not restrict
  which races can live in your settlements.

## The courts

### The Silk Spinners

*"Every court has a thread we hold"* — violet `#B282FF`

| Combat | Production | Spies | Defense | Trade | Growth | Happiness |
|---:|---:|---:|---:|---:|---:|---:|
| 0.80 | 0.95 | **1.45** | 0.90 | 1.15 | 1.00 | 0 |

The espionage court. The best agents and the worst armies; war is something you
avoid by knowing everything first. Native race: Silk Spinner.

### The Breplid Clans

*"Peace is only the pause between raids"* — red `#EB6E5A`

| Combat | Production | Spies | Defense | Trade | Growth | Happiness |
|---:|---:|---:|---:|---:|---:|---:|
| **1.35** | 0.95 | 0.85 | 1.15 | 0.70 | 1.00 | −5 |

The war court. Strong attacks, solid walls, poor markets, and a population that
is never quite content. Native race: Human.

### The Hapset Remnant

*"Stone endures where blood does not"* — blue `#78C8EB`

| Combat | Production | Spies | Defense | Trade | Growth | Happiness |
|---:|---:|---:|---:|---:|---:|---:|
| 1.00 | 1.00 | 0.95 | **1.35** | 1.00 | 1.00 | **+10** |

The builder court. No glaring weakness, the strongest forts, and the happiest
subjects. Native race: Human.

### The Argo Combine

*"A well-run ledger wins wars"* — gold `#FFC85A`

| Combat | Production | Spies | Defense | Trade | Growth | Happiness |
|---:|---:|---:|---:|---:|---:|---:|
| 0.90 | **1.30** | 0.95 | 1.00 | **1.25** | 0.90 | −5 |

The industrial court. The strongest economy and the weakest population growth;
you buy your way to a network instead of growing into one. Native race: Human.

### The Mez Cabal

*"We already know what you will do"* — green `#6EE6B4`

| Combat | Production | Spies | Defense | Trade | Growth | Happiness |
|---:|---:|---:|---:|---:|---:|---:|
| 0.85 | 1.05 | **1.30** | 0.95 | 1.10 | 1.00 | 0 |

The manipulator court. Second-best spies, decent trade, and enough production to
fund both. Native race: Reptilian.

### The Hive Swarms

*"One is nothing, ten thousand is everything"* — orange `#F0823C`

| Combat | Production | Spies | Defense | Trade | Growth | Happiness |
|---:|---:|---:|---:|---:|---:|---:|
| 1.10 | 1.10 | 0.90 | 0.85 | 0.90 | **1.45** | −5 |

The swarm court. Fastest growth and above-average production, paid for with poor
defence and unhappy subjects. Native race: Hive Swarm.

## Summary

| Court | Combat | Prod | Spies | Def | Trade | Growth | Happy | Native race |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| Silk Spinners | 0.80 | 0.95 | 1.45 | 0.90 | 1.15 | 1.00 | 0 | Silk Spinner |
| Breplid Clans | 1.35 | 0.95 | 0.85 | 1.15 | 0.70 | 1.00 | −5 | Human |
| Hapset Remnant | 1.00 | 1.00 | 0.95 | 1.35 | 1.00 | 1.00 | +10 | Human |
| Argo Combine | 0.90 | 1.30 | 0.95 | 1.00 | 1.25 | 0.90 | −5 | Human |
| Mez Cabal | 0.85 | 1.05 | 1.30 | 0.95 | 1.10 | 1.00 | 0 | Reptilian |
| Hive Swarms | 1.10 | 1.10 | 0.90 | 0.85 | 0.90 | 1.45 | −5 | Hive Swarm |

## What a court does **not** change

- Build costs, road costs, unit rosters, or building availability.
- The rules of growth, order, fealty, or espionage — only the multipliers above.
- Diplomacy or reputation; neither exists yet.

## Candidate courts (not implemented)

These appear in older design notes. They are recorded here so the ideas are not
lost, but no code, stats, or art exist for them:

- **Beetle Soldiers** — heavy defensive infantry.
- **Mantis Hunters** — precision strikes and assassination.
- **Primates** — agility and research.
- **Ethereals** — psionics and vision.
- **Cephalopods** — strategy and counter-tactics.
- **Reptilian kingdoms** — poison and ambush (the Reptilian *race* exists; a
  court does not).

Adding one means more than a multiplier row: it needs a reason to pick it that
the other six do not already cover, a unique mechanic, and a test proving that
mechanic. Until then, the roster stays at six. See `ROADMAP.md` → "Cut / not
planned".
