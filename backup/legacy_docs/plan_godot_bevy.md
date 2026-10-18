# Node Empire - Game Development Plan

## Project Overview

**Working Title**: Node Empire  
**Engine**: Godot 4 + Bevy (hybrid)  
**Multiplayer**: Phase 2 (up to 4v4 = 8 players)  
**Platform**: Desktop (Windows, Linux, Mac)  
**Release Strategy**: Single-player first, then online multiplayer  
**Visual Style**: Minimalist/Stylized (solo dev friendly)  
**Technology Era**: Medieval

---

## Game Setup

| Setting | Value |
|---------|-------|
| Grid Type | Square tiles |
| Map Size | 50x50 to 100x100 (configurable) |
| Players | 1-8 (human + AI, max 4v4) |
| Factions | 12 total (see `factions.md`) - **MVP: 1 faction** |
| Starting Resources | 100 food, 100 gold |

---

## MVP Scope

| Element | MVP Decision |
|---------|-------------|
| Factions | 1 humanoid faction (TBD - will refine factions later) |
| Map | Pre-built generator with fixed seeds (no procedural yet) |
| Win Conditions | 2: Gold (Economic) + Military (Domination) |
| Tech Tree | Equipment tiers (Wood→Copper→Iron→Steel) + economic/spy bonuses |
| Visuals | Color-coded terrain, resource icons, nodes as settlements, edges between them |
| AI | Simple/passive (MVP) |
| Multiplayer | Not in MVP |

---

## Factions

See [factions.md](./factions.md) for complete faction details including:
- 12 playable factions (4 human, 4 humanoid, 4 fantasy)
- Race relationship matrix (xenophobia/xenophilia)
- Base racial bonuses and debuffs

**Note**: MVP uses 1 humanoid faction. Factions will be refined later.

---

## Win Conditions (All Configurable On/Off)

1. **Domination (Military)**: Capture all enemy capitals
2. **Economic (Gold)**: Accumulate configurable wealth threshold (player sets before game)

---

## City/Territory System

### City Tiers

| Tier | Population Range | Visual Size | Description |
|------|-----------------|-------------|-------------|
| **Village** | 0-500 | Small | Starting point, minimal production |
| **Town** | 501-2,000 | Medium | Basic infrastructure, some production |
| **City** | 2,001-10,000 | Large | Full production, can build forts |
| **Megacity** | 10,001+ | Very Large | Maximum production, multiple forts |

### City Stats

| Stat | Description |
|------|-------------|
| Population | Current residents |
| Population Growth | Rate based on happiness and resources |
| Happiness | Affected by food, gold, public order, neighbor races |
| Public Order | Affected by: employment, forts nearby, crime (no jobs = crime) |
| Public Order Mechanic | Settlement without employment or forts nearby → crime → public order drops by 15% |
| Manufactured Goods | Clay pots, iron utensils, soaps, bread, etc. |

### City Growth Mechanics

- **Resource-driven**: Growth based on food, gold, and manufactured goods transported to city
- **Population migration**: People move from low-opportunity settlements to high-opportunity ones via edges
  - Settlement A with jobs + Settlement B without jobs = population migrates B → A
  - Cannot exceed megacity population limit
- **Time-based**: Natural growth possible but slow without resource investment
- **Growth rate**: Affected by happiness and public order

---

## Node Types

| Node | Produces | Notes |
|------|----------|-------|
| Settlement | Population, Food, Gold | Can grow to Village/Town/City/Megacity |
| Fort | Military | Public order in region, defense bonus, equipment upgrade (blacksmith) |
| Mine | Ore | |
| Factory | Manufactured Goods | Converts ore to products |
| Farm | Food | |
| Port | Trade | Sea routes (future expansion) |
| Research Building | Tech Points | Unlocks tech tree tiers and bonuses |

---

## Neutral Cities

### Behavior
- **Independent**: Not owned by any player faction
- **Incitement**: Can be incited to attack other neutral factions or player factions
- **Coalitions**: Neutral cities near each other can randomly form coalitions (trade + defense alliances)

### Coalition System
- **Formation**: Random event-based when 3+ neutral cities are close
- **Benefits**: Trade bonus, mutual defense
- **Threats**: Can attack players who siphon resources via espionage

### Hostile Actions Against Players
- Attack if player spies siphon resources from them
- Attack if player owns fort nearby (feel threatened)
- Attack based on race relationships (xenophobia/xenophilia)

---

## Bandit/Rogue System

### Bandit Camps
- **Spawn**: Generated on map during map generation
- **Behavior**: Attack nearby settlements, siphon resources
- **Threat Level**: Increases over time if not dealt with

### Rogue Settlements
- **Transformation**: Neutral cities can go rogue
- **Triggers**: Low public order, player espionage abuse, threats nearby

