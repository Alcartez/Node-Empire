//! Core game vocabulary: terrain, buildings, factions, resources, and the
//! serializable RNG that keeps every generated map reproducible from its seed.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Tunables — the whole balance of the game lives here.
// ---------------------------------------------------------------------------

/// Simulation steps per real second at 1x speed.
pub const TICKS_PER_SEC: f32 = 5.0;
/// Real seconds per simulation tick.
pub const TICK_DT: f32 = 1.0 / TICKS_PER_SEC;

pub const STARTING_GOLD: i32 = 300;
pub const STARTING_POP: i32 = 30;
pub const STARTING_FOOD: i32 = 60;

/// Economic victory threshold.
pub const WIN_GOLD: i32 = 5000;
/// Web victory: fraction of enemy settlements that must be bound
/// (embedded spy + high influence).
pub const WEB_VICTORY_RATIO: f32 = 1.0;

pub const MIN_EDGE_TILES: f32 = 1.5;
/// Straight-line max length for drawing a road.
pub const MAX_EDGE_TILES: f32 = 14.0;
/// A* may wander up to this multiple of the straight-line distance.
pub const MAX_PATH_FACTOR: f32 = 2.6;
pub const EDGE_BASE_COST: i32 = 5;

pub const MAX_CARRAVANS: i32 = 6;
pub const TILE_TRAVEL_TICKS: f32 = 2.0; // caravan ticks per tile of path

// Worker training (gold cost, starting skill, seconds of training baked into tiers)
pub const TRAIN_COSTS: [i32; 3] = [10, 40, 90]; // basic, specialist, expert
pub const TRAIN_SKILLS: [i32; 3] = [10, 40, 80];

pub const SPY_TRAIN_COST: i32 = 75;
pub const SPY_TRAVEL_TICKS_PER_TILE: f32 = 4.0;

// Settlement tiers
pub const TIER_LIMITS: [i32; 3] = [100, 500, 2000]; // village < 100 < town < 500 < city < 2000 < metropolis

// ---------------------------------------------------------------------------
// Public order & population growth
//
// Order is the civic backbone: it drags happiness along with it, happiness
// drives taxes and the growth roll. Its only two inputs are *employment*
// (share of a realm's workable posts that are actually filled) and a *fort
// supply link* (a garrison keeping the peace). A realm with neither sits
// stable but stagnant; a realm that develops either one starts to grow.
// ---------------------------------------------------------------------------

/// Order a fed, settled realm holds through inertia alone.
pub const ORDER_CIVIC_BASE: f32 = 32.0;
/// Order earned per percentage point of employment.
pub const ORDER_EMPLOYMENT_WEIGHT: f32 = 0.6;
/// Order granted by a Supply road to one of your forts.
pub const ORDER_FORT_BONUS: f32 = 20.0;
/// Per-tick pull of order toward its target.
pub const ORDER_PULL: f32 = 0.05;
/// Small permanent civic drift (keeps order off hard zero).
pub const ORDER_DRIFT: f32 = 0.1;
/// Order lost while a settlement is under siege.
pub const ORDER_SIEGE_PENALTY: f32 = 25.0;

/// Happiness floor for the growth roll.
pub const GROWTH_MIN_HAPPINESS: f32 = 55.0;
/// Public-order floor for the growth roll.
pub const GROWTH_MIN_ORDER: f32 = 40.0;

// ---------------------------------------------------------------------------
// Garrisons (scaled well beyond their Seven Kingdoms root: a village fort
// holds ~8 soldiers, a metropolis citadel ~20, and never more than a fifth
// of the people it protects).
// ---------------------------------------------------------------------------

/// Share of a settlement's population a fort may hold as garrison.
pub const GARRISON_SHARE: f32 = 0.20;
/// Absolute garrison ceiling per settlement tier (village … metropolis).
pub const GARRISON_CEILING: [i32; 4] = [8, 12, 16, 20];

