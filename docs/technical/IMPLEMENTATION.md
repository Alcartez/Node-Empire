# Node Empire - Complete Implementation Plan

## Overview
Full implementation for an automated graph-based strategy game with 7K-style worker mechanics.

---

## Game Settings (Unchanged)
- Map: 50x50
- Tick: Every frame (~60/sec)
- Win: 5000 gold

---

## Phase 1: Foundation

### 1.1 Resources (New)
Add to `GameState`:
- `copper: i32` - per node
- `iron: i32` - per node  
- `firewood: i32` - per node
- `planks: i32` - per node
- `copper_goods: i32` - per node
- `iron_goods: i32` - per node

### 1.2 Node Types (New)
Add to `NodeType`:
```rust
enum NodeType {
    Settlement,
    Farm,
    Mine,      // Generic - will specify ore type
    CopperMine, // NEW
    IronMine,   // NEW
    Woodcamp,   // NEW - forest
    Factory,
    PlankIndustry, // NEW
    CopperFactory, // NEW
    IronFactory,  // NEW
    Market,    // NEW
    Fort,
}
```

### 1.3 Ore Types
```rust
enum OreType {
    None,
    Copper,
    Iron,
}
```

Add to `GameNode`:
- `ore_type: OreType`

### 1.4 Forest Placement
Update `generate_nodes()`:
- Random forests (like ores) on 8% of tiles
- Woodcamps spawn on forests
- 2 Copper mines (more common)
- 1 Iron mine (less common)

### 1.5 Connection Cost Formula
```rust
fn get_connection_cost(from: &GameNode, to: &GameNode) -> i32 {
    let distance = ((from.x - to.x).powi(2) + (from.y - to.y).powi(2)).sqrt() as i32;
    5 + distance
}
```

---

## Phase 2: Food & Growth

### 2.1 Local Food System
Remove global `player_food`. Each settlement has `food: i32`.

**Consumption:**
```rust
// 10 citizens eat 1 food per 10 ticks
let consumption = node.population / 100; // integer division
if node.food >= consumption {
    node.food -= consumption;
} else {
    // Starving - lose population
    node.population = (node.population - 1).max(0);
}
```

### 2.2 Farm Production
- Farm produces 10 food per tick
- Can connect to 1 settlement (1:1)
- Multiple farms can supply one settlement

### 2.3 Settlement Tiers
| Tier | Name | Pop Range | Radius |
|------|------|-----------|--------|
| 0 | Camp | 0-99 | 1.0x |
| 1 | Village | 100-999 | 1.25x |
| 2 | Town | 1000-5999 | 1.50x |
| 3 | Minor City | 6000-19999 | 1.75x |
| 4 | Major City | 20000+ | 2.0x |

**Radius affects:** visual size, max farms allowed

### 2.4 Farm Distance Constraint
Farm must be at least 3 units from settlement for growth potential.

---

## Phase 3: Industries & Workers

### 3.1 Industry Chain

| Resource | Input | Output |
|----------|-------|--------|
| Farm | - | Food |
| Woodcamp | - | Firewood |
| PlankIndustry | Firewood + Settlement | Planks |
| CopperFactory | Copper | CopperGoods |
| IronFactory | Iron | IronGoods |

### 3.2 Worker System

**Training Costs:**
```rust
const WORKER_TIERS: &[(i32, i32, &str)] = &[
    (10, 50, "Basic"),    // skill, cost, name
    (40, 200, "Specialist"),
    (80, 400, "Expert"),
];
```

**Training Time:**
- 5 ticks per skill point
- Basic: 50 ticks, Specialist: 200 ticks, Expert: 400 ticks

**Worker struct:**
```rust
struct Worker {
    skill: i32,
    training_end_tick: u64,
    target_tier: i8,
}
```

**Auto-replacement:**
- New worker replaces lowest skilled at linked factory
- On tie → lowest production

### 3.3 Worker UI
Add to left panel when factory selected:
```rust
if ui.button("Train Basic (10g)").clicked() {
    start_training(node.id, 10, 10);
}
if ui.button("Train Specialist (40g)").clicked() {
    start_training(node.id, 40, 40);
}
if ui.button("Train Expert (80g)").clicked() {
    start_training(node.id, 80, 80);
}
```

### 3.4 On Disconnect
```rust
// Workers return to settlement, reset to skill 10
for worker in &mut node.workers {
    worker.skill = 10;
}
```

---

## Phase 4: Markets (If Time Permits)

### 4.1 Market Building
```rust
enum NodeType {
    // ... existing ...
    Market,
}
```

### 4.2 Population Control
Toggle: "Cap population at X"
- Excess food sold at market rate

### 4.3 Influence
- Neutral settlement receives food/goods → influence increases
- At threshold → may flip to player

---

## Implementation Order

### Execute via subagents:
1. Add new node types and resources
2. Update map generation (forests, copper, iron)
3. Implement connection cost formula
4. Local food system + consumption
5. Settlement tiers
6. Industry chain
7. Worker training UI
8. Auto-replacement logic
9. Market (if time)

---

## Key Formulas Reference

| Formula | Value |
|---------|-------|
| Connection cost | `5 + distance` |
| Food consumption | `population / 100` per 10 ticks |
| Training time | `skill × 5` ticks |
| Farm production | 10 food/tick |
| Worker replacement | Lowest skill, then lowest production |

---

*Last Updated: 2026-04-18*