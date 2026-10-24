//! The world: nodes, edges, spies, faction ledgers, and the queries/commands
//! that everything else (sim, AI, UI) builds on.

use super::map::{agent_name, place_name, Map};
use super::types::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Nodes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: u32,
    pub kind: NodeType,
    pub owner: Owner,
    /// Which great faction owns this node (only meaningful for Player/Enemy).
    pub faction: Option<Faction>,
    pub x: f32,
    pub y: f32,
    pub name: String,

    /// Population (settlements) or garrison (forts, bandit camps).
    pub population: i32,
    /// Local production stock, shipped along roads by caravans.
    pub stock: Stock,
    pub workers: Vec<Worker>,

    // Settlement governance -----------------------------------------------
    /// 0..100, drives growth and tax yield.
    pub happiness: f32,
    /// 0..100, drifts with food, employment, forts.
    pub public_order: f32,
    /// 0..100, resistance to espionage & rebellion. Eroded by Incite.
    pub loyalty: i32,

    /// Empire-wide research banked here comes from Labs; this tracks
    /// per-node research accumulator for Laboratory nodes.
    pub research: f32,

    /// Temporary sabotage timer (ticks remaining, halves production).
    pub sabotage: u32,
    /// While > 0 the node is under siege (visual pulse + happiness loss).
    pub besieged: bool,
    /// 0..100 siege investment. Rises while a hostile fort holds a Siege
    /// road to this settlement; decays when the siege is broken.
    #[serde(default)]
    pub siege_progress: f32,
    /// Accumulator so fractional production isn't lost.
    pub prod_frac: f32,

    /// Tier index for settlements (0 village … 3 metropolis).
    pub tier: u8,

    /// Commanding general (for forts).
    #[serde(default)]
    pub general_id: Option<u32>,
    /// Dominant racial demographics (settlements and forts).
    #[serde(default)]
    pub dominant_race: Option<Race>,
    /// Fort equipment level (militia -> bronze -> iron-clad -> steel).
    #[serde(default)]
    pub equipment: EquipmentTier,
    /// Fealty scores per owner for neutral sites (Player -> score, Enemy -> score).
    #[serde(default)]
    pub fealty: HashMap<Owner, f32>,
    /// Neutral site specialty (Highland Nomads, Iron Guild, etc.).
    #[serde(default)]
    pub specialty: Option<NeutralSpecialty>,
}

impl Node {
    pub fn new(
        id: u32,
        kind: NodeType,
        owner: Owner,
        faction: Option<Faction>,
        x: f32,
        y: f32,
        name: String,
    ) -> Self {
        let (population, happiness, public_order, loyalty, tier) = match kind {
            NodeType::Settlement => (
                if owner == Owner::Neutral {
                    80
                } else {
                    STARTING_POP
                },
                65.0,
                70.0,
                100,
                0,
            ),
            NodeType::Fort => (0, 50.0, 80.0, 100, 0),
            NodeType::BanditCamp => (20, 40.0, 30.0, 100, 0),
            _ => (0, 50.0, 60.0, 100, 0),
        };

        let dominant_race = faction.map(|f| f.native_race());

        Self {
            id,
            kind,
            owner,
            faction,
            x,
            y,
            name,
            population,
            stock: Stock::default(),
            workers: Vec::new(),
            happiness,
            public_order,
            loyalty,
            research: 0.0,
            sabotage: 0,
            besieged: false,
            siege_progress: 0.0,
            prod_frac: 0.0,
            tier,
            general_id: None,
            dominant_race,
            equipment: EquipmentTier::Militia,
            fealty: HashMap::new(),
            specialty: None,
        }
    }

    pub fn fealty_tier(&self, owner: Owner) -> AllegianceTier {
        let f = self.fealty.get(&owner).copied().unwrap_or(0.0);
        if f >= 85.0 {
            AllegianceTier::IntegratedDomain
        } else if f >= 50.0 {
            AllegianceTier::AlliedProtector
        } else if f >= 25.0 {
            AllegianceTier::TradingPartner
        } else {
            AllegianceTier::WaryNeutral
        }
    }

    pub fn is_settlement(&self) -> bool {
        self.kind == NodeType::Settlement
    }

    pub fn is_production(&self) -> bool {
        matches!(
            self.kind,
            NodeType::Farm
                | NodeType::Mine
                | NodeType::Woodcamp
                | NodeType::Factory
                | NodeType::Market
        )
    }

    /// Can trained workers actually be posted here? Settlements "employ
    /// everyone indirectly" and cannot host a crew, so their slots must never
    /// count against a realm's employment (they would be unfillable).
    pub fn can_employ(&self) -> bool {
        self.is_production() || matches!(self.kind, NodeType::Fort | NodeType::Laboratory)
    }

    /// Posts that count toward employment: 0 for a settlement.
    pub fn employable_slots(&self) -> i32 {
        if self.can_employ() {
            self.job_slots()
        } else {
            0
        }
    }

    /// Jobs this building can employ (settlements employ everyone indirectly).
    pub fn job_slots(&self) -> i32 {
        match self.kind {
            NodeType::Settlement => 6 + self.tier as i32 * 4,
            NodeType::Fort => 8,
            NodeType::Farm | NodeType::Mine | NodeType::Woodcamp => 4,
            NodeType::Factory | NodeType::Market => 6,
            NodeType::Laboratory => 4,
            NodeType::BanditCamp => 4,
        }
    }

    pub fn worker_count(&self) -> i32 {
        self.workers.len() as i32
    }

    /// Average skill of ready workers in a given job, if any.
    pub fn avg_skill(&self, job: JobType) -> f32 {
        let list: Vec<&Worker> = self
            .workers
            .iter()
            .filter(|w| w.job == job && w.ready())
            .collect();
        if list.is_empty() {
            0.0
        } else {
            list.iter().map(|w| w.skill as f32).sum::<f32>() / list.len() as f32
        }
    }

    /// Production multiplier from stationed ready workers.
    pub fn worker_boost(&self) -> f32 {
        let ready: Vec<&Worker> = self.workers.iter().filter(|w| w.ready()).collect();
        if ready.is_empty() {
            1.0
        } else {
            1.0 + ready.iter().map(|w| w.skill as f32).sum::<f32>() / 400.0
        }
    }

    /// Current effective production multiplier (sabotage aware).
    pub fn prod_mult(&self) -> f32 {
        let base = self.worker_boost();
        if self.sabotage > 0 {
            base * 0.5
        } else {
            base
        }
    }

