# Node Empire: The Seven Kingdoms Soul & Idle Economy Overhaul (`changes.md`)

## Executive Summary & Vision

**"An Idle Empire Engine with a Game-of-Thrones Heart"**

Node Empire will combine the frictionless satisfaction of an **idle/incremental economy** (settlements expand, roads hum, caravans deliver, revenue trickles in reliably) with the high-stakes personal drama of **Seven Kingdoms** (named generals, fragile loyalty, mutinies, audacious espionage, and moral reputation).

The player will **not** micro-manage every caravan or peasant. Instead, the economy runs automatically like a well-oiled machine, freeing the player’s active attention for:
1. **Generals & Military Command** (Appointing commanders, paying wages, managing egos).
2. **Loyalty & Mutiny Prevention** (Preventing your generals from flipping rogue or defecting).
3. **Audacious Espionage** (Infiltrating foreign forts, bribing enemy generals, counter-intelligence).
4. **Reputation & High-Stakes Diplomacy** (Annexing villages peacefully or dealing with world coalitions).

---

## 1. Immediate Change: Starting Fort & Starting General

### Current State
* The player starts with 1 Settlement and nearby unowned resource nodes.
* A fort must be manually unlocked, paid for (150 gold), built, connected, and staffed from scratch before order stabilization kicks in.

### The Change
* **Guaranteed Starter Fort**: Every viable new game start spawns with:
  * 1 Capital Settlement (Pop: ~30, Food: ~60, Gold: ~300).
  * 1 Fort placed within 3–6 tiles of the capital.
  * 1 pre-built **Supply Road** connecting the Capital to the Fort.
  * **Starting General**: The fort is commanded from tick 1 by the King’s Champion / Founding General (e.g. *General Kaelen*, Leadership: 65, Combat: 60, Loyalty: 90).
  * **Starting Garrison**: 5 trained soldiers already stationed in the fort.
* **Gameplay Impact**:
  * Gives the player an immediate military stronghold, territorial presence, and civic order boost from second one.
  * Establishes the player's personal attachment to their military leadership immediately.

---

## 2. Generals & Leadership System

### Data Structure (`src/game/types.rs`)
Add a first-class `General` entity to the simulation:

```rust
pub struct General {
    pub id: u32,
    pub name: String,
    pub owner: Owner,
    pub fort_id: Option<u32>,        // Fort currently commanded
    pub leadership: u8,              // 1–100: Boosts garrison combat & order in supplied towns
    pub combat: u8,                  // 1–100: Personal prowess in battle & siege defense
    pub loyalty: u8,                 // 0–100: Fragile allegiance to the crown
    pub salary: i32,                 // Gold upkeep per 100 ticks (e.g., 10–25 gold)
    pub race: Race,                  // General's biological race
    pub kills: u32,                  // Track record and prestige
}
```

### Core General Mechanics
1. **Command Radius & Order**:
   * A fort's civic order bonus (`ORDER_FORT_BONUS`) scales with the General's `leadership`:
     $$\text{Fort Order Bonus} = 15.0 + 10.0 \times \left(\frac{\text{leadership}}{100}\right)$$
2. **Combat Effectiveness**:
   * Troops commanded by a high-leadership/combat general deal up to $+50\%$ damage and have higher morale in sieges.
3. **Salary & Loyalty Dynamics**:
   * **Loyalty Decay**: If the kingdom runs out of gold to pay general salaries, loyalty drops by $-2$ every pay cycle.
   * **Loyalty Boosts**: Granting gold gifts (bribes/bonuses), winning battles, assigning prestigious posts, and maintaining high Kingdom Reputation (+1 to +5).

---

## 3. Macro Race System (Demographics Without Tedious Micro)

Rather than forcing players to micro-manage individual peasants across fields and mines, **Races operate at the macro level (Settlements, Generals, Conscription, and Espionage)**.

### Core Philosophy: "Automated Towns, Strategic Appointments"
- The daily labor and reproduction run completely on **idle autopilot**.
- The player interacts with racial demographics through **3 high-impact strategic levers**:
  1. **Who governs the fort?** (Matching General race vs. Settlement race).
  2. **Who do you conscript?** (Racial troops drafted into the garrison).
  3. **Who do you send as a spy?** (Matching operative race to infiltrate foreign hives).

