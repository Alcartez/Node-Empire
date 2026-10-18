# Node Empire - MVP Specification

## Overview

**Node Empire** is an automated graph-based strategy game inspired by Seven Kingdoms: Ancient Adversaries. Build settlements, connect buildings via edges, and manage your kingdom through indirect control. No unit micro-management - systems run automatically. Combat is Tower War style: connect forts and they auto-fight using population from linked settlements.

**Platform**: Desktop (Rust + Egui + egui_graphs)
**Starting**: MVP-Ready in ~2-3 weeks of dev time

---

## Game Settings

| Setting | Value |
|---------|-------|
| Map size | 50×50 |
| Starting gold | 100 |
| Starting food | 100 |
| Starting population | 10 |
| Tick rate | 5 ticks/sec |
| Win threshold | 5000 gold |
| AI opponents | 1 enemy + 10 neutral villages |
| Faction differences | None (identical) |

---

## Resources

| Resource | Symbol | Start | Source | Use |
|----------|--------|-------|--------|-----|
| Gold | 🪙 | 100 | Tax from population | Building, hiring |
| Food | 🌾 | 100 | Farms | Population upkeep, hiring |
| Population | 👥 | 10 | Villages | Workers, soldiers |

---

## Nodes

### Settlement (🏠)
- **Function**: Population center, tax collection
- **Tiers**:
  - Village (0-500 pop): +1 farm capacity
  - Town (501-2000 pop): +2 farm capacity
  - City (2000+ pop): +3 farm capacity
- **Production**: Tax: 1 gold per population per 10 ticks
- **Unlocked buildings**: Farm, Fort

### Farm (🌾)
- **Function**: Food production
- **Placement**: Within edge range of a settlement, on suitable terrain
- **Production**: 5 food per farm per 10 ticks
- **Cost**: 50 gold

### Mine (⛏️)
- **Function**: Ore extraction
- **Placement**: Pre-generated neutral nodes (natural resources)
- **Production**: 2 ore per mine per 10 ticks
- **Requires**: Connection to settlement (workers)

### Factory (⚙️)
- **Function**: Process ore into goods
- **Input**: Ore from connected mines
- **Output**: Goods (sold for gold)
- **Production**: 1 good per 5 ore per 10 ticks
- **Sale**: 10 gold per good automatically
- **Cost**: 100 gold

### Fort (⚔️)
- **Function**: Defense, combat
- **Linked settlement**: Must connect to one settlement
- **Combat**: Pulls 1 population/tick from linked settlement
- **Defense bonus**: +10% per settlement tier
- **Cost**: 75 gold

---

## Edges

| Edge Type | Connection | Effect |
|----------|------------|--------|
| **Resource** | Any two buildings | Transfers resources between them |
| **Supply** | Settlement → Farm/Fort/Factory | Workers flow to buildings |
| **Combat** | Fort ↔ Fort | Auto-combat when connected |

**Edge rules**:
- Maximum edge length: 10 tiles
- Edges path around impassable terrain
- Resource transfer speed: 1 unit per (edge_length × 2) ticks

---

## Worker System (7K Style)

### Training
1. Select settlement
2. Choose job type (Miner, Manufacturer, Constructor, Soldier)
3. Wait 10 ticks
4. Worker exits with skill 20 (untrained = skill 10)

### Assignment
- **Direct**: Right-click worker → building = instant assignment
- **Via UI**: Select worker → click building in "Assign to" menu

### Job Types

| Job | Works At | Output | Skill Affects |
|-----|----------|--------|-------------|
| **Peasant** | Village | Tax, food | Production +0% |
| **Miner** | Mine | Ore | +5% per skill level |
| **Manufacturer** | Factory | Goods | +5% per skill level |
| **Constructor** | Any | Build/repair | +10% per skill level |
| **Soldier** | Fort | Combat | +5% per skill level |

### Skill Growth
- Trained workers: Start at 20, +1 per 10 ticks while working
- Untrained workers: Start at 10, +1 per 20 ticks while working

---

## Combat System (Tower War Style)

### Setup
1. Build fort
2. Connect fort to your settlement (supply edge)
3. Connect fort to enemy fort (combat edge)
4. Combat begins automatically

### Combat Flow
```
Tick 1-∞: Each fort pulls 1 population/tick from linked settlement
         Population in fort = "combatants"
         Higher defender tier = +10% defense per tier
         Higher skill = +5% attack power

Combat resolves at 1 damage per 2 combatants per tick
First fort to lose all linked population is captured
Captured forts can be converted to your faction
```

### Capture
- Empty fort (0 population from linked settlement) = captured
- Capture cost: 25 gold to reinforce
- Original owner can recapture by reconnecting

---

## Influence System

### Calculation
```
Influence = (King Rating × 10) + (Food Surplus × 5) + (Happiness × 2)
           - Distance decay (per tile from nearest settlement)
```

### Effects