---

## Fort System

### Defensive Functions
- **Regional Public Order**: Improves public order in connected settlements
- **Rebellion Prevention**: Reduces chance of settlement rebellion
- **Defense Bonus**: Against enemy attacks (+fort advantage in combat)

### Combat Benefits
- **Fort Advantage**: Positional bonus for defenders
- **Recruitment**: Continuous soldier recruitment during battle
- **Equipment**: Blacksmith upgrade allows better weapons based on ore quality

---

## Resource System

### Resource Types

| Resource | Source | Usage |
|----------|--------|-------|
| Food | Farms, Settlements | Soldier maintenance, population growth |
| Gold | Settlements, Trade | Upgrades, espionage, diplomacy |
| Population | Settlements | Recruiting soldiers |
| Ore | Mines | Weapon/armor production |
| Manufactured Goods | Factories | Weapons, siege equipment |
| Tech Points | Research Buildings | Unlocking tech tree |

### Edge Mechanics

- **Connect nodes** via supply lines
- **Transfer speed** = `base_speed / distance`
- **Longer edges** = slower supply
- **Cut enemy edges** to disrupt supply chain
- **Population migration** via edges (job opportunities)

---

## Tech Tree

### Progression Structure

Research building efficiency determines research speed. Tiers unlock equipment and bonuses.

| Tier | Equipment | Bonuses |
|------|-----------|---------|
| 1 - Wood Age | Wood weapons | Basic production |
| 2 - Copper Age | Copper weapons | +10% production |
| 3 - Iron Age | Iron weapons | +20% production, basic espionage |
| 4 - Steel Age | Steel weapons | +30% production, advanced espionage |

### Research Mechanic

- **Research Building**: Required in settlement to generate tech points
- **Efficiency**: Higher-tier research buildings = faster research
- **Tech Points**: Accumulated to unlock tiers and bonuses

---

## Combat System

### Flow
1. Player connects Fort A → Fort B via edge
2. Soldiers travel along edge toward enemy fort
3. Arrival triggers combat
4. Combat resolves continuously until one side retreats or is destroyed

### Combat Rules
- **Defender advantage**: Fort bonus for positional defense
- **Attack types**:
  - **Siege**: Slow, high damage, requires manufactured goods
  - **Rush**: Fast, low damage
  - **Ranged**: Reduces defender bonus, requires manufactured goods
- **Equipment quality**: Determined by highest ore available at fort (blacksmith upgrade)
- **Continuous recruitment**: During battle based on supply line from settlement

### Loss Conditions
- Supply line cut → no new soldiers, existing troops retreat/die
- Settlement runs out of deployable population
- All forts lost = eliminated from game

---

## Espionage System

| Action | Effect | Cost | Requirements |
|--------|--------|------|--------------|
| Insert Spy | Place agent in enemy node | Medium | Spy unit |
| Bribe General | Unit defects to your side | High | General present, high gold |
| Blackmail General | Fort switches sides (win condition!) | Very High | Leverage on general |
| Sabotage | Destroy production/resources | Medium | Spy in position |
| Assassinate | Kill enemy leader/general | Very High | Spy in position |
| Steal Tech | Copy unit upgrades | High | Spy in factory |
| Infiltrate | Reveal enemy positions | Low | Spy in position |
| Incite | Make neutral city attack target | Medium | Spy in position |
| Siphon Resources | Steal gold/resources over time | Low | Spy in position |

---

## Diplomacy System

### Race Relationships
- **Xenophobia/Xenophilia**: Based on faction interactions and random events
- **Relations scale**: -100 (hostile) to +100 (allied)
- **Affected by**: Wars, trade, espionage, shared enemies, race compatibility
- See [factions.md](./factions.md) for full race relationship matrix

### Agreements

| Agreement | Effect |
|-----------|--------|
| Alliance | Mutual defense pact |
| Trade Agreement | Resource sharing |
| Treaty | Ceasefire |
| War | Open conflict |
| Coalition | Neutral city defense + trade alliance |

---

## AI System

### Personalities
| Personality | Behavior |
|-------------|----------|
| Aggressive | Prioritizes military, attacks frequently |
| Defensive | Fortifies positions, builds walls |
| Economic | Focuses on resource accumulation, economic victory |
| Balanced | Mix of all strategies |

### Difficulty Levels
| Level | Description |
|-------|-------------|
| Easy | Suboptimal decisions, slower reactions |
| Medium | Standard behavior |
| Hard | Optimal decisions, faster reactions |
| Unfair | Bonus resources, advanced knowledge |

### Implementation
- Behavior trees for decision making
- Up to 8 AI opponents (4v4 max)
- AI handles all systems: military, espionage, diplomacy, city growth

---

## Save System