/// How many troops a fort may hold, given the people supporting it.
///
/// `pool_pop` is village + garrison together. Taking `share / (1 + share)` of
/// the pool settles the split at exactly [`GARRISON_SHARE`] of the *village*:
/// a village of 30 keeps 25 while its fort holds 5, and the settlement is
/// never stripped bare to feed a garrison. The result is capped by tier, so a
/// village fort holds ~8 spears and a metropolis citadel ~20.
pub fn garrison_cap(tier: u8, pool_pop: i32) -> i32 {
    let share = (pool_pop as f32 * GARRISON_SHARE / (1.0 + GARRISON_SHARE)).round() as i32;
    share.clamp(1, GARRISON_CEILING[(tier as usize).min(GARRISON_CEILING.len() - 1)])
}

// Building costs
pub const BUILD_COSTS: [(NodeType, i32); 8] = [
    (NodeType::Settlement, 200),
    (NodeType::Farm, 50),
    (NodeType::Mine, 75),
    (NodeType::Woodcamp, 40),
    (NodeType::Factory, 100),
    (NodeType::Market, 100),
    (NodeType::Fort, 125),
    (NodeType::Laboratory, 150),
];

pub fn build_cost(kind: NodeType) -> i32 {
    BUILD_COSTS.iter().find(|(t, _)| *t == kind).map(|(_, c)| *c).unwrap_or(50)
}

// ---------------------------------------------------------------------------
// RNG — xorshift64*, fully serializable so saves resume identically.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform in [0, 1).
    pub fn float(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in [lo, hi).
    pub fn range_f(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.float() * (hi - lo)
    }

    /// Uniform in [lo, hi] (inclusive, integers).
    pub fn range_i(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo + 1) as u64) as i32
    }

    /// Index into a slice.
    pub fn index(&mut self, len: usize) -> usize {
        if len == 0 {
            0
        } else {
            (self.next_u64() % len as u64) as usize
        }
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.float() < p
    }
}

// ---------------------------------------------------------------------------
// Map
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terrain {
    Water,
    Sand,
    Grass,
    Forest,
    Mountain,
}

impl Terrain {
    /// Water and mountains block roads (and building placement).
    pub fn passable(&self) -> bool {
        !matches!(self, Terrain::Water | Terrain::Mountain)
    }

    pub fn buildable(&self) -> bool {
        matches!(self, Terrain::Grass | Terrain::Sand | Terrain::Forest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Deposit {
    None,
    Copper,
    Iron,
}

// ---------------------------------------------------------------------------
// Buildings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Settlement,
    Farm,
    Mine,
    Woodcamp,
    Factory,
    Market,
    Fort,
    Laboratory,
    BanditCamp,
}

impl NodeType {
    pub fn label(&self) -> &'static str {
        match self {
            NodeType::Settlement => "Settlement",
            NodeType::Farm => "Farm",
            NodeType::Mine => "Mine",
            NodeType::Woodcamp => "Woodcamp",
            NodeType::Factory => "Factory",
            NodeType::Market => "Market",
            NodeType::Fort => "Fort",
            NodeType::Laboratory => "Laboratory",
            NodeType::BanditCamp => "Bandit Camp",
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            NodeType::Settlement => "SET",
            NodeType::Farm => "FRM",
            NodeType::Mine => "MNE",
            NodeType::Woodcamp => "WOD",
            NodeType::Factory => "FAC",
            NodeType::Market => "MKT",
            NodeType::Fort => "FRT",
            NodeType::Laboratory => "LAB",
            NodeType::BanditCamp => "BND",
        }
    }

    /// The worker profession this building trains/uses.
    pub fn job(&self) -> JobType {
        match self {
            NodeType::Farm => JobType::Farmer,
            NodeType::Mine => JobType::Miner,
            NodeType::Woodcamp => JobType::Lumberjack,
            NodeType::Factory | NodeType::Market => JobType::Manufacturer,
            NodeType::Fort => JobType::Soldier,
            NodeType::Laboratory => JobType::Scholar,
            _ => JobType::Peasant,
        }
    }

    pub fn is_buildable(&self) -> bool {
        !matches!(self, NodeType::BanditCamp)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Owner {
    Player,
    Enemy,
    Neutral,
    Bandit,
}

impl Owner {
    pub fn is_hostile_to(self, other: Owner) -> bool {
        matches!(
            (self, other),
            (Owner::Bandit, _) | (_, Owner::Bandit) | (Owner::Player, Owner::Enemy) | (Owner::Enemy, Owner::Player)
        )
    }
}

// ---------------------------------------------------------------------------
// Factions (flavour drawn from factions.md)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    SilkSpinners,
    Breplid,
    Hapset,
    Argo,
    Mez,
    HiveSwarms,
}

// ---------------------------------------------------------------------------
// Races & Demographics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Race {
    Human,
    Beetlefolk,
    HiveSwarm,
    SilkSpinner,
    Reptilian,
}

impl Race {
    pub fn label(&self) -> &'static str {
        match self {
            Race::Human => "Human",
            Race::Beetlefolk => "Beetlefolk",
            Race::HiveSwarm => "Hive Swarm",
            Race::SilkSpinner => "Silk Spinner",
            Race::Reptilian => "Reptilian",
        }
    }