### 1. Settlement Dominant Race (`dominant_race: Race`)
Each settlement has a native race determining its baseline idle characteristics:
* **Humans (Argo / Hapset)**: Balanced economy, fast building speed, versatile military.
* **Beetlefolk**: Natural chitin armor, slower population growth, exceptional mining & fort construction.
* **Hive Swarms**: High reproduction rate (idle population explodes), lower individual combat stats, cheap swarm conscription.
* **Silk Spinners**: Exceptional loom production, automated intelligence gathering along connected roads, lethal web traps.
* **Reptilians**: Cold-blooded metabolic efficiency (lower food consumption), poison strikes, superior counter-espionage.

### 2. Racial Governance & Friction
When a Fort protects a settlement:
* **Same-Race Harmony**: If the General's race matches the settlement's race:
  * $+20$ General Loyalty.
  * $+15\%$ Public Order and tax compliance.
* **Alien / Foreign Rule Friction**: If an alien General governs a conquered race:
  * $-15\%$ Public Order; unrest slowly builds if taxes are high.
  * Conquered peoples may petition for a governor of their own kind.
  * Appointing a native collaborator general instantly pacifies the region!

### 3. Racial Conscription & Military Units
* Conscripting troops from a fort drafts soldiers from the **connected settlement's race**:
  * Drafting from a **Beetle** town produces heavy carapace infantry (high defense).
  * Drafting from a **Hive** town produces swarms of rapid strike troops.
  * Drafting from a **Human** town produces disciplined line soldiers.
* The player does not micromanage individual soldier gear; the unit identity comes organically from the demographic map!

---

## 4. Equipment, Arms & Blacksmith Logistics (Firms to Forts)

In *Seven Kingdoms*, weapons were physically forged in Blacksmiths/Factories from iron or bronze and supplied to forts. To prevent tedious micro-inventory, equipment operates at the **Fort & Garrison tier level**:

### 1. Automated Supply Chain (Mine → Factory → Fort)
* `Mine` (extracts Copper or Iron Ore) $\xrightarrow{\text{Road}}$ `Factory / Blacksmith` (forges Weapons & Armor) $\xrightarrow{\text{Supply Road}}$ `Fort`.
* Forts passively consume arms from connected blacksmiths to upgrade their equipment tier.
* No individual soldier inventory management required—the garrison upgrades as a collective regiment!

### 2. Fort Equipment Tiers
| Tier | Material / Arms | Combat Damage | Defense / Armor | Special Battlefield Effect |
|:---:|:---:|:---:|:---:|:---|
| **0** | **Militia / Wood** | $1.0\times$ (Base) | $1.0\times$ (Base) | Peasant levymen with wooden clubs and sharpened staves. |
| **1** | **Copper / Bronze** | $+25\%$ | $+15\%$ | Bronze cuirass, copper spearheads. |
| **2** | **Iron-Clad** | $+60\%$ | $+35\%$ | Iron broadswords, mail armor; halves incoming skirmisher/arrow damage. |
| **3** | **Folded Steel & Siege** | $+100\%$ | $+60\%$ | Steel plate armor; deploys field Catapults/Ballistas during sieges to breach walls. |

### 3. General's Masterwork Relic
* Each General can equip or be granted 1 prestigious heirloom (e.g., *Ancestral Blade*, *Aegis Shield*, *Banner of the Founder*).
* Grants unique auras: $+10$ Morale/Loyalty floor, $+15\%$ Siege resistance, or $+20\%$ Combat prowess.
* Relics can be captured as battle trophies if the rival general is defeated or assassinated!

---

## 5. Loyalty, Mutiny & Rebellion (The 7K Threat)

In traditional RTS games, units are robotic pawns. In *Seven Kingdoms*, troops are loyal to **their General**, not the player.

### Loyalty Thresholds
| Loyalty Range | Status | Behavior |
|:---:|:---:|:---|
| **70 – 100** | **Devoted** | Full combat buffs; immune to standard enemy bribes; defends to the death. |
| **40 – 69** | **Content** | Normal behavior; minor wage complaints if treasury runs dry. |
| **20 – 39** | **Disgruntled** | Refuses march commands outside their defensive sector; vulnerable to enemy spy bribes. |
| **0 – 19** | **Treacherous** | **High risk of Mutiny / Defection**. |