    /// Display radius on the map, in world units.
    pub fn radius(&self) -> f32 {
        match self.kind {
            NodeType::Settlement => 1.4 + self.tier as f32 * 0.55,
            NodeType::Fort => 1.5,
            NodeType::BanditCamp => 1.4,
            NodeType::Factory | NodeType::Market => 1.15,
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Faction ledger — empire-wide gold and research
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub gold: i32,
    pub tech: i32,
    pub age: Age,
    /// Gold at the previous tick — powers the King Rating (+1/0/-1).
    pub prev_gold: i32,
    pub income_last: i32,
    pub income_shown: f32,
}

impl Ledger {
    fn new(gold: i32) -> Self {
        Self {
            gold,
            tech: 0,
            age: Age::Wood,
            prev_gold: gold,
            income_last: 0,
            income_shown: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Events log
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    Good,
    Bad,
    War,
    Spy,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub tick: u64,
    pub kind: EventKind,
    pub text: String,
}

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub map: Map,
    pub nodes: HashMap<u32, Node>,
    pub edges: Vec<Edge>,
    pub spies: Vec<Spy>,

    pub player_ledger: Ledger,
    pub enemy_ledger: Ledger,

    pub rng: Rng,
    pub seed: u64,
    pub tick: u64,
    pub next_id: u32,

    pub difficulty: Difficulty,
    pub player_faction: Faction,
    pub enemy_faction: Faction,

    pub win_gold: i32,

    pub victory: Option<Victory>,
    pub log: Vec<LogEvent>,

    /// AI bookkeeping.
    pub ai_timer: u64,
    pub ai_personality: u8, // 0 balanced, 1 aggressive, 2 economic
    /// Ticks until bandits strike again (per-camp handled via rng + timer).
    pub bandit_timer: u64,

    /// Stats for the end screen.
    pub stats: Stats,

    /// Generals commanding forts and leading regiments.
    #[serde(default)]
    pub generals: Vec<General>,
    #[serde(default)]
    pub next_general_id: u32,

    /// Autosave into slot 0 every 600 ticks. Headless tests switch this off so
    /// a 2000-tick economy test cannot clobber a player's real save.
    #[serde(default = "autosave_default")]
    pub autosave: bool,
}

/// `serde` default: saves written before this flag existed keep autosaving.
fn autosave_default() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub battles_won: i32,
    pub spies_born: i32,
    pub spies_caught: i32,
    pub gold_earned: i32,
    pub settlements_taken: i32,
    pub goods_sold: i32,
}

impl World {
    // -----------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------

    pub fn new(seed: u64, player_faction: Faction, difficulty: Difficulty) -> Self {
        let map = Map::generate(seed, 120, 90);
        let mut world = Self {
            map,
            nodes: HashMap::new(),
            edges: Vec::new(),
            spies: Vec::new(),
            player_ledger: Ledger::new(STARTING_GOLD),
            enemy_ledger: Ledger::new(STARTING_GOLD),
            rng: Rng::new(seed ^ 0x1234_5678),
            seed,
            tick: 0,
            next_id: 0,
            difficulty,
            player_faction,
            enemy_faction: random_enemy_faction(player_faction, seed),
            win_gold: WIN_GOLD,
            victory: None,
            log: Vec::new(),
            ai_timer: difficulty.ai_interval(),
            ai_personality: 0,
            bandit_timer: 250,
            stats: Stats::default(),
            generals: Vec::new(),
            next_general_id: 0,
            autosave: true,
        };

        world.place_world();
        world
    }

    // -----------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------

    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: u32) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn player(&self) -> &Ledger {
        &self.player_ledger
    }

    pub fn enemy(&self) -> &Ledger {
        &self.enemy_ledger
    }

    pub fn ledger_mut(&mut self, owner: Owner) -> &mut Ledger {
        match owner {
            Owner::Enemy => &mut self.enemy_ledger,
            _ => &mut self.player_ledger,
        }
    }

    pub fn nodes_owned_by(&self, owner: Owner) -> Vec<&Node> {
        self.nodes.values().filter(|n| n.owner == owner).collect()
    }

    pub fn settlements_of(&self, owner: Owner) -> Vec<&Node> {
        self.nodes
            .values()
            .filter(|n| n.owner == owner && n.is_settlement())
            .collect()
    }

    pub fn edges_of(&self, node_id: u32) -> impl Iterator<Item = &Edge> {
        self.edges
            .iter()
            .filter(move |e| e.from == node_id || e.to == node_id)
    }

    /// (filled posts, workable posts) across everything `owner` holds.
    ///
    /// Settlements contribute nothing to the denominator — they cannot host a
    /// crew, so counting their slots would cap employment below 100% forever.
    pub fn employment_of(&self, owner: Owner) -> (i32, i32) {
        let mut filled = 0;
        let mut posts = 0;
        for n in self.nodes.values().filter(|n| n.owner == owner) {
            let slots = n.employable_slots();
            if slots > 0 {
                posts += slots;
                filled += n.worker_count().min(slots);
            }
        }
        (filled, posts)
    }

    /// True when this settlement has a same-owner Supply road to a fort.
    pub fn has_fort_supply(&self, id: u32) -> bool {
        self.fort_supplying(id).is_some()
    }

    /// Returns the fort ID supplying this settlement, if any.
    pub fn fort_supplying(&self, id: u32) -> Option<u32> {
        let owner = {
            let n = self.nodes.get(&id)?;
            n.owner
        };
        self.edges_of(id).find_map(|e| {
            if e.kind == EdgeType::Supply {
                let other_id = e.other(id);
                if self
                    .nodes
                    .get(&other_id)
                    .map(|f| f.kind == NodeType::Fort && f.owner == owner)
                    .unwrap_or(false)
                {
                    Some(other_id)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// The friendly settlement feeding this fort by Supply road, preferring
    /// the one with the largest goods stock so armory upgrades draw on the
    /// best available store.
    pub fn fort_supply_source(&self, fort_id: u32, owner: Owner) -> Option<u32> {
        self.edges
            .iter()
            .filter(|e| e.kind == EdgeType::Supply)
            .filter_map(|e| {
                let other = if e.from == fort_id {
                    e.to
                } else if e.to == fort_id {
                    e.from
                } else {
                    return None;
                };
                self.nodes
                    .get(&other)
                    .filter(|n| n.owner == owner && n.is_settlement())
                    .map(|n| n.id)
            })
            .max_by_key(|id| self.nodes.get(id).map(|n| n.stock.goods).unwrap_or(0))
    }

    /// Is this fort connected to any friendly node by a Supply road? This is
    /// the military lifeline: unsupplied garrisons fight and besiege worse.
    pub fn fort_is_supplied(&self, fort_id: u32) -> bool {
        let Some(fort) = self.nodes.get(&fort_id) else {
            return false;
        };
        self.edges_of(fort_id).any(|e| {
            e.kind == EdgeType::Supply
                && self
                    .nodes
                    .get(&e.other(fort_id))
                    .map(|n| n.owner == fort.owner)
                    .unwrap_or(false)
        })
    }

    pub fn general(&self, id: u32) -> Option<&General> {
        self.generals.iter().find(|g| g.id == id)
    }

    pub fn general_mut(&mut self, id: u32) -> Option<&mut General> {
        self.generals.iter_mut().find(|g| g.id == id)
    }

    pub fn generals_of(&self, owner: Owner) -> Vec<&General> {
        self.generals.iter().filter(|g| g.owner == owner).collect()
    }

    /// Is food actually being carried to this settlement — a Resource road to
    /// a farm it may draw from (its own or a neutral one it has linked)?
    pub fn fed_by_link(&self, id: u32) -> bool {
        let owner = match self.nodes.get(&id) {
            Some(n) => n.owner,
            None => return false,
        };
        self.edges_of(id).any(|e| {
            e.kind == EdgeType::Resource
                && self
                    .nodes
                    .get(&e.other(id))
                    .map(|n| {
                        n.kind == NodeType::Farm && (n.owner == owner || n.owner == Owner::Neutral)
                    })
                    .unwrap_or(false)
        })
    }

    /// Garrison ceiling for a fort: what the villages feeding it can support
    /// (0 when nothing feeds it).
    pub fn garrison_cap_at(&self, fort_id: u32) -> i32 {
        let fort = match self.nodes.get(&fort_id) {
            Some(f) => f,
            None => return 0,
        };
        let mut cap = 0;
        for e in self.edges_of(fort_id) {
            if e.kind != EdgeType::Supply {
                continue;
            }
            if let Some(s) = self.nodes.get(&e.other(fort_id)) {
                if s.is_settlement() && s.owner == fort.owner {
                    cap = cap.max(garrison_cap(s.tier, s.population + fort.population));
                }
            }
        }
        cap
    }

    pub fn edge_between(&self, a: u32, b: u32) -> Option<usize> {
        self.edges
            .iter()
            .position(|e| (e.from == a && e.to == b) || (e.from == b && e.to == a))
    }

    /// Nodes reachable from `start` by following edges allowed for `viewer`.
    ///
    /// A node is *linked* to a faction when there is a path of that faction's
    /// own roads (Resource/Supply/Trade) to one of its settlements. Neutral
    /// producers only work when a faction links them — the 7K "claim by
    /// connection" idea.
    pub fn linked_set(&self, viewer: Owner) -> HashSet<u32> {
        let mut seen = HashSet::new();
        let mut queue: Vec<u32> = self
            .nodes
            .values()
            .filter(|n| n.owner == viewer && n.is_settlement())
            .map(|n| n.id)
            .collect();

        // Factions with no settlements still "own" their forts/production.
        if queue.is_empty() {
            queue = self
                .nodes
                .values()
                .filter(|n| n.owner == viewer)
                .map(|n| n.id)
                .collect();
        }

        while let Some(id) = queue.pop() {
            if !seen.insert(id) {
                continue;
            }
            for e in self.edges_of(id) {
                if matches!(e.kind, EdgeType::Combat | EdgeType::Siege) {
                    continue;
                }
                // Traversal only through edges the viewer built or owns the
                // far end of (you can walk into neutrals you trade with).
                let other = e.other(id);
                let other_owner = self.nodes.get(&other).map(|n| n.owner);
                if e.by != viewer && other_owner != Some(viewer) {
                    continue;
                }
                if !seen.contains(&other) {
                    queue.push(other);
                }
            }
        }
        seen
    }

    pub fn is_linked(&self, node_id: u32, viewer: Owner) -> bool {
        self.linked_set(viewer).contains(&node_id)
    }

    /// Total idle population across a faction's settlements.
    pub fn total_pop(&self, owner: Owner) -> i32 {
        self.settlements_of(owner)
            .into_iter()
            .map(|n| n.population)
            .sum()
    }

    /// Aggregate local stock across a faction's nodes.
    pub fn total_stock(&self, owner: Owner) -> Stock {
        let mut s = Stock::default();
        for n in self.nodes.values().filter(|n| n.owner == owner) {
            s.food += n.stock.food;
            s.ore += n.stock.ore;
            s.timber += n.stock.timber;
            s.goods += n.stock.goods;
        }
        s
    }

    /// Influence of `viewer` over a neutral/enemy settlement, 0..100.
    /// Drives petitions, fomentation, and the Web victory.
    pub fn influence(&self, viewer: Owner, target_id: u32) -> f32 {
        let Some(target) = self.nodes.get(&target_id) else {
            return 0.0;
        };
        if target.owner == viewer || !target.is_settlement() {
            return 0.0;
        }

        let ledger = match viewer {
            Owner::Enemy => &self.enemy_ledger,
            _ => &self.player_ledger,
        };

        // King Rating: +1 gold increasing, -1 decreasing.
        let king = (ledger.gold - ledger.prev_gold).signum() as f32;

        // Food surplus across the viewer's settlements.
        let food = self
            .settlements_of(viewer)
            .into_iter()
            .map(|n| n.stock.food)
            .sum::<i32>() as f32;

        let happiness = {
            let settlements = self.settlements_of(viewer);
            if settlements.is_empty() {
                50.0
            } else {
                settlements.iter().map(|n| n.happiness).sum::<f32>() / settlements.len() as f32
            }
        };

        // Distance decay from nearest viewer settlement.
        let mut min_dist = f32::MAX;
        for s in self.settlements_of(viewer) {
            let d = Map::dist(s.x, s.y, target.x, target.y);
            min_dist = min_dist.min(d);
        }
        if min_dist == f32::MAX {
            min_dist = 60.0;
        }

        // Trade routes and infiltrating spies add direct pressure.
        let trade_bonus = self
            .edges_of(target_id)
            .filter(|e| e.by == viewer && e.kind == EdgeType::Trade)
            .count() as f32
            * 8.0;

        let spy_bonus =
            if self.spies.iter().any(|s| {
                s.owner == viewer && s.target == target_id && s.status == SpyStatus::Embedded
            }) {
                25.0
            } else {
                0.0
            };

        let raw =
            (king * 8.0) + (food * 0.06) + ((happiness - 50.0) * 0.35) + trade_bonus + spy_bonus
                - (min_dist * 0.9)
                + 22.0;

        raw.clamp(0.0, 100.0)
    }

    /// Which enemy settlements are "bound" for the Web victory
    /// (embedded spy with influence ≥ 70).
    pub fn web_progress(&self) -> (usize, usize) {
        let enemy_settlements: Vec<u32> = self
            .settlements_of(Owner::Enemy)
            .into_iter()
            .map(|n| n.id)
            .collect();
        let total = enemy_settlements.len();
        let bound = enemy_settlements
            .iter()
            .filter(|id| {
                self.spies.iter().any(|s| {
                    s.owner == Owner::Player
                        && s.target == **id
                        && s.status == SpyStatus::Embedded
                        && s.op == SpyOp::Infiltrate
                }) && self.influence(Owner::Player, **id) >= 70.0
            })
            .count();
        (bound, total)
    }

    // -----------------------------------------------------------------
    // Logging
    // -----------------------------------------------------------------

    pub fn log(&mut self, kind: EventKind, text: impl Into<String>) {
        self.log.push(LogEvent {
            tick: self.tick,
            kind,
            text: text.into(),
        });
        // Keep the log bounded.
        if self.log.len() > 400 {
            let excess = self.log.len() - 400;
            self.log.drain(0..excess);
        }
    }

    // -----------------------------------------------------------------
    // Road building
    // -----------------------------------------------------------------

    /// Cost (in gold) of a road between two nodes.
    pub fn road_cost(&self, from_id: u32, to_id: u32) -> Option<i32> {
        let a = self.nodes.get(&from_id)?;
        let b = self.nodes.get(&to_id)?;
        let straight = Map::dist(a.x, a.y, b.x, b.y);
        if straight > MAX_EDGE_TILES || straight < MIN_EDGE_TILES {
            return None;
        }
        let path = self
            .map
            .find_road((a.x as i32, a.y as i32), (b.x as i32, b.y as i32))?;
        Some(EDGE_BASE_COST + path.len() as i32)
    }

    /// Classify what kind of road this is (Supply vs Combat vs Resource…).
    pub fn classify_edge(&self, a: u32, b: u32) -> Option<EdgeType> {
        let na = self.nodes.get(&a)?;
        let nb = self.nodes.get(&b)?;

        // Fort ↔ hostile fort or bandit camp: battle.
        if na.kind == NodeType::Fort
            && nb.kind == NodeType::Fort
            && na.owner.is_hostile_to(nb.owner)
        {
            return Some(EdgeType::Combat);
        }
        if (na.kind == NodeType::Fort && nb.kind == NodeType::BanditCamp)
            || (nb.kind == NodeType::Fort && na.kind == NodeType::BanditCamp)
        {
            return Some(EdgeType::Combat);
        }
        // Fort → hostile settlement: siege.
        if na.kind == NodeType::Fort && nb.is_settlement() && na.owner.is_hostile_to(nb.owner) {
            return Some(EdgeType::Siege);
        }
        if nb.kind == NodeType::Fort && na.is_settlement() && na.owner.is_hostile_to(nb.owner) {
            return Some(EdgeType::Siege);
        }
        // Same-owner settlement ↔ fort: supply.
        if na.owner == nb.owner
            && na.owner != Owner::Neutral
            && ((na.kind == NodeType::Fort && nb.is_settlement())
                || (nb.kind == NodeType::Fort && na.is_settlement()))
        {
            return Some(EdgeType::Supply);
        }
        // Anything touching a hostile economy node: trade (espionage cover).
        if na.owner.is_hostile_to(nb.owner)
            && !matches!(na.kind, NodeType::Fort)
            && !matches!(nb.kind, NodeType::Fort)
        {
            return Some(EdgeType::Trade);
        }
        // Otherwise: resource road.
        Some(EdgeType::Resource)
    }

    /// Build a road. Returns the new edge index or an error message.
    pub fn build_road(&mut self, from_id: u32, to_id: u32, payer: Owner) -> Result<usize, String> {
        if from_id == to_id {
            return Err("Cannot connect a node to itself".into());
        }
        if self.edge_between(from_id, to_id).is_some() {
            return Err("These nodes are already connected".into());
        }

        let cost = self
            .road_cost(from_id, to_id)
            .ok_or("No viable route (too far, blocked terrain, or off-map)")?;

        let kind = self
            .classify_edge(from_id, to_id)
            .ok_or("These nodes cannot be connected")?;

        let ledger_gold = match payer {
            Owner::Enemy => self.enemy_ledger.gold,
            _ => self.player_ledger.gold,
        };
        if ledger_gold < cost {
            return Err(format!("Need {} gold for that road", cost));
        }

        let a = self.nodes[&from_id].clone();
        let b = self.nodes[&to_id].clone();
        let path = self
            .map
            .find_road((a.x as i32, a.y as i32), (b.x as i32, b.y as i32))
            .ok_or("No viable route through this terrain")?;

        match payer {
            Owner::Enemy => self.enemy_ledger.gold -= cost,
            _ => self.player_ledger.gold -= cost,
        }

        let caravans = if kind == EdgeType::Resource { 1 } else { 0 };
        let id = self.next_id;
        self.next_id += 1;

        self.edges.push(Edge {
            id,
            from: from_id,
            to: to_id,
            kind,
            by: payer,
            caravans,
            progress: 0.0,
            path,
        });

        Ok(self.edges.len() - 1)
    }

    /// Cut a road (player action, refunds nothing).
    pub fn cut_road(&mut self, edge_index: usize, payer: Owner) -> Result<(), String> {
        let e = self.edges.get(edge_index).ok_or("No such road")?;
        if e.by != payer {
            return Err("That road belongs to your enemy".into());
        }
        self.edges.remove(edge_index);
        Ok(())
    }

    // -----------------------------------------------------------------
    // Building placement
    // -----------------------------------------------------------------

    /// Read-only placement validation (used by `place_building` and by the
    /// engine's build-ghost preview).
    pub fn can_place(&self, kind: NodeType, x: f32, y: f32, payer: Owner) -> Result<(), String> {
        if !kind.is_buildable() {
            return Err("That cannot be built".into());
        }

        let tx = x as i32;
        let ty = y as i32;

        if !self.map.in_bounds(tx, ty) {
            return Err("Outside the map".into());
        }
        let terrain = self.map.terrain_at(tx, ty);
        if !terrain.buildable() {
            return Err("Cannot build on water or mountains".into());
        }

        // Terrain requirements per building type.
        match kind {
            NodeType::Mine => {
                if self.map.deposit_at(tx, ty) == Deposit::None {
                    return Err("Mines must sit on an ore deposit (orange/silver specks)".into());
                }
            }
            NodeType::Woodcamp => {
                if terrain != Terrain::Forest {
                    return Err("Woodcamps must be built in forest".into());
                }
            }
            _ => {
                if terrain == Terrain::Forest
                    && matches!(
                        kind,
                        NodeType::Settlement | NodeType::Market | NodeType::Laboratory
                    )
                {
                    return Err("Clear ground needed (not forest)".into());
                }
            }
        }

        // Minimum spacing from other nodes.
        for n in self.nodes.values() {
            let d = Map::dist(n.x, n.y, x, y);
            let min = if kind == NodeType::Settlement || n.is_settlement() {
                4.0
            } else {
                2.5
            };
            if d < min {
                return Err(format!("Too close to {}", n.name));
            }
        }

        // Settlements need open country.
        if kind == NodeType::Settlement {
            let mut land = 0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if self.map.passable(tx + dx, ty + dy) {
                        land += 1;
                    }
                }
            }
            if land < 16 {
                return Err("Settlements need open land around them".into());
            }
        }

        // Cost.
        let cost = build_cost(kind);
        let ledger_gold = match payer {
            Owner::Enemy => self.enemy_ledger.gold,
            _ => self.player_ledger.gold,
        };
        if ledger_gold < cost {
            return Err(format!("Need {} gold to build a {}", cost, kind.label()));
        }

        // New buildings must be within reach of the payer's network
        // (or, for a first settlement, anywhere sensible).
        if kind != NodeType::Settlement {
            let linked = self.linked_set(payer);
            let reachable = linked.iter().any(|id| {
                self.nodes
                    .get(id)
                    .map(|n| Map::dist(n.x, n.y, x, y) <= MAX_EDGE_TILES)
                    .unwrap_or(false)
            });
            if !reachable && !linked.is_empty() {
                return Err("Too far from your network (build within road range)".into());
            }
        }
        Ok(())
    }

    /// Try to place a building at world position (x, y).
    pub fn place_building(
        &mut self,
        kind: NodeType,
        x: f32,
        y: f32,
        payer: Owner,
    ) -> Result<u32, String> {
        self.can_place(kind, x, y, payer)?;

        let cost = build_cost(kind);
        match payer {
            Owner::Enemy => self.enemy_ledger.gold -= cost,
            _ => self.player_ledger.gold -= cost,
        }

        let id = self.next_id;
        self.next_id += 1;

        let name = match kind {
            NodeType::Settlement => place_name(self.rng.next_u64()),
            NodeType::BanditCamp => "Bandit Camp".into(),
            _ => format!("{} {}", kind.short(), id),
        };

        let faction = match payer {
            Owner::Player => Some(self.player_faction),
            Owner::Enemy => Some(self.enemy_faction),
            _ => None,
        };

        let mut node = Node::new(id, kind, payer, faction, x, y, name);
        if kind == NodeType::Settlement {
            node.population = 10;
            node.stock.food = 20;
        }
        self.nodes.insert(id, node);
        Ok(id)
    }

    /// Demolish a player building (half refund). Settlements are protected
    /// if it's the last one.
    pub fn demolish(&mut self, node_id: u32, payer: Owner) -> Result<i32, String> {
        let node = self.nodes.get(&node_id).ok_or("No such node")?;
        if node.owner != payer {
            return Err("Not your building".into());
        }
        if node.kind == NodeType::Settlement && self.settlements_of(payer).len() <= 1 {
            return Err("Cannot abandon your last settlement".into());
        }
        if node.kind == NodeType::BanditCamp {
            return Err("Bandits do not negotiate".into());
        }

        let refund = build_cost(node.kind) / 2;

        // A fort being demolished releases its garrison back home if possible.
        self.nodes.remove(&node_id);
        self.edges.retain(|e| e.from != node_id && e.to != node_id);
        self.spies
            .retain(|s| s.home != node_id && s.target != node_id);

        match payer {
            Owner::Enemy => self.enemy_ledger.gold += refund,
            _ => self.player_ledger.gold += refund,
        }
        Ok(refund)
    }

    // -----------------------------------------------------------------
    // Workers
    // -----------------------------------------------------------------

    /// Train a worker at a production building.
    pub fn train_worker(&mut self, node_id: u32, tier: usize, payer: Owner) -> Result<(), String> {
        let cost = TRAIN_COSTS[tier];
        let skill = TRAIN_SKILLS[tier];

        // Validate the destination before taking any gold, so a refused command
        // never costs the player.
        let job = {
            let node = self.nodes.get(&node_id).ok_or("No such node")?;
            if node.owner != payer {
                return Err("Not your building".into());
            }
            if !node.is_production()
                && node.kind != NodeType::Fort
                && node.kind != NodeType::Laboratory
            {
                return Err("Train workers at production buildings, forts, or labs".into());
            }
            if node.worker_count() >= node.job_slots() {
                return Err("This building is fully staffed".into());
            }
            node.kind.job()
        };

        let ledger_ok = match payer {
            Owner::Enemy => self.enemy_ledger.gold >= cost,
            _ => self.player_ledger.gold >= cost,
        };
        if !ledger_ok {
            return Err(format!("Need {} gold", cost));
        }

        // Training requires an idle soul from one of your settlements.
        let mut source = None;
        for n in self.nodes.values_mut() {
            if n.owner == payer && n.is_settlement() && n.population > 1 {
                source = Some(n.id);
                break;
            }
        }
        let source = source.ok_or("No settlement with population to send")?;

        match payer {
            Owner::Enemy => self.enemy_ledger.gold -= cost,
            _ => self.player_ledger.gold -= cost,
        }

        let train_ticks = (skill as u32) * 2;
        let name = agent_name(self.rng.next_u64());

        let node = self.nodes.get_mut(&node_id).ok_or("No such node")?;
        node.workers
            .push(Worker::new(job, skill, train_ticks, name));

        // The worker is drawn from settlement population.
        if let Some(s) = self.nodes.get_mut(&source) {
            s.population -= 1;
        }

        Ok(())
    }

    /// Dismiss a worker (half the training cost refunded).
    pub fn dismiss_worker(
        &mut self,
        node_id: u32,
        index: usize,
        payer: Owner,
    ) -> Result<i32, String> {
        let removed = {
            let node = self.nodes.get_mut(&node_id).ok_or("No such node")?;
            if node.owner != payer {
                return Err("Not your building".into());
            }
            if index >= node.workers.len() {
                return Err("No such worker".into());
            }
            node.workers.remove(index)
        };
        let refund = 5;
        let _ = removed;

        // Send them home if there's a settlement to receive them.
        let home = self
            .nodes
            .values()
            .filter(|n| n.owner == payer && n.is_settlement())
            .map(|n| n.id)
            .next();
        if let Some(h) = home {
            if let Some(n) = self.nodes.get_mut(&h) {
                n.population += 1;
            }
        }
        Ok(refund)
    }

    // -----------------------------------------------------------------
    // Espionage
    // -----------------------------------------------------------------

    pub fn train_spy(&mut self, settlement_id: u32, payer: Owner) -> Result<u32, String> {
        let ledger_ok = match payer {
            Owner::Enemy => self.enemy_ledger.gold >= SPY_TRAIN_COST,
            _ => self.player_ledger.gold >= SPY_TRAIN_COST,
        };
        if !ledger_ok {
            return Err(format!("Need {} gold", SPY_TRAIN_COST));
        }

        let mut source = None;
        for n in self.nodes.values_mut() {
            if n.owner == payer && n.is_settlement() && n.population > 3 {
                source = Some(n.id);
                break;
            }
        }
        let source = source.ok_or("Spies need a settlement with 4+ population")?;

        match payer {
            Owner::Enemy => self.enemy_ledger.gold -= SPY_TRAIN_COST,
            _ => self.player_ledger.gold -= SPY_TRAIN_COST,
        }

        let id = self.next_id;
        self.next_id += 1;

        let faction_bonus = match payer {
            Owner::Enemy => faction_def(self.enemy_faction).spy,
            _ => faction_def(self.player_faction).spy,
        };
        let age_bonus = match payer {
            Owner::Enemy => self.enemy_ledger.age.spy_bonus(),
            _ => self.player_ledger.age.spy_bonus(),
        };
        let skill = ((25.0 * faction_bonus) as i32 + age_bonus).clamp(10, 100);

        let spy = Spy {
            id,
            owner: payer,
            name: agent_name(self.rng.next_u64()),
            home: source,
            target: settlement_id,
            status: SpyStatus::Traveling,
            op: SpyOp::Infiltrate,
            skill,
            progress: 0.0,
            embed_ticks: 0,
            siphoned: 0,
        };

        if let Some(s) = self.nodes.get_mut(&source) {
            s.population -= 1;
        }
        if payer == Owner::Player {
            self.stats.spies_born += 1;
        }
        self.spies.push(spy);
        Ok(id)
    }

    pub fn set_spy_op(&mut self, spy_id: u32, op: SpyOp, payer: Owner) -> Result<(), String> {
        let spy = self
            .spies
            .iter_mut()
            .find(|s| s.id == spy_id)
            .ok_or("No such agent")?;
        if spy.owner != payer {
            return Err("Not your agent".into());
        }
        spy.op = op;
        spy.embed_ticks = 0;
        Ok(())
    }

    pub fn recall_spy(&mut self, spy_id: u32, payer: Owner) -> Result<(), String> {
        let spy = self
            .spies
            .iter_mut()
            .find(|s| s.id == spy_id)
            .ok_or("No such agent")?;
        if spy.owner != payer {
            return Err("Not your agent".into());
        }
        if spy.status != SpyStatus::Returning {
            spy.status = SpyStatus::Returning;
            spy.progress = 0.0;
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // Influence actions (peaceful annexation of neutrals)
    // -----------------------------------------------------------------

    /// Petition: a neutral joins you when influence is high (called from sim).
    /// Manual actions:
    pub fn foment(&mut self, target_id: u32, payer: Owner) -> Result<(), String> {
        const COST: i32 = 100;
        let inf = self.influence(payer, target_id);
        if inf < 61.0 {
            return Err("Influence too low (need 61+)".into());
        }
        let ledger_ok = match payer {
            Owner::Enemy => self.enemy_ledger.gold >= COST,
            _ => self.player_ledger.gold >= COST,
        };
        if !ledger_ok {
            return Err(format!("Need {} gold", COST));
        }
        match payer {
            Owner::Enemy => self.enemy_ledger.gold -= COST,
            _ => self.player_ledger.gold -= COST,
        }
        if let Some(n) = self.nodes.get_mut(&target_id) {
            n.owner = payer;
            n.loyalty = 45;
            n.happiness = (n.happiness - 15.0).max(10.0);
        }
        Ok(())
    }

    pub fn buy_settlement(&mut self, target_id: u32, payer: Owner) -> Result<(), String> {
        const COST: i32 = 200;
        let inf = self.influence(payer, target_id);
        if inf < 81.0 {
            return Err("Influence too low (need 81+)".into());
        }
        let ledger_ok = match payer {
            Owner::Enemy => self.enemy_ledger.gold >= COST,
            _ => self.player_ledger.gold >= COST,
        };
        if !ledger_ok {
            return Err(format!("Need {} gold", COST));
        }
        match payer {
            Owner::Enemy => self.enemy_ledger.gold -= COST,
            _ => self.player_ledger.gold -= COST,
        }
        if let Some(n) = self.nodes.get_mut(&target_id) {
            n.owner = payer;
            n.loyalty = 70;
            n.happiness = (n.happiness + 10.0).min(100.0);
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // Research
    // -----------------------------------------------------------------

    pub fn advance_age(&mut self, payer: Owner) -> Result<(), String> {
        let ledger = match payer {
            Owner::Enemy => &mut self.enemy_ledger,
            _ => &mut self.player_ledger,
        };
        let cost = ledger
            .age
            .cost_to_advance()
            .ok_or("Already in the Steel Age")?;
        if ledger.tech < cost {
            return Err(format!(
                "Need {} research points (have {})",
                cost, ledger.tech
            ));
        }
        ledger.tech -= cost;
        ledger.age = Age::from_index(ledger.age.index() + 1);
        Ok(())
    }

    /// Pay a 50g bonus to a general to boost their loyalty by +15.
    pub fn pay_general_bonus(&mut self, gen_id: u32, payer: Owner) -> Result<(), String> {
        let cost = 50;
        let has_gold = match payer {
            Owner::Enemy => self.enemy_ledger.gold >= cost,
            _ => self.player_ledger.gold >= cost,
        };
        if !has_gold {
            let gold = match payer {
                Owner::Enemy => self.enemy_ledger.gold,
                _ => self.player_ledger.gold,
            };
            return Err(format!(
                "Need {} gold to bestow bonus (have {})",
                cost, gold
            ));
        }

        let (name, new_loyalty) = {
            let general = self
                .generals
                .iter_mut()
                .find(|g| g.id == gen_id)
                .ok_or("General not found")?;
            general.loyalty = (general.loyalty + 15).min(100);
            (general.name.clone(), general.loyalty)
        };

        match payer {
            Owner::Enemy => self.enemy_ledger.gold -= cost,
            _ => self.player_ledger.gold -= cost,
        };

        if payer == Owner::Player {
            self.log(
                EventKind::Good,
                format!(
                    "Bestowed 50g bonus to {}: loyalty is now {}%!",
                    name, new_loyalty
                ),
            );
        }
        Ok(())
    }

    /// Upgrade equipment at a fort using realm gold.
    pub fn upgrade_fort_equipment(&mut self, fort_id: u32, payer: Owner) -> Result<(), String> {
        let (current_tier, name) = {
            let fort = self.nodes.get(&fort_id).ok_or("Fort not found")?;
            if fort.kind != NodeType::Fort {
                return Err("Node is not a fort".into());
            }
            if fort.owner != payer {
                return Err("Cannot upgrade foreign fort equipment".into());
            }
            (fort.equipment, fort.name.clone())
        };

        let next_tier = current_tier
            .next()
            .ok_or("Already at maximum equipment tier")?;
        let gold_cost = current_tier.upgrade_gold();
        let goods_cost = current_tier.upgrade_goods();
        let gold_have = match payer {
            Owner::Enemy => self.enemy_ledger.gold,
            _ => self.player_ledger.gold,
        };

        let source_id = self
            .fort_supply_source(fort_id, payer)
            .ok_or("Fort needs a Supply road to a friendly settlement")?;
        let goods_have = self
            .nodes
            .get(&source_id)
            .map(|s| s.stock.goods)
            .unwrap_or(0);
        let source_name = self
            .nodes
            .get(&source_id)
            .map(|s| s.name.clone())
            .unwrap_or_default();

        if gold_have < gold_cost {
            return Err(format!(
                "Need {} gold for {} weapons (have {})",
                gold_cost,
                next_tier.label(),
                gold_have
            ));
        }
        if goods_have < goods_cost {
            return Err(format!(
                "Need {} goods at {} for {} weapons (have {})",
                goods_cost,
                source_name,
                next_tier.label(),
                goods_have
            ));
        }

        // Both costs are validated before either is taken.
        match payer {
            Owner::Enemy => self.enemy_ledger.gold -= gold_cost,
            _ => self.player_ledger.gold -= gold_cost,
        }
        if let Some(source) = self.nodes.get_mut(&source_id) {
            source.stock.goods -= goods_cost;
        }
        if let Some(fort) = self.nodes.get_mut(&fort_id) {
            fort.equipment = next_tier;
        }

        if payer == Owner::Player {
            self.log(
                EventKind::Good,
                format!(
                    "{} upgraded its armory to {} ({}g + {} goods from {})!",
                    name,
                    next_tier.label(),
                    gold_cost,
                    goods_cost,
                    source_name
                ),
            );
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // World generation: capitals, neutrals, resources, bandits
    // -----------------------------------------------------------------

    fn place_world(&mut self) {
        let cols = self.map.cols;
        let rows = self.map.rows;

        // Player capital: north-west quadrant. Enemy: south-east.
        let pw = self.map.nearest_passable(cols / 7, rows / 7, 3);
        let ew = self
            .map
            .nearest_passable(cols - cols / 7, rows - rows / 7, 3);

        let pid = self.spawn_settlement(Owner::Player, pw.0, pw.1);
        let eid = self.spawn_settlement(Owner::Enemy, ew.0, ew.1);

        // Neutral settlements scattered through the middle.
        let neutral_count = 16;
        let mut placed: Vec<(f32, f32)> =
            vec![(pw.0 as f32, pw.1 as f32), (ew.0 as f32, ew.1 as f32)];
        let mut villages: Vec<u32> = Vec::new();
        for _ in 0..neutral_count {
            if let Some((x, y)) = self.find_spot(&placed, 12.0, true) {
                let id = self.spawn_settlement(Owner::Neutral, x as i32, y as i32);
                villages.push(id);
                placed.push((x, y));
            }
        }

        // Resource nodes: farms, woodcamps, ore deposits — each must be
        // reachable by both sides for fairness.
        let mut all: Vec<(f32, f32)> = placed.clone();
        let resources = [
            (NodeType::Farm, 24),
            (NodeType::Woodcamp, 18),
            (NodeType::Mine, 16),
        ];
        for (kind, count) in resources {
            for _ in 0..count {
                if let Some((x, y)) = self.find_resource_spot(kind, &all, 6.0) {
                    self.spawn_resource(kind, x, y);
                    all.push((x, y));
                }
            }
        }

        // Bandit camps — neutral hazards that raid over time.
        for _ in 0..6 {
            if let Some((x, y)) = self.find_spot(&all, 15.0, false) {
                let id = self.next_id;
                self.next_id += 1;
                let mut n = Node::new(
                    id,
                    NodeType::BanditCamp,
                    Owner::Bandit,
                    None,
                    x,
                    y,
                    format!("Bandit Camp {}", id),
                );
                n.population = 20 + (self.rng.next_u64() % 20) as i32;
                n.stock.food = 30;
                self.nodes.insert(id, n);
                all.push((x, y));
            }
        }

        // Every settlement gets its own starter ring, capitals and independent
        // villages alike. A village with no food of its own starves to zero
        // within a minute of play, which is how the map filled with dead cities.
        let mut to_bootstrap = vec![pid, eid];
        to_bootstrap.extend(villages.iter().copied());
        for id in to_bootstrap {
            self.bootstrap_settlement(id);
        }
        self.starting_road(pid);
        self.starting_road(eid);
        self.spawn_starter_fort(pid);
        self.spawn_starter_fort(eid);
    }

    /// Guarantee a settlement has local food/timber/ore to work with: place one
    /// of each starter building in a ring around it, road them up for free, and
    /// pre-crew each with a novice. The ring belongs to the settlement's owner,
    /// so a new lord can actually staff it.
    ///
    /// Food is *guaranteed*: if no ring spot suits a farm, one is carved out at
    /// the nearest buildable tile. A settlement without a food link starves to
    /// zero population and used to be an unrecoverable start.
    fn bootstrap_settlement(&mut self, capital: u32) {
        let Some(cap) = self.nodes.get(&capital) else {
            return;
        };
        let (cx, cy, owner) = (cap.x, cap.y, cap.owner);

        let starters = [
            (NodeType::Farm, Terrain::Grass, 5.0, 9.0),
            (NodeType::Woodcamp, Terrain::Forest, 4.0, 11.0),
            (NodeType::Mine, Terrain::Grass, 6.0, 13.0),
        ];

        let mut placed_farm = false;
        for (kind, want, rmin, rmax) in starters {
            let mut placed: Option<(f32, f32)> = None;
            for _ in 0..120 {
                let ang = self.rng.range_f(0.0, std::f32::consts::TAU);
                let r = self.rng.range_f(rmin, rmax);
                let x = cx + ang.cos() * r;
                let y = cy + ang.sin() * r;
                let tx = x as i32;
                let ty = y as i32;
                if !self.map.in_bounds(tx, ty) {
                    continue;
                }
                // Mines must land on *buildable* ore (deposits on mountains
                // are impassable and would leave the building stranded).
                let ok = match kind {
                    NodeType::Mine => {
                        self.map.deposit_at(tx, ty) != Deposit::None
                            && self.map.terrain_at(tx, ty).buildable()
                    }
                    NodeType::Woodcamp => self.map.terrain_at(tx, ty) == Terrain::Forest,
                    _ => self.map.terrain_at(tx, ty) == want && want.buildable(),
                };
                if !ok {
                    continue;
                }
                // Clear of anything already built.
                if self.nodes.values().any(|n| Map::dist(n.x, n.y, x, y) < 3.0) {
                    continue;
                }
                placed = Some((x, y));
                break;
            }

            // A capital must never go hungry: force a farm off the ring.
            if placed.is_none() && kind == NodeType::Farm {
                if let Some((tx, ty)) = self.nearest_tile_for(kind, cx as i32, cy as i32) {
                    placed = Some((tx as f32 + 0.5, ty as f32 + 0.5));
                }
            }

            if let Some((x, y)) = placed {
                let id = self.spawn_resource_owned(kind, x, y, owner, true);
                self.free_road(capital, id, owner);
                if kind == NodeType::Farm {
                    placed_farm = true;
                }
            }
        }

        // Last resort: nowhere on this map grows food. Stock the larder so the
        // capital survives while its lord sorts an economy out.
        if !placed_farm {
            if let Some(s) = self.nodes.get_mut(&capital) {
                s.stock.food = s.stock.food.max(STARTING_FOOD * 3);
            }
        }
    }

    /// Nearest in-bounds tile satisfying `kind`'s terrain rule, clear of
    /// existing nodes. Deterministic — used to guarantee a starter farm.
    fn nearest_tile_for(&self, kind: NodeType, cx: i32, cy: i32) -> Option<(i32, i32)> {
        for r in 3i32..=30 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if !self.map.in_bounds(x, y) {
                        continue;
                    }
                    let t = self.map.terrain_at(x, y);
                    let ok = match kind {
                        NodeType::Mine => {
                            self.map.deposit_at(x, y) != Deposit::None && t.buildable()
                        }
                        NodeType::Woodcamp => t == Terrain::Forest,
                        _ => t.buildable(),
                    };
                    if !ok {
                        continue;
                    }
                    if self
                        .nodes
                        .values()
                        .any(|n| Map::dist(n.x, n.y, x as f32, y as f32) < 4.0)
                    {
                        continue;
                    }
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// Free starter road (no gold charged). Falls back to a straight line when
    /// A* cannot route, so a starter link is never silently lost.
    fn free_road(&mut self, from: u32, to: u32, owner: Owner) -> bool {
        let Some(a) = self.nodes.get(&from) else {
            return false;
        };
        let Some(b) = self.nodes.get(&to) else {
            return false;
        };
        let (ax, ay) = (a.x as i32, a.y as i32);
        let (bx, by) = (b.x as i32, b.y as i32);
        let path = self
            .map
            .find_road((ax, ay), (bx, by))
            .unwrap_or_else(|| Map::straight_path((ax, ay), (bx, by)));
        let id = self.next_id;
        self.next_id += 1;
        self.edges.push(Edge {
            id,
            from,
            to,
            kind: EdgeType::Resource,
            by: owner,
            caravans: 1,
            progress: 0.0,
            path,
        });
        true
    }

    fn spawn_settlement(&mut self, owner: Owner, x: i32, y: i32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let name = place_name(self.rng.next_u64());
        let faction = match owner {
            Owner::Player => Some(self.player_faction),
            Owner::Enemy => Some(self.enemy_faction),
            _ => None,
        };
        let mut n = Node::new(
            id,
            NodeType::Settlement,
            owner,
            faction,
            x as f32,
            y as f32,
            name,
        );
        n.population = STARTING_POP;
        n.stock.food = STARTING_FOOD;
        n.stock.ore = 0;
        let dominant_race = match owner {
            Owner::Player => Some(self.player_faction.native_race()),
            Owner::Enemy => Some(self.enemy_faction.native_race()),
            _ => {
                let races = [
                    Race::Human,
                    Race::Beetlefolk,
                    Race::HiveSwarm,
                    Race::SilkSpinner,
                    Race::Reptilian,
                ];
                Some(races[self.rng.index(races.len())])
            }
        };
        n.dominant_race = dominant_race;
        if owner == Owner::Neutral {
            let specialties = [
                NeutralSpecialty::HighlandNomads,
                NeutralSpecialty::IronGuild,
                NeutralSpecialty::MysticOrder,
                NeutralSpecialty::RiverCorsairs,
            ];
            n.specialty = Some(specialties[self.rng.index(specialties.len())]);
        }
        self.nodes.insert(id, n);
        id
    }

    fn spawn_resource(&mut self, kind: NodeType, x: f32, y: f32) -> u32 {
        self.spawn_resource_owned(kind, x, y, Owner::Neutral, false)
    }

    /// Spawn a resource building. `crew` posts one ready novice — the starting
    /// ring is pre-crewed so a fresh realm is not born with zero employment.
    fn spawn_resource_owned(
        &mut self,
        kind: NodeType,
        x: f32,
        y: f32,
        owner: Owner,
        crew: bool,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let name = match kind {
            NodeType::Farm => place_name(self.rng.next_u64()) + " Farms",
            NodeType::Woodcamp => place_name(self.rng.next_u64()) + " Woods",
            NodeType::Mine => {
                let tx = x as i32;
                let ty = y as i32;
                match self.map.deposit_at(tx, ty) {
                    Deposit::Iron => place_name(self.rng.next_u64()) + " Ironworks",
                    _ => place_name(self.rng.next_u64()) + " Digs",
                }
            }
            _ => kind.label().into(),
        };
        let faction = match owner {
            Owner::Player => Some(self.player_faction),
            Owner::Enemy => Some(self.enemy_faction),
            _ => None,
        };
        let mut n = Node::new(id, kind, owner, faction, x, y, name);
        // A small stockpile so the first caravans have something to carry.
        match kind {
            NodeType::Farm => n.stock.food = 25,
            NodeType::Woodcamp => n.stock.timber = 20,
            NodeType::Mine => n.stock.ore = 15,
            _ => {}
        }
        if crew {
            let worker = agent_name(self.rng.next_u64());
            n.workers
                .push(Worker::new(kind.job(), TRAIN_SKILLS[0], 0, worker));
        }
        self.nodes.insert(id, n);
        id
    }

    /// Find a spot for a neutral settlement: far from existing nodes,
    /// passable, inside the map.
    fn find_spot(
        &mut self,
        existing: &[(f32, f32)],
        min_dist: f32,
        avoid_edges: bool,
    ) -> Option<(f32, f32)> {
        for _ in 0..300 {
            let x = self.rng.range_f(4.0, self.map.cols as f32 - 4.0);
            let y = self.rng.range_f(4.0, self.map.rows as f32 - 4.0);
            let tx = x as i32;
            let ty = y as i32;
            if self.map.terrain_at(tx, ty) != Terrain::Grass {
                continue;
            }
            if avoid_edges {
                // Keep neutrals off the extreme border.
                if tx < 6 || ty < 6 || tx > self.map.cols - 7 || ty > self.map.rows - 7 {
                    continue;
                }
            }
            if existing
                .iter()
                .all(|(ex, ey)| Map::dist(*ex, *ey, x, y) >= min_dist)
            {
                return Some((x, y));
            }
        }
        None
    }

    /// Find a spot suitable for a specific resource building.
    fn find_resource_spot(
        &mut self,
        kind: NodeType,
        existing: &[(f32, f32)],
        min_dist: f32,
    ) -> Option<(f32, f32)> {
        for _ in 0..400 {
            let x = self.rng.range_f(3.0, self.map.cols as f32 - 3.0);
            let y = self.rng.range_f(3.0, self.map.rows as f32 - 3.0);
            let tx = x as i32;
            let ty = y as i32;
            let terrain = self.map.terrain_at(tx, ty);

            let ok = match kind {
                NodeType::Mine => {
                    self.map.deposit_at(tx, ty) != Deposit::None && terrain.buildable()
                }
                NodeType::Woodcamp => terrain == Terrain::Forest,
                NodeType::Farm => terrain == Terrain::Grass,
                _ => terrain.buildable(),
            };
            if !ok {
                continue;
            }
            if existing
                .iter()
                .all(|(ex, ey)| Map::dist(*ex, *ey, x, y) >= min_dist)
            {
                return Some((x, y));
            }
        }
        None
    }

    /// Connect a capital to its nearest neutral resource (free starter road).
    fn starting_road(&mut self, settlement: u32) {
        let Some(cap) = self.nodes.get(&settlement) else {
            return;
        };
        let (cx, cy) = (cap.x, cap.y);
        let owner = cap.owner;

        let mut best: Option<(u32, f32)> = None;
        for n in self.nodes.values() {
            if n.owner != Owner::Neutral || !n.is_production() {
                continue;
            }
            if self.edge_between(settlement, n.id).is_some() {
                continue;
            }
            let d = Map::dist(cx, cy, n.x, n.y);
            if d < 10.0 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((n.id, d));
            }
        }
        if let Some((target, _)) = best {
            // Free road (it's part of the starting kit).
            let a = self.nodes[&settlement].clone();
            let b = self.nodes[&target].clone();
            if let Some(path) = self
                .map
                .find_road((a.x as i32, a.y as i32), (b.x as i32, b.y as i32))
            {
                let id = self.next_id;
                self.next_id += 1;
                self.edges.push(Edge {
                    id,
                    from: settlement,
                    to: target,
                    kind: EdgeType::Resource,
                    by: owner,
                    caravans: 1,
                    progress: 0.0,
                    path,
                });
            }
        }
    }

    /// Spawn an initial fortified stronghold linked to a capital by a Supply road,
    /// garrisoned with troops and commanded by a founding General.
    fn spawn_starter_fort(&mut self, capital_id: u32) {
        let Some(cap) = self.nodes.get(&capital_id) else {
            return;
        };
        let (cx, cy, owner, faction) = (cap.x, cap.y, cap.owner, cap.faction);
        let race = faction.map(|f| f.native_race()).unwrap_or(Race::Human);

        let mut placed: Option<(f32, f32)> = None;
        for _ in 0..150 {
            let ang = self.rng.range_f(0.0, std::f32::consts::TAU);
            let r = self.rng.range_f(3.5, 7.5);
            let x = cx + ang.cos() * r;
            let y = cy + ang.sin() * r;
            let tx = x as i32;
            let ty = y as i32;
            if !self.map.in_bounds(tx, ty) {
                continue;
            }
            let t = self.map.terrain_at(tx, ty);
            if !t.buildable() {
                continue;
            }
            if self.nodes.values().any(|n| Map::dist(n.x, n.y, x, y) < 3.0) {
                continue;
            }
            placed = Some((x, y));
            break;
        }

        if placed.is_none() {
            if let Some((tx, ty)) = self.nearest_tile_for(NodeType::Fort, cx as i32, cy as i32) {
                placed = Some((tx as f32 + 0.5, ty as f32 + 0.5));
            }
        }

        if let Some((x, y)) = placed {
            let fort_id = self.next_id;
            self.next_id += 1;
            let fort_name = format!("{} Fort", place_name(self.rng.next_u64()));
            let mut fort = Node::new(fort_id, NodeType::Fort, owner, faction, x, y, fort_name);
            fort.population = 5; // Initial garrison of 5 soldiers
            fort.dominant_race = Some(race);

            // Create founding General
            let gen_id = self.next_general_id;
            self.next_general_id += 1;
            let gen_name = format!("General {}", agent_name(self.rng.next_u64()));
            let mut general = General::new(gen_id, gen_name, owner, race, 65, 60, 90);
            general.fort_id = Some(fort_id);
            fort.general_id = Some(gen_id);
            self.generals.push(general);

            self.nodes.insert(fort_id, fort);

            // Connect Capital <-> Fort with a Supply road (reinforcements & order)
            self.free_supply_road(capital_id, fort_id, owner);
        }
    }

    /// Free starter Supply road (from settlement to fort).
    fn free_supply_road(&mut self, from: u32, to: u32, owner: Owner) -> bool {
        let Some(a) = self.nodes.get(&from) else {
            return false;
        };
        let Some(b) = self.nodes.get(&to) else {
            return false;
        };
        let (ax, ay) = (a.x as i32, a.y as i32);
        let (bx, by) = (b.x as i32, b.y as i32);
        let path = self
            .map
            .find_road((ax, ay), (bx, by))
            .unwrap_or_else(|| Map::straight_path((ax, ay), (bx, by)));
        let id = self.next_id;
        self.next_id += 1;
        self.edges.push(Edge {
            id,
            from,
            to,
            kind: EdgeType::Supply,
            by: owner,
            caravans: 0,
            progress: 0.0,
            path,
        });
        true
    }
}

/// Pick a random enemy faction different from the player's.
fn random_enemy_faction(player: Faction, seed: u64) -> Faction {
    let mut rng = Rng::new(seed ^ 0xDEAD_BEEF);
    let options: Vec<Faction> = FACTIONS
        .iter()
        .map(|f| f.id)
        .filter(|f| *f != player)
        .collect();
    options[rng.index(options.len())]
}