    /// Settlement population growth multiplier.
    pub fn growth_multiplier(&self) -> f32 {
        match self {
            Race::Human => 1.0,
            Race::Beetlefolk => 0.85,
            Race::HiveSwarm => 1.45,
            Race::SilkSpinner => 1.05,
            Race::Reptilian => 0.90,
        }
    }

    /// Food consumption multiplier per citizen.
    pub fn food_consumption_multiplier(&self) -> f32 {
        match self {
            Race::Human => 1.0,
            Race::Beetlefolk => 1.0,
            Race::HiveSwarm => 1.30,
            Race::SilkSpinner => 0.95,
            Race::Reptilian => 0.75, // Cold-blooded efficiency
        }
    }

    /// Combat power multiplier in battles.
    pub fn combat_multiplier(&self) -> f32 {
        match self {
            Race::Human => 1.0,
            Race::Beetlefolk => 1.20,
            Race::HiveSwarm => 0.85,
            Race::SilkSpinner => 0.90,
            Race::Reptilian => 1.15,
        }
    }

    /// Defensive power multiplier when holding a fort or town.
    pub fn defense_multiplier(&self) -> f32 {
        match self {
            Race::Human => 1.0,
            Race::Beetlefolk => 1.35, // Thick carapace armor
            Race::HiveSwarm => 0.85,
            Race::SilkSpinner => 1.0,
            Race::Reptilian => 1.15,
        }
    }
}

// ---------------------------------------------------------------------------
// Equipment — Fort arms tiers supplied by Mines and Factories
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EquipmentTier {
    Militia,    // Wood/militia
    Bronze,     // Copper/bronze weapons
    IronClad,   // Iron weapons and mail
    SteelSiege, // Steel plate and siege weapons
}

impl Default for EquipmentTier {
    fn default() -> Self {
        EquipmentTier::Militia
    }
}

impl EquipmentTier {
    pub fn label(&self) -> &'static str {
        match self {
            EquipmentTier::Militia => "Militia (Wood)",
            EquipmentTier::Bronze => "Bronze",
            EquipmentTier::IronClad => "Iron-Clad",
            EquipmentTier::SteelSiege => "Steel & Siege",
        }
    }

    pub fn combat_mult(&self) -> f32 {
        match self {
            EquipmentTier::Militia => 1.0,
            EquipmentTier::Bronze => 1.25,
            EquipmentTier::IronClad => 1.60,
            EquipmentTier::SteelSiege => 2.00,
        }
    }

    pub fn defense_mult(&self) -> f32 {
        match self {
            EquipmentTier::Militia => 1.0,
            EquipmentTier::Bronze => 1.15,
            EquipmentTier::IronClad => 1.35,
            EquipmentTier::SteelSiege => 1.60,
        }
    }

    pub fn next(&self) -> Option<EquipmentTier> {
        match self {
            EquipmentTier::Militia => Some(EquipmentTier::Bronze),
            EquipmentTier::Bronze => Some(EquipmentTier::IronClad),
            EquipmentTier::IronClad => Some(EquipmentTier::SteelSiege),
            EquipmentTier::SteelSiege => None,
        }
    }

    pub fn upgrade_cost(&self) -> i32 {
        match self {
            EquipmentTier::Militia => 10,   // 10 goods
            EquipmentTier::Bronze => 25,    // 25 goods
            EquipmentTier::IronClad => 50,  // 50 goods
            EquipmentTier::SteelSiege => 0,
        }
    }
}

