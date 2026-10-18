# Node Empire - Design Notes

## Core Philosophy

**"Set the network, watch it grow, respond to events."**

Node Empire is not a traditional RTS where the player issues constant commands. Instead, the player functions as a **network architect and event responder**:
- Build and connect nodes (settlements, forts, mines, factories, research)
- Deploy automated systems (spies, trade routes, recruitment pipelines)
- Monitor the network passively as background processes run
- Intervene only when events or quests demand attention

The player's skill is in **designing efficient networks** and **strategic decision-making** during events—not micromanaging every unit.

---

## Player Experience

### What the Player Does

1. **Build the Network**
   - Place and connect nodes
   - Establish supply lines (edges)
   - Deploy spies at key points
   - Set up trade routes

2. **Configure Automation**
   - Set production priorities per settlement
   - Define spy objectives (siphon, sabotage, infiltrate)
   - Configure recruitment rates
   - Assign research focus

3. **Watch and Learn**
   - Observe resource flows along edges
   - Monitor spy network effectiveness
   - Track settlement growth
   - Watch AI and neutral factions

4. **Respond to Events**
   - Handle espionage revealed
   - React to coalition attacks
   - Decide on quest opportunities
   - Manage crises (bandit raids, rebellions)

### What the Player Does NOT Do

- Micro-manage individual units
- Click every soldier into position
- Manually trigger resource transfers
- Upgrade individual soldiers

---

## Automation Systems

### Background Processes (Automatic)

| System | Behavior |
|--------|----------|
| Resource Transfer | Goods flow automatically along edges based on priority settings |
| Population Migration | Citizens move from low-opportunity to high-opportunity settlements via edges |
| Spy Operations | Spies perform assigned tasks (siphon, infiltrate) automatically |
| Recruitment | Soldiers spawn at settlements based on configured rates |
| Research | Tech points accumulate, tiers unlock when threshold reached |
| Public Order Decay | Crime increases when unemployment is high and no forts nearby |
| Bandit Threat | Bandit camps slowly grow in threat level if ignored |

### Player-Triggered Actions

| Action | Trigger |
|--------|---------|
| Build Node | Click on map or via menu |
| Create Edge | Drag from node to node |
| Deploy Spy | Select node → Spy menu → Assign objective |
| Attack | Connect forts via edge → soldiers march |
| Espionage Operation | Select spy → Choose action (bribe, blackmail, sabotage) |
| Research Selection | Open research panel → Select tier/bonus |
| Quest Response | Event popup → Choose option |

---

## Event System

### Event Categories

| Category | Examples |
|----------|----------|
| **Espionage** | Spy discovered, agent defected, intelligence gathered |
| **Military** | Border conflict, settlement under attack, fort besieged |
| **Economic** | Trade route disrupted, resource shortage, gold influx |
| **Diplomatic** | Coalition formed, alliance offer, betrayal |
| **Internal** | Rebellion, crime wave, disease outbreak |
| **Quest** | Merchant request, refugee crisis, opportunity |

### Event Flow

```
1. Event Triggers
   ↓
2. Notification Appears (toast/panel)
   ↓
3. Player Reviews Options
   ↓
4. Player Chooses Response (or ignores)
   ↓
5. Consequences Execute
```

### Event Design Principles

- Events should **interrupt passive watching** only when meaningful
- Events should have **clear stakes** and **multiple valid responses**
- Some events should be **opportunities** (gain something) not just crises
- Ignoring events should have **consequences**

---

## Quest System

### Quest Types

| Type | Source | Description |
|------|--------|-------------|
| **Opportunity** | Random | Chance to gain resources, allies, or intel |
| **Crisis** | Random | Must respond or suffer penalties |
| **Contract** | NPC Factions | Complete task for reward |
| **Story** | Milestone-based | Major events tied to game progress |

### Quest Examples

| Quest | Trigger | Options | Reward |
|-------|---------|---------|--------|
| Refugee Crisis | Population decline in neighboring settlement | Accept (+pop, -food) / Ignore (-relations) / Attack (+pop, -relations) | Population |
| Merchant Guild | Trade edge established | Partner (trade bonus) / Tax (gold, -relations) / Refuse | Gold or relations |
| Bandit Bounty | Bandit camp threat level high | Hunt (-soldiers, +gold) / Bribe (-gold, -threat) / Ignore (risk raid) | Gold |
| Espionage Lead | Spy network detects weakness | Exploit (+intel) / Warn (+relations) / Ignore | Intel or relations |
| Coalition Invite | Neutral city coalition forming | Join (mutual defense) / Sabotage (-relations, prevent) / Ignore | Varies |