### The Mutiny Event Pipeline (`src/game/sim.rs`)
When a General reaches Treacherous status:
1. **Warning Signs**: Urgent event toast appears: *"General [Name] is openly questioning your reign and withholding taxes!"*
2. **Player Intervention**: The player has a small window to intervene (e.g., Send Gold Gift, Bestow Title, or dispatch an Assassin/Inquisitor).
3. **The Mutiny**:
   * If unaddressed, the General revolts!
   * The fort, its entire garrison, and any connected rebellious villages flip to **Rebel Independent** (or defect directly to a rival faction if bribed by an enemy spy).
   * Generates a sudden, intense crisis right in the player's core territory.

---

## 5. Loyalty, Mutiny & Rebellion (The 7K Threat)

In traditional RTS games, units are robotic pawns. In *Seven Kingdoms*, troops are loyal to **their General**, not the player.

### Loyalty Thresholds
| Loyalty Range | Status | Behavior |
|:---:|:---:|:---|
| **70 – 100** | **Devoted** | Full combat buffs; immune to standard enemy bribes; defends to the death. |
| **40 – 69** | **Content** | Normal behavior; minor wage complaints if treasury runs dry. |
| **20 – 39** | **Disgruntled** | Refuses march commands outside their defensive sector; vulnerable to enemy spy bribes. |
| **0 – 19** | **Treacherous** | **High risk of Mutiny / Defection**. |

### The Mutiny Event Pipeline (`src/game/sim.rs`)
When a General reaches Treacherous status:
1. **Warning Signs**: Urgent event toast appears: *"General [Name] is openly questioning your reign and withholding taxes!"*
2. **Player Intervention**: The player has a small window to intervene (e.g., Send Gold Gift, Bestow Title, or dispatch an Assassin/Inquisitor).
3. **The Mutiny**:
   * If unaddressed, the General revolts!
   * The fort, its entire garrison, and any connected rebellious villages flip to **Rebel Independent** (or defect directly to a rival faction if bribed by an enemy spy).
   * Generates a sudden, intense crisis right in the player's core territory.

---

## 6. The Idle / Frictionless Economy Layer

While military and espionage demand focus, the economic layer runs smoothly in the background:

1. **Autonomous Settlement Expansion**:
   * Settlements automatically grow from **Village → Town → City → Metropolis** as long as food and civic order are satisfied.
   * Population growth automatically increases tax revenue and workable labor pools without clicking "upgrade".
2. **Hands-Off Logistics**:
   * Connecting a Mine or Farm to a Settlement creates an automated road ribbon.
   * Caravans dispatch and ferry goods automatically based on destination demand.
3. **Bulk Employment & Worker Training**:
   * Postings at farms, mines, and factories auto-fill from the connected settlement's unemployed pool.
   * Workers gain skill on the job over time ($10 \to 40 \to 80$) passively.
4. **Passive Gold Ticker**:
   * Clear HUD displaying net gold rate: $+ \text{Taxes} - \text{Salaries} - \text{Road Upkeep} = \text{Net Income/tick}$.

---

## 7. Audacious Espionage Overhaul

Transform spies from passive stat-siphons into covert operators with tangible agency:

### 1. Infiltration Roles
Spies do not just sit on a node; they take on an undercover persona:
* **Infiltrate as Soldier**: Enlist in an enemy fort. Gain military rank over time. Can open gates during a siege, poison fort supplies, or assassinate the enemy General.
* **Infiltrate as Agitator**: Planted in an enemy settlement. Siphons treasury, lowers civic order, and incites peasants to petition for your annexation.
* **Infiltrate Court**: High-risk mission in rival capital. Intercepts enemy plans and warns of incoming attacks.

### 2. Bribing Enemy Generals
* Send a spy to meet an enemy general whose loyalty is $< 50$.
* Pay a gold bounty to secretly buy their allegiance:
  * The general stays in the enemy's service as a **sleeper agent**.
  * When war erupts, you trigger the secret pact: the general surrenders the fort and switches sides instantly!