impl Faction {
    pub fn native_race(&self) -> Race {
        match self {
            Faction::SilkSpinners => Race::SilkSpinner,
            Faction::Breplid | Faction::Hapset | Faction::Argo => Race::Human,
            Faction::Mez => Race::Reptilian,
            Faction::HiveSwarms => Race::HiveSwarm,
        }
    }
}

// ---------------------------------------------------------------------------
// Generals — Military commanders with leadership, combat, and fragile loyalty
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct General {
    pub id: u32,
    pub name: String,
    pub owner: Owner,
    pub fort_id: Option<u32>,
    pub leadership: u8,
    pub combat: u8,
    pub loyalty: u8,
    pub salary: i32,
    pub race: Race,
    pub kills: u32,
}

impl General {
    pub fn new(
        id: u32,
        name: String,
        owner: Owner,
        race: Race,
        leadership: u8,
        combat: u8,
        loyalty: u8,
    ) -> Self {
        Self {
            id,
            name,
            owner,
            fort_id: None,
            leadership,
            combat,
            loyalty,
            salary: 15,
            race,
            kills: 0,
        }
    }

    /// Order bonus contributed to towns connected via Supply road.
    pub fn fort_order_bonus(&self) -> f32 {
        ORDER_FORT_BONUS * (0.8 + 0.4 * (self.leadership as f32 / 100.0))
    }

    /// Combat power multiplier contributed by this commander.
    pub fn combat_multiplier(&self) -> f32 {
        1.0 + (self.combat as f32 / 100.0) * 0.5
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FactionDef {
    pub id: Faction,
    pub name: &'static str,
    pub motto: &'static str,
    pub color: [u8; 3],
    /// combat power multiplier
    pub combat: f32,
    /// production multiplier
    pub production: f32,
    /// spy success multiplier
    pub spy: f32,
    /// defensive multiplier
    pub defense: f32,
    /// trade/sale income multiplier
    pub trade: f32,
    /// population growth multiplier
    pub growth: f32,
    /// flat happiness modifier
    pub happiness: i32,
    pub blurb: &'static str,
}

pub const FACTIONS: [FactionDef; 6] = [
    FactionDef {
        id: Faction::SilkSpinners,
        name: "The Silk Spinners",
        motto: "Every court has a thread we hold",
        color: [178, 130, 255],
        combat: 0.80,
        production: 0.95,
        spy: 1.45,
        defense: 0.90,
        trade: 1.15,
        growth: 1.0,
        happiness: 0,
        blurb: "Espionage masters. Weaker armies, unmatched web of agents.",
    },
    FactionDef {
        id: Faction::Breplid,
        name: "The Breplid Clans",
        motto: "Peace is only the pause between raids",
        color: [235, 110, 90],
        combat: 1.35,
        production: 0.95,
        spy: 0.85,
        defense: 1.15,
        trade: 0.70,
        growth: 1.0,
        happiness: -5,
        blurb: "Xenophobic warriors. Devastating in battle, poor traders.",
    },
    FactionDef {
        id: Faction::Hapset,
        name: "The Hapset Remnant",
        motto: "Stone endures where blood does not",
        color: [120, 200, 235],
        combat: 1.0,
        production: 1.0,
        spy: 0.95,
        defense: 1.35,
        trade: 1.0,
        growth: 1.0,
        happiness: 10,
        blurb: "Defensive builders. Forts and walls are their answer to war.",
    },
    FactionDef {
        id: Faction::Argo,
        name: "The Argo Combine",
        motto: "A well-run ledger wins wars",
        color: [255, 200, 90],
        combat: 0.90,
        production: 1.30,
        spy: 0.95,
        defense: 1.0,
        trade: 1.25,
        growth: 0.90,
        happiness: -5,
        blurb: "Merchant-industrialists. Build fast, sell everything.",
    },
    FactionDef {
        id: Faction::Mez,
        name: "The Mez Cabal",
        motto: "We already know what you will do",
        color: [110, 230, 180],
        combat: 0.85,
        production: 1.05,
        spy: 1.30,
        defense: 0.95,
        trade: 1.10,
        growth: 1.0,
        happiness: 0,
        blurb: "Manipulators and informants. Money talks, secrets shout.",
    },
    FactionDef {
        id: Faction::HiveSwarms,
        name: "The Hive Swarms",
        motto: "One is nothing, ten thousand is everything",
        color: [240, 130, 60],
        combat: 1.10,
        production: 1.10,
        spy: 0.90,
        defense: 0.85,
        trade: 0.90,
        growth: 1.45,
        happiness: -5,
        blurb: "Endless reproduction. Overwhelm before they can fortify.",
    },
];

pub fn faction_def(f: Faction) -> &'static FactionDef {
    FACTIONS.iter().find(|d| d.id == f).unwrap()
}

// ---------------------------------------------------------------------------
// Ages / research
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Age {
    Wood,
    Copper,
    Iron,
    Steel,
}

impl Age {
    pub fn label(&self) -> &'static str {
        match self {
            Age::Wood => "Wood Age",
            Age::Copper => "Copper Age",
            Age::Iron => "Iron Age",
            Age::Steel => "Steel Age",
        }
    }