- **Format**: JSON
- **Local**: File-based saves in standard game save directory
- **Cloud**: Steam Cloud integration
- **Features**: Multiple save slots, auto-save, save/load during gameplay

---

## Technical Architecture

### Engine & Dependencies

```
Godot 4 (UI, scene management, Godot-native features)
└── Bevy (core game logic, ECS, rendering for game world)
```

**Godot 4** handles:
- UI (Control nodes)
- Scene management
- Input handling
- Editor integration

**Bevy** handles:
- Game world rendering
- ECS architecture
- Game logic systems

### ECS Architecture

```
World
├── Faction (12)
│   ├── Race Type (human, humanoid, fantasy)
│   ├── Resources
│   ├── Race Bonuses/Weaknesses
│   ├── Diplomacy
│   ├── Visibility
│   └── Win Condition Progress
├── Node (Entities)
│   ├── Type (Settlement, Fort, Mine, Factory, Farm, Port, Research)
│   ├── Owner (Player, Neutral, Bandit)
│   ├── Tier (Village/Town/City/Megacity)
│   ├── Population
│   ├── Happiness
│   ├── Public Order
│   ├── Production Rates
│   └── Upgrades
├── Edge (Connections)
│   ├── Source Node
│   ├── Target Node
│   ├── Resource Flow
│   ├── Population Migration
│   └── Military Movement
├── Unit
│   ├── Soldiers
│   ├── Generals (leadership bonus)
│   └── Spies
├── Combat
│   ├── Active Battles
│   ├── Troop Movement
│   └── Resolution
├── Neutral Factions
│   ├── Cities
│   └── Coalitions
├── Bandit Camps
├── Tech Tree
│   ├── Current Tier
│   ├── Research Progress
│   └── Unlocked Bonuses
└── Game State
    ├── Time
    ├── Win Conditions
    ├── Victory/Loss
    └── Pause State
```

### Key Modules

| Module | Responsibility |
|--------|----------------|
| `map_gen` | Seeded terrain generation, node placement, edge creation |
| `city_system` | City tiers, growth, population migration, happiness |
| `resource_system` | Production, consumption, edge transfer |
| `tech_system` | Research building, tech tree progression, equipment tiers |
| `combat_system` | Battle resolution, soldier movement |
| `espionage_system` | Spy actions, incitement, siphoning |
| `diplomacy_system` | Alliances, treaties, race relations |
| `neutral_ai` | Neutral city behavior, coalition formation |
| `bandit_system` | Bandit camps, rogue settlements |
| `ai` | Single-player opponents (behavior trees) |
| `networking` | Multiplayer sync (Phase 2) |
| `ui` | Menus, map view, unit commands |
| `save_system` | JSON save/load, Steam Cloud |

---

## UI Elements

### MVP UI Elements

| UI Element | Priority | Description |
|------------|----------|-------------|
| Main Menu | Required | Start Game, Load Game, Options, Quit |
| Map Viewport | Required | Main game view with pan/zoom |
| Resources HUD | Required | Always visible top bar (Food, Gold, Ore, Goods, Pop) |
| Settlement Panel | Required | Side panel on settlement click - stats + production |
| Production Queue | Required | In settlement panel - soldiers, buildings, research |
| Research Panel | Required | In settlement panel (tab) - tech tree/flat list |
| Context Menu | Required | Right-click settlement for options |
| Win Condition Tracker | Required | Progress display for Gold/Military victory |
| Save/Load | MVP | Main menu save slots |
| Minimap | MVP | Useful for large maps |

### Deferred UI Elements

| UI Element | Phase | Description |
|------------|-------|-------------|
| Espionage Panel | Later | Spy management, actions |
| Diplomacy Panel | Later | Relations, treaties, alliances |
| Combat View | MVP | Battle UI during combat |
| Settings Menu | MVP | Audio, graphics, keybinds |

---

## Visual Style

### Terrain
- **Color-coded terrain types**: Grass, desert, forest, water, etc.
- **Impassable terrain**: Dark grey (edges must path around)
- **Resource deposits**: Distinct marks/icons for iron, gold, clay, copper

### Nodes
- **Settlements**: Visually represented as nodes with size indicating tier
- **Forts**: Military-style markers
- **Mines**: Pickaxe/ore icons
- **Factories**: Gear/smoke icons
- **Farms**: Wheat/crop icons
- **Research Buildings**: Book/tower icons

### Edges
- **Visual lines** connecting nodes
- **Path around impassable terrain**
- **Transfer animations** for resources

---

## Development Phases

### Phase 1: Core Engine (Weeks 1-4)
- [ ] Godot 4 + Bevy integration setup
- [ ] Square grid system
- [ ] Seeded terrain generation (fixed seeds, no procedural yet)
- [ ] Node placement (seeded algorithm)
- [ ] Edge creation system (pathfinding around impassable terrain)
- [ ] Bandit camp spawning (seeded)
- [ ] Camera controls (pan, zoom)