### 3. Counter-Espionage & Mole Hunts
* Rival AI will send spies into your forts and towns.
* Spies stationed defensively inside your own towns act as **Inquisitors**, uncovering enemy agents and triggering execution / counter-play options.

---

## 8. Kingdom Reputation System

Introduce global **Reputation** ($-100$ to $+100$) as a prominent high-stakes metric:

* **Actions that Gain Reputation**:
  * Clearing bandit camps and protecting trade roads ($+5$).
  * Honoring treaties and granting aid to neutral towns ($+10$).
  * Keeping taxes moderate and public order high ($+1$ passive drift).
* **Actions that Destroy Reputation**:
  * Attacking neutral settlements or slaughtering civilians ($-30$).
  * Defaulting on general salaries ($-10$).
  * Caught red-handed conducting dirty espionage operations ($-15$).
* **Reputation Effects**:
  * **High Reputation (> 75)**: Neutral settlements peacefully petition to join your empire for free; Generals are fiercely loyal; enemy spies find it nearly impossible to turn your people.
  * **Dread / Low Reputation (< -30)**: Neutral villages form coalitions against you; your own generals become paranoid and prone to mutiny; assassination attempts spike.

---

## 9. Implementation Roadmap

### Phase 1: Starter Fort & General Spawning (Completed)
* [x] Update `src/game/map.rs` and `world.rs::new()` to generate an owned Fort linked to the capital with a starting General and 5 garrisoned troops.
* [x] Update spawn tests in `tests/spawn.rs` to assert capital fort & general presence across seeds.

### Phase 2: Race Definition & Settlement Demographics (Completed)
* [x] Define `Race` enum (`Human`, `Beetlefolk`, `HiveSwarm`, `SilkSpinner`, `Reptilian`) in `src/game/types.rs`.
* [x] Assign `dominant_race` to `Node` (Settlements & Forts) during map generation and neutral settlement seeding.
* [x] Tie conscripted soldier stats and demographic traits to `dominant_race` (`growth_multiplier`, `food_consumption_multiplier`, `combat_multiplier`, `defense_multiplier`).

### Phase 3: General Struct & Fort Mechanics (Completed)
* [x] Implement `General` struct with `race`, `leadership`, `combat`, `loyalty`, `salary`, `kills` in `src/game/types.rs`.
* [x] Connect fort command to `General` (Leadership order bonus, racial harmony bonus/friction, salary deduction in ledger).
* [x] Add General loyalty tick updates in `src/game/sim.rs` (`general_upkeep_pass`, salary payments, loyalty decay on missed payroll).
* [x] Add `pay_general_bonus` method to bestow 50g bonus and boost loyalty by +15.

### Phase 4: Equipment & Blacksmith Logistics (Completed)
* [x] Implement Fort Equipment Tiers (`Militia` → `Bronze` → `IronClad` → `SteelSiege`) with multipliers and upgrade costs.
* [x] Upgrade Fort equipment logic (`upgrade_fort_equipment`) with gold/ore consumption and combat/defense power buffs.
* [x] Factor equipment tier multipliers into fort defense and siege calculations.

### Phase 5: Mutiny & Defection Pipeline (Completed)
* [x] Implement loyalty decay and mutiny trigger when loyalty drops below 10 in `src/game/sim.rs`.
* [x] Mutiny revolt logic: Fort flips to `Owner::Bandit` with independent commander and garrison.
* [x] Event logging and war alert dispatched to player.

### Phase 6: Espionage Infiltration & Bribes (Planned Next)
* [ ] Replace generic siphon loop with discrete spy assignments (Fort Infiltration, Town Agitation, Counter-Intel).
* [ ] Implement "Bribe General" operation for spies targeting vulnerable enemy commanders.

### Phase 7: UI & Presentation Polish (Completed)
* [x] Add **General Card** in Fort inspection panel (showing Name, Race badge, Combat, Leadership, Kills, Loyalty meter, and "Pay 50g Bonus" button).
* [x] Show Settlement `Race` demographic badges and trait blurbs on inspection cards.
* [x] Show Fort `Equipment Tier` badges, stat multipliers, and "Forge Upgrade" button.
* [x] Add Generals roster to the Agents tab with quick "View Fort" navigation and bonus distribution.
