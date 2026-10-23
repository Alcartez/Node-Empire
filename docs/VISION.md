# Node Empire — Vision

> **Design intent, not a feature list.** Nothing in this document is implemented
> unless [`MECHANICS.md`](MECHANICS.md) says so and [`../ROADMAP.md`](../ROADMAP.md)
> schedules it. This file exists so the long-term fantasy cannot be mistaken for
> the current build.

## The game we are aiming at

A living political map that runs in the background. You set the network — roads,
villages, forts, agents — and then respond to what the world does with it.
Peasants move to work, generals acquire ambitions, neutral courts weigh your
reputation against your neighbours', and a road is simultaneously a supply line,
a claim, and a channel for culture.

The player is a **sovereign, not a micromanager**. Skill is in designing a
network that can feed itself, defend itself, and absorb its neighbours without
being ordered to every few seconds.

## Four pillars

Every feature should pull on at least two pillars; a feature that pulls on none
is decoration.

### 1. Military

Garrisons, fort equipment, supply lines, and commanders. The interesting
question is never "how many soldiers" but "can the network behind them pay,
feed, and reinforce them". Over-mobilising should drain the workforce that pays
for the army.

### 2. Economy

Production chains, caravans, markets, and infrastructure. Roads are the unit of
investment. A richer network should be a more attractive network, which makes
economy a form of expansion, not just a score.

### 3. Happiness and order

Taxes, food, employment, garrisons, and civic identity. Neglect should produce
consequences the player can see: emigration, stalled growth, secession. Order
should be the quiet backbone that makes every other pillar work.

### 4. Espionage

Agents, cover, influence, and information. Spies should be an alternative to
armies, not a damage-over-time debuff: they steal, misinform, bribe, and
occasionally flip a fortress. Counter-intelligence should make paranoia
rational.

### The interlocks

- **Military ↔ Economy** — conscription removes workers; a big army starves the
  network that supports it.
- **Military ↔ Happiness** — garrisons suppress unrest, but long wars breed
  weariness.
- **Economy ↔ Happiness** — food security and festivals buy calm; high taxes
  buy gold and lose both.
- **Espionage ↔ everything** — a spy can substitute for an army in one place
  and destabilise the economy in another.

The intended failure mode of over-focusing is a crisis in a different pillar,
not a flat penalty.

## What "living world" means

- Neutral villages have identity, defence, and opinions. They can be traded
  with, wooed, intimidated, or conquered.
- Prosperous settlements attract people along roads; struggling ones lose them.
- Several AI courts scheme against each other, not only against the player.
- Succession and ambition create internal politics, so the player's own court is
  a source of drama.
- The map hides information: scouts and roads reveal it, and spies sell it.

## Design rules

1. **Connection is the unit of play.** If a mechanic can be expressed as a road,
   a route, or a network, prefer that over a radius or a global modifier.
2. **Automate the boring part, dramatise the choice.** Workers should find jobs;
   the player should decide where the jobs are.
3. **Every number needs a counter-pressure.** A bonus with no cost becomes the
   only correct play.
4. **The world should be legible.** A player should be able to explain a
   settlement's order or a general's loyalty from the panel, not a wiki.
5. **No content that cannot be tested or playtested.** If a system cannot be
   asserted headlessly, it needs a checklist step.
6. **Ship the politics before the polish.** A shallow game with good audio is
   still shallow.

## Explicitly not implemented (and therefore not promised)

- Asymmetric faction rosters (unique units, buildings, doctrines).
- More than one AI opponent, or any diplomacy between powers.
- Reputation, treaties, alliances, or a tax slider.
- Population migration, inns, or recruitable generals beyond the starter.
- Spy cover identities, general bribery, coups, or counter-intelligence
  networks.
- Fog of war, scouts, watchtowers, or map monoliths.
- Audio, localisation, multiplayer, or a scenario editor.

Each appears in a milestone in [`../ROADMAP.md`](../ROADMAP.md). Until it ships,
it belongs here and nowhere else.