| Influence | Effect |
|-----------|--------|
| 0-20 | None |
| 21-40 | +10 happiness to nearby neutrals |
| 41-60 | Neutrals may petition to join (free) |
| 61-80 | Can incite enemy rebellion (costs 100 gold) |
| 81-100 | Can buy enemy settlements (costs 200 gold) |

### King Rating
- +1: Gold increasing
- +0: Gold stable
- -1: Gold decreasing

---

## Procedural Map Generation

### Algorithm
1. **Terrain**: Generate noise-based terrain (grass, water, mountains)
2. **Neutral mines**: 8 pre-placed on ore deposits
3. **Neutral farms**: 10 on suitable terrain
4. **Player 1**: 1 settlement starting position (top-left)
5. **Player 2 (Enemy)**: 1 settlement starting position (bottom-right)
6. **Neutral settlements**: 10 scattered (not near players)
7. **Player buildings**: Start with empty built slots, 100 gold to build

### Map Constraints
- No placement on impassable terrain (water, mountains)
- Minimum distance between settlements: 15 tiles
- Edge pathfinding: A* around obstacles

---

## UI Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ 🪙 100 🌾 100 ⛏️ 0 ⚙️ 0 │ [?] │ Selected: None            │
├─────────┬───────────────────────────────────────────┬───────────┤
│ [Build] │                                           │ Events:   │
│ 🏠 Village │    ○ Village A ───○ Fort B            │           │
│ ⚔️ Fort    │    │            ⛏ Mine               │           │
│ ⚙️ Factory │    ○ Farm                             │           │
│ 🌾 Farm    │          ○ Factory ───○ Enemy Fort    │           │
├─────────┴───────────────────────────────────────────┴───────────┤
│ [Select a node to see details. Drag between nodes to connect.]    │
└─────────────────────────────────────────────────────────────────┘
```

### Panels

| Panel | Position | Contents |
|-------|----------|----------|
| Resources | Top-left | Gold, Food, Ore, Goods |
| Help [?] | Top-right | Help overlay |
| Build Menu | Left | Building buttons |
| Events | Right | Game messages |
| Status | Bottom | Selected node info, controls |

---

## Help Overlay

### [?] Help Contents

| Section | Text |
|---------|------|
| **Getting Started** | "You have 1 village. Build farms and mines to grow. Connect buildings with edges." |
| **Building** | "Select a building type, click on suitable terrain to place. Cost: Village 50, Farm 50, Fort 75, Factory 100" |
| **Connections** | "Drag from one building to another to create an edge. Resources flow along edges." |
| **Workers** | "Select village → Train → select job. Workers auto-assign if buildings are connected." |
| **Combat** | "Connect your fort to an enemy fort. They will auto-fight using population from linked settlements." |
| **Influence** | "High food and happiness attract neutral villages to join you peacefully." |
| **Winning** | "Reach 5000 gold, or capture all enemy settlements." |

---

## Win / Lose Conditions

| Condition | Trigger |
|-----------|---------|
| **Win (Gold)** | Gold >= 5000 |
| **Win (Domination)** | All enemy settlements captured |
| **Lose** | Player settlement count = 0 |

---

## Save / Load

- **Format**: JSON
- **Auto-save**: Every 60 ticks
- **Slots**: 3 manual saves + 1 auto

---

## Technical Stack

| Component | Technology |
|-----------|------------|
| App framework | eframe 0.33 |
| UI | egui 0.33 |
| Graph visualization | egui_graphs 0.6 |
| Serialization | serde + JSON |
| Random | rand 0.8 |

---

## Development Phases

### Phase 1: Core (Day 1-2)
- [ ] Project setup (Cargo.toml, main.rs)
- [ ] Egui + egui_graphs rendering
- [ ] Basic node types

### Phase 2: Map Gen (Day 3-4)
- [ ] Procedural terrain
- [ ] Node placement algorithm
- [ ] Edge generation with pathfinding

### Phase 3: Resources (Day 5-6)
- [ ] Gold/food/population tracking
- [ ] Production ticks
- [ ] Resource flow along edges

### Phase 4: Workers (Day 7-8)
- [ ] Training system
- [ ] Assignment system
- [ ] Skill growth

### Phase 5: Combat (Day 9-10)
- [ ] Fort linking
- [ ] Auto-combat
- [ ] Capture mechanics

### Phase 6: Influence (Day 11-12)
- [ ] Influence calculation
- [ ] Petition system
- [ ] Win/lose detection

### Phase 7: Polish (Day 13-14)
- [ ] Help system
- [ ] Save/load
- [ ] First playable!

---

## MVP Goals

- [ ] 50×50 playable map
- [ ] All node types functional
- [ ] Resource system working
- [ ] Worker training/assignment
- [ ] Combat auto-resolves
- [ ] Influence system active
- [ ] Win conditions check
- [ ] Help overlay works

---

*Last Updated: 2026-04-18*