    /// Tech points required to advance *from* this age.
    pub fn cost_to_advance(&self) -> Option<i32> {
        match self {
            Age::Wood => Some(120),
            Age::Copper => Some(300),
            Age::Iron => Some(650),
            Age::Steel => None,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Age::Wood => 0,
            Age::Copper => 1,
            Age::Iron => 2,
            Age::Steel => 3,
        }
    }

    pub fn from_index(i: usize) -> Age {
        match i {
            0 => Age::Wood,
            1 => Age::Copper,
            2 => Age::Iron,
            _ => Age::Steel,
        }
    }

    /// Production bonus: +10% per age above Wood.
    pub fn production_mult(&self) -> f32 {
        1.0 + 0.10 * self.index() as f32
    }

    /// Combat bonus: +15% per age above Wood.
    pub fn combat_mult(&self) -> f32 {
        1.0 + 0.15 * self.index() as f32
    }

    /// Espionage skill bonus.
    pub fn spy_bonus(&self) -> i32 {
        self.index() as i32 * 8
    }
}

// ---------------------------------------------------------------------------
// Workers & jobs (Seven-Kingdoms-style skill growth)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    Peasant,
    Farmer,
    Miner,
    Lumberjack,
    Manufacturer,
    Soldier,
    Scholar,
}

impl JobType {
    pub fn label(&self) -> &'static str {
        match self {
            JobType::Peasant => "Peasant",
            JobType::Farmer => "Farmer",
            JobType::Miner => "Miner",
            JobType::Lumberjack => "Lumberjack",
            JobType::Manufacturer => "Manufacturer",
            JobType::Soldier => "Soldier",
            JobType::Scholar => "Scholar",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub job: JobType,
    /// 0..100 — boosts production, combat, research.
    pub skill: i32,
    /// Ticks until in-game training completes (0 = ready).
    pub training_left: u32,
    pub name: String,
}

impl Worker {
    pub fn new(job: JobType, skill: i32, training: u32, name: String) -> Self {
        Self {
            job,
            skill,
            training_left: training,
            name,
        }
    }

    pub fn ready(&self) -> bool {
        self.training_left == 0
    }

    /// Production multiplier contributed by a ready worker.
    pub fn boost(&self) -> f32 {
        if self.ready() {
            1.0 + self.skill as f32 / 200.0
        } else {
            1.0
        }
    }
}

// ---------------------------------------------------------------------------
// Edges
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    /// Farms/mines/factories → settlements: goods and food flow.
    Resource,
    /// Same-owner settlement ↔ fort: garrison reinforcement.
    Supply,
    /// Fort ↔ fort (hostile): they fight.
    Combat,
    /// Fort → hostile settlement: siege.
    Siege,
    /// Non-fort → hostile settlement: trade route (influence & risk).
    Trade,
}