### Phase 2: City System (Weeks 5-8)
- [ ] City tiers (village → town → city → megacity)
- [ ] Visual size scaling
- [ ] Population system
- [ ] Happiness and public order
- [ ] Crime mechanic (no employment/forts = crime = -15% public order)
- [ ] Population migration via edges
- [ ] City growth mechanics
- [ ] Neutral city generation

### Phase 3: Resource System (Weeks 9-12)
- [ ] Resource types and tracking
- [ ] Edge-based resource transfer
- [ ] Distance-based transfer speed
- [ ] Supply line management
- [ ] Manufactured goods economy
- [ ] UI for resource display

### Phase 4: Research & Tech Tree (Weeks 13-16)
- [ ] Research building
- [ ] Research efficiency system
- [ ] Equipment tiers (Wood → Copper → Iron → Steel)
- [ ] Economic bonuses per tier
- [ ] Espionage bonuses per tier
- [ ] Research UI panel

### Phase 5: Military System (Weeks 17-20)
- [ ] Settlement → Fort soldier pipeline
- [ ] Edge-based troop movement
- [ ] Combat resolution system
- [ ] Fort defensive bonus
- [ ] Siege, rush, ranged attack types
- [ ] Equipment system (ore quality)
- [ ] Blacksmith upgrade
- [ ] Combat UI

### Phase 6: Single-Player AI (Weeks 21-24)
- [ ] Behavior tree framework
- [ ] AI personalities (aggressive, defensive, economic, balanced)
- [ ] Difficulty scaling (easy, medium, hard, unfair)
- [ ] Up to 8 AI opponents
- [ ] AI decision making for all systems
- [ ] AI testing and balancing

### Phase 7: Espionage & Diplomacy (Weeks 25-28)
- [ ] Spy unit system
- [ ] All espionage actions (bribe, blackmail, sabotage, etc.)
- [ ] Incitement of neutral cities
- [ ] Resource siphoning
- [ ] Diplomacy relations system
- [ ] Race relationships (xenophobia/xenophilia)
- [ ] Alliances, treaties, trade agreements
- [ ] Espionage victory condition
- [ ] UI for espionage and diplomacy

### Phase 8: Neutral & Bandit AI (Weeks 29-32)
- [ ] Neutral city behavior
- [ ] Coalition formation (random events)
- [ ] Neutral city attacks on players
- [ ] Bandit camp behavior
- [ ] Rogue settlement transformation
- [ ] AI responses to threats

### Phase 9: Multiplayer (Weeks 33-36)
- [ ] Godot multiplayer integration
- [ ] Server-authoritative architecture
- [ ] 4v4 networking
- [ ] Lobby system
- [ ] State synchronization
- [ ] Multiplayer testing

### Phase 10: Save System & Polish (Weeks 37-40)
- [ ] JSON save/load
- [ ] Steam Cloud integration
- [ ] Multiple save slots
- [ ] Auto-save
- [ ] UI polish
- [ ] Visual effects
- [ ] Balance testing
- [ ] Bug fixes

---

## Key Design Decisions

1. **Godot 4 + Bevy hybrid**: Godot handles UI/editor, Bevy handles game core
2. **Single-player first**: AI opponents before multiplayer to validate game mechanics
3. **Seeded maps first**: Pre-built generator with fixed seeds, procedural generation later
4. **Server-authoritative networking**: Prevents cheating in multiplayer
5. **JSON saves**: Human-readable, easy to debug
6. **Square grid**: Simpler implementation than hex, good for node-based gameplay
7. **Configurable win conditions**: Player flexibility
8. **Complex economy**: Multiple resource types and manufactured goods
9. **Race relationships**: Dynamic xenophobia/xenophilia based on interactions
10. **MVP focused**: Ship playable demo before adding complexity

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Scope creep | Strict phase boundaries, MVP first |
| Godot+Bevy integration complexity | Prototyping integration early in Phase 1 |
| Multiplayer complexity | Single-player first, validate mechanics |
| AI difficulty | Behavior trees with configurable parameters |
| Performance | ECS architecture, efficient rendering |
| Solo dev burnout | Realistic timeline, modular development |

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Frame Rate | 60 FPS on low-end hardware |
| Unit Count | Supports 1000+ units on map |
| Memory | Reasonable save file sizes |

---

## Next Steps

1. Prototype Godot + Bevy integration
2. Implement map generation with fixed seeds
3. Build city system with tiers and growth
4. Create basic resource and military systems
5. Add research and tech tree
6. Implement AI opponents
7. Add espionage and diplomacy
8. Integrate multiplayer
9. Polish and save system

---

*Last Updated: 2026-04-06*