### Quest UI

- Quests appear in a **Quest Panel** (separate from settlement panel)
- Active quest notifications show in HUD
- Quest log accessible from menu
- Quest rewards/penalties previewed before commitment

---

## Network Visualization

Since the core gameplay is about networks, the UI must clearly show:

| Element | Visual |
|---------|--------|
| Nodes | Size = tier (village/town/city/megacity) |
| Edges | Lines with optional flow animation |
| Resource Flow | Color-coded or animated arrows along edges |
| Spy Networks | Dotted lines or special markers |
| Fort Coverage | Shaded regions showing public order influence |
| Impassable Terrain | Dark grey blocking movement |

### Edge States

| State | Visual |
|-------|--------|
| Active (transferring) | Animated flow in direction |
| Idle | Dim/static line |
| Blocked/Cut | Red X or gap |
| Under Attack | Pulsing red |

---

## Impassable Terrain

- **Color**: Dark grey
- **Behavior**: Edges cannot pass through; must path around
- **Generation**: Pre-defined in seeded maps; procedurally placed later
- **Strategic**: Creates natural chokepoints and territories

---

## UI Principles

### Key Tenets

1. **Map is Primary** - The map view dominates. UI panels dock to edges and don't obscure gameplay.
2. **Contextual Information** - Panels show relevant info for selected node. No global overlays unless essential.
3. **Automation Visibility** - Player should always know what their network is doing without clicking.
4. **Event Prominence** - Events demand attention without being spammy. Toasts + sound + quest panel.
5. **Set and Forget** - If player set it up correctly, systems run. Player only intervenes on events.

### Panel Layout

```
+--------------------------------------------------+
|  [Resources HUD: Food | Gold | Ore | Goods | Pop] |
+--------------------------------------------------+
|                                          [Minimap]
|  [Settlement/Node Panel]                        |
|  - Stats (pop, happiness, public order)         |
|  - Production Queue                             |
|  - Spy Assignment                              |
|  - Research Tab                                |
|                                          [Map]   |
|                                                  |
|                                                  |
+--------------------------------------------------+
|  [Event/Quest Panel - collapsible]              |
+--------------------------------------------------+
```

### Controls

| Action | Input |
|--------|-------|
| Pan | WASD, Arrow keys, or Middle-click drag |
| Zoom | Scroll wheel |
| Select Node | Left-click |
| Context Menu | Right-click on node |
| Deselect | ESC or Left-click empty |
| Open Menu | ESC or Menu button |

---

## Faction: Silk Spinners (MVP)

### Theme
Information networks, saboteurs, puppet masters

### Base Bonuses
| Bonus | Effect |
|-------|--------|
| Intelligence | +40% spy intelligence gathering |
| Network | +3 spy capacity per settlement |
| Sabotage | +50% sabotage efficiency |
| Trap | +30% trap effectiveness |

### Base Weaknesses
| Weakness | Effect |
|----------|--------|
| Direct Combat | -35% soldier attack power |
| Fort Defense | -20% fort defense |
| Public Order | -15% public order |
| Population | -25% population capacity |

### Visual Style
Web-like patterns, silver/white colors, delicate appearance

### MVP Relevance
- Forces espionage-first gameplay
- Spies are the primary offensive tool
- Network visualization is core to faction identity
- Player learns to build automated spy networks

---

## Win Conditions (MVP)

### Economic (Gold)
- Accumulate configurable gold threshold
- Default: 10,000 (Short) / 25,000 (Medium) / 50,000 (Long)
- Threshold set before game start via slider

### Military (Domination)
- Capture all enemy capitals
- Capital = starting settlement of each faction
- All capitals owned = victory

---

## MVP Scope Summary

| Element | Scope |
|---------|-------|
| Factions | 1 (Silk Spinners) |
| Map | Seeded, pre-built generator, 50x50 |
| Win Conditions | Economic (Gold), Military (Domination) |
| AI | Passive only (bandits, neutral cities) |
| Automation | Full background systems |
| Events | Basic event system + quests |
| UI | Standard RTS controls, docked panels |

---

## Deferred Features

| Feature | Phase |
|---------|-------|
| Procedural terrain generation | Later |
| Multiple factions | Phase 7+ |
| Active AI opponents | Phase 6 |
| Espionage victory condition | Phase 7 |
| Alliances/diplomacy UI | Phase 7 |
| Coalition system | Phase 8 |
| Multiplayer | Phase 9 |

---

*Last Updated: 2026-04-06*