impl EdgeType {
    pub fn label(&self) -> &'static str {
        match self {
            EdgeType::Resource => "Road",
            EdgeType::Supply => "Supply",
            EdgeType::Combat => "Battle",
            EdgeType::Siege => "Siege",
            EdgeType::Trade => "Trade Route",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub kind: EdgeType,
    /// Who paid for this road.
    pub by: Owner,
    /// Parallel caravans (speeds transfers).
    pub caravans: i32,
    /// Ticks accumulated toward the next transfer.
    pub progress: f32,
    /// A* path as tile coordinates, for rendering and distance.
    pub path: Vec<(i32, i32)>,
}

impl Edge {
    pub fn length(&self) -> f32 {
        self.path.len().max(1) as f32
    }

    pub fn other(&self, id: u32) -> u32 {
        if self.from == id {
            self.to
        } else {
            self.from
        }
    }
}

// ---------------------------------------------------------------------------
// Spies — the signature system of Node Empire
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpyOp {
    /// Passive: reveals target info, +influence.
    Infiltrate,
    /// Steal gold from the target over time.
    Siphon,
    /// Halve target production for a while.
    Sabotage,
    /// Erode loyalty/happiness until the settlement rebels.
    Incite,
}

impl SpyOp {
    pub fn label(&self) -> &'static str {
        match self {
            SpyOp::Infiltrate => "Infiltrate",
            SpyOp::Siphon => "Siphon",
            SpyOp::Sabotage => "Sabotage",
            SpyOp::Incite => "Incite",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            SpyOp::Infiltrate => "Reveal the target's secrets, boost influence over it",
            SpyOp::Siphon => "Skim gold from the target's treasury each tick",
            SpyOp::Sabotage => "Halve the target's production while embedded",
            SpyOp::Incite => "Whisper dissent: erode loyalty until rebellion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpyStatus {
    /// Walking to the target through the wilds.
    Traveling,
    /// At the target, running an operation.
    Embedded,
    /// Heading home (recalled or op finished).
    Returning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spy {
    pub id: u32,
    pub owner: Owner,
    pub name: String,
    /// Home settlement.
    pub home: u32,
    pub target: u32,
    pub status: SpyStatus,
    pub op: SpyOp,
    pub skill: i32,
    /// 0..1 along the current leg.
    pub progress: f32,
    /// Ticks embedded — operation duration.
    pub embed_ticks: u32,
    /// Total gold siphoned (for flavor stats / web victory flavor).
    pub siphoned: i32,
}

// ---------------------------------------------------------------------------
// Misc state containers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub fn label(&self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        }
    }

    /// AI thinking cadence in ticks (lower = smarter/faster reactions).
    pub fn ai_interval(&self) -> u64 {
        match self {
            Difficulty::Easy => 60,
            Difficulty::Medium => 35,
            Difficulty::Hard => 18,
        }
    }

    /// Free gold the AI secretly earns per interval.
    pub fn ai_income_bonus(&self) -> i32 {
        match self {
            Difficulty::Easy => 0,
            Difficulty::Medium => 15,
            Difficulty::Hard => 40,
        }
    }

    pub fn ai_aggression(&self) -> f32 {
        match self {
            Difficulty::Easy => 0.6,
            Difficulty::Medium => 1.0,
            Difficulty::Hard => 1.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Victory {
    Economic,
    Domination,
    Web,
    Defeat,
}

impl Victory {
    pub fn title(&self) -> &'static str {
        match self {
            Victory::Economic => "Economic Victory",
            Victory::Domination => "Domination Victory",
            Victory::Web => "The Silent Web",
            Victory::Defeat => "Your Court Falls Silent",
        }
    }

    pub fn blurb(&self) -> &'static str {
        match self {
            Victory::Economic => "Your treasury swells beyond all rivals. Coins win wars.",
            Victory::Domination => "Every enemy settlement flies your banner now.",
            Victory::Web => "Every enemy court dances on your strings. No blade was needed.",
            Victory::Defeat => "Your last settlement has fallen. The network is severed.",
        }
    }
}

/// Resource stock held locally at a node. Shipped along roads by caravans.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Stock {
    pub food: i32,
    pub ore: i32,
    pub timber: i32,
    pub goods: i32,
}

impl Stock {
    pub fn total(&self) -> i32 {
        self.food + self.ore + self.timber + self.goods
    }
}
