use eframe::egui;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn is_valid_position(
    x: f32,
    y: f32,
    nodes: &HashMap<usize, GameNode>,
    min_dist: f32,
) -> bool {
    for node in nodes.values() {
        let dx = node.x - x;
        let dy = node.y - y;
        if (dx * dx + dy * dy).sqrt() < min_dist {
            return false;
        }
    }
    true
}

const WIN_THRESHOLD: i32 = 5000;
const STARTING_GOLD: i32 = 100;
const STARTING_FOOD: i32 = 100;
const STARTING_POP: i32 = 10;

const TRAIN_BASIC_COST: i32 = 10; // skill 10
const TRAIN_SPECIALIST_COST: i32 = 40; // skill 40
const TRAIN_EXPERT_COST: i32 = 80; // skill 80
const GOLD_TICK_INTERVAL: u64 = 60; // gold produced every 60 ticks (1 second)

// Zoom
const ZOOM_SENSITIVITY: f32 = 0.05;
const ZOOM_MIN: f32 = 0.3;
const ZOOM_MAX: f32 = 2.0;

// Training time: 5 ticks per skill point
fn training_time(skill: i32) -> u64 {
    (skill as u64) * 5
}

const MINER_PRODUCTION: i32 = 2;
const MFGR_PRODUCTION: i32 = 2;
const FARMER_PRODUCTION: i32 = 3;
const MIN_NODE_DISTANCE: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Settlement,
    Farm,
    Mine,
    Factory,
    Fort,
    CopperMine,
    IronMine,
    Woodcamp,
    PlankIndustry,
    CopperFactory,
    IronFactory,
    Market,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Owner {
    Player,
    Enemy,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    Peasant,
    Miner,
    Manufacturer,
    Constructor,
    Soldier,
    Lumberjack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OreType {
    None,
    Copper,
    Iron,
}

impl Default for OreType {
    fn default() -> Self {
        OreType::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    Resource,   // Building ↔ Building: Transfers resources
    Supply,     // Settlement ↔ Fort: Feeds population to forts
    Combat,     // Fort ↔ Fort: Triggers auto-combat
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    job: JobType,
    skill: i32,
    location: usize,
    training_tier: i8,          // 0 = none, 1 = basic, 2 = specialist, 3 = expert
    training_target_skill: i32, // target skill when training completes
    training_end_tick: u64,     // tick when training completes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameNode {
    id: usize,
    node_type: NodeType,
    owner: Owner,
    x: f32,
    y: f32,
    population: i32,
    gold: i32,
    food: i32,
    ore: i32,
    goods: i32,
    workers: Vec<Worker>,
    label: String,
    copper: i32,
    iron: i32,
    firewood: i32,
    planks: i32,
    copper_goods: i32,
    iron_goods: i32,
    ore_type: OreType,
}

impl GameNode {
    fn new(id: usize, node_type: NodeType, owner: Owner, x: f32, y: f32) -> Self {
        Self::new_with_ore_type(id, node_type, owner, x, y, OreType::None)
    }

    fn new_with_ore_type(
        id: usize,
        node_type: NodeType,
        owner: Owner,
        x: f32,
        y: f32,
        ore_type: OreType,
    ) -> Self {
        let label = match owner {
            Owner::Player => "You".to_string(),
            Owner::Enemy => "Enemy".to_string(),
            Owner::Neutral => match node_type {
                NodeType::Settlement => "Town".to_string(),
                NodeType::Mine | NodeType::CopperMine => "Mine".to_string(),
                NodeType::IronMine => "IMine".to_string(),
                NodeType::Farm => "Farm".to_string(),
                NodeType::Factory | NodeType::CopperFactory | NodeType::IronFactory => {
                    "Factory".to_string()
                }
                NodeType::Woodcamp => "Wood".to_string(),
                NodeType::PlankIndustry => "Plank".to_string(),
                NodeType::Market => "Market".to_string(),
                _ => "Building".to_string(),
            },
        };
        Self {
            id,
            node_type,
            owner,
            x,
            y,
            population: 0,
            gold: 0,
            food: 0,
            ore: 0,
            goods: 0,
            workers: Vec::new(),
            label,
            copper: 0,
            iron: 0,
            firewood: 0,
            planks: 0,
            copper_goods: 0,
            iron_goods: 0,
            ore_type,
        }
    }

    fn count_workers_by_job(&self, job: JobType) -> i32 {
        self.workers.iter().filter(|w| w.job == job).count() as i32
    }

    fn count_miners(&self) -> i32 {
        self.count_workers_by_job(JobType::Miner)
    }

    fn count_manufacturers(&self) -> i32 {
        self.count_workers_by_job(JobType::Manufacturer)
    }

    fn count_farmers(&self) -> i32 {
        self.count_workers_by_job(JobType::Peasant)
    }

    fn count_lumberjacks(&self) -> i32 {
        self.count_workers_by_job(JobType::Lumberjack)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    from: usize,
    to: usize,
    edge_type: EdgeType,
    caravans: i32,         // Number of caravans on this edge (0 = no transfer)
    ticks_accumulated: u64, // Progress toward next transfer
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    nodes: HashMap<usize, GameNode>,
    edges: Vec<EdgeData>,
    player_gold: i32,
    player_food: i32,
    player_ore: i32,
    player_goods: i32,
    selected_node: Option<usize>,
    selected_edge: Option<usize>, // Index in edges vector
    connecting_from: Option<usize>,
    tick_count: u64,
    game_over: bool,
    victory: Option<bool>,
    show_help: bool,
    camera_x: f32,
    camera_y: f32,
    camera_zoom: f32,
    prev_gold: i32, // Track gold trend for King Rating
}

fn connection_cost(from: &GameNode, to: &GameNode) -> i32 {
    let dx = from.x - to.x;
    let dy = from.y - to.y;
    let dist = ((dx * dx + dy * dy).sqrt() as i32).abs();
    5 + dist
}

impl GameState {
    fn new() -> Self {
        let seed = 12345u64;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut nodes = HashMap::new();

        nodes.insert(0, {
            let mut n = GameNode::new(0, NodeType::Settlement, Owner::Player, 10.0, 10.0);
            n.population = STARTING_POP;
            n
        });
        
        nodes.insert(1, {
            let mut n = GameNode::new(1, NodeType::Settlement, Owner::Enemy, 90.0, 90.0);
            n.population = STARTING_POP;
            n
        });

        Self {
            nodes,
            edges: Vec::new(),
            player_gold: STARTING_GOLD,
            player_food: STARTING_FOOD,
            player_ore: 0,
            player_goods: 0,
            selected_node: None,
            selected_edge: None,
            connecting_from: None,
            tick_count: 0,
            game_over: false,
            victory: None,
            show_help: false,
            camera_x: 700.0,
            camera_y: 700.0,
            camera_zoom: 1.0,
            prev_gold: STARTING_GOLD,
        }
    }
                attempts += 1;
                if attempts > 100 {
                    x = 5.0 + i as f32;
                    y = 10.0 + i as f32;
                    break;
                }
            }
            let mut n = GameNode::new(i, NodeType::Settlement, Owner::Neutral, x, y);
            n.population = rng.gen_range(5..20);
            n.label = format!("N{}", i);
            nodes.insert(i, n);
        }

        // Copper mines with better spacing
        for i in 8..10 {
            let mut x: f32;
            let mut y: f32;
            let mut attempts = 0;
            loop {
                x = rng.gen_range(5..95) as f32;
                y = rng.gen_range(5..95) as f32;
                if is_valid_position(x, y, &nodes, MIN_NODE_DISTANCE) {
                    break;
                }
                attempts += 1;
                if attempts > 100 {
                    x = 20.0 + (i - 8) as f32 * 2.0;
                    y = 10.0;
                    break;
                }
            }
            
            let mut n = GameNode::new_with_ore_type(
                i,
                NodeType::Mine,
                Owner::Neutral,
                x,
                y,
                OreType::Copper,
            );
            n.label = format!("Cu{}", i - 7);
            nodes.insert(i, n);
        }

        // Iron mine with better spacing
        let mut x: f32;
        let mut y: f32;
        let mut attempts = 0;
        loop {
            x = rng.gen_range(5..95) as f32;
            y = rng.gen_range(5..95) as f32;
            if is_valid_position(x, y, &nodes, MIN_NODE_DISTANCE) {
                break;
            }
            attempts += 1;
            if attempts > 100 {
                x = 20.0 + (i - 8) as f32 * 2.0;
                y = 10.0;
                break;
            }
        }
        let mut n = GameNode::new_with_ore_type(
                i,
                NodeType::Mine,
                Owner::Neutral,
                x,
                y,
                OreType::Copper,
            );
            n.label = format!("Cu{}", i - 7);
            nodes.insert(i, n);
        }

        // Iron mine with better spacing
        let mut x: f32;
        let mut y: f32;
        let mut attempts = 0;
        loop {
            x = rng.gen_range(5..95) as f32;
            y = rng.gen_range(5..95) as f32;
            if is_valid_position(x, y, &nodes, MIN_NODE_DISTANCE) {
                break;
            }
            attempts += 1;
            if attempts > 100 {
                x = 30.0;
                y = 10.0;
                break;
            }
        }
        let mut n =
            GameNode::new_with_ore_type(10, NodeType::Mine, Owner::Neutral, x, y, OreType::Iron);
        n.label = "Fe".to_string();
        nodes.insert(10, n);

        // Woodcamps with better spacing
        for i in 11..15 {
            let mut x: f32;
            let mut y: f32;
            let mut attempts = 0;
            loop {
                x = rng.gen_range(5..95) as f32;
                y = rng.gen_range(5..95) as f32;
                if is_valid_position(x, y, &nodes, MIN_NODE_DISTANCE) {
                    break;
                }
                attempts += 1;
                if attempts > 100 {
                    x = 35.0 + (i - 11) as f32 * 2.0;
                    y = 10.0;
                    break;
                }
            }
            let mut n = GameNode::new(i, NodeType::Woodcamp, Owner::Neutral, x, y);
            n.label = "Wood".to_string();
            nodes.insert(i, n);
        }

        // Farms
        for i in 15..19 {
            let mut x: f32;
            let mut y: f32;
            let mut attempts = 0;
            loop {
                x = rng.gen_range(5..95) as f32;
                y = rng.gen_range(5..95) as f32;
                if is_valid_position(x, y, &nodes, MIN_NODE_DISTANCE) {
                    break;
                }
                attempts += 1;
                if attempts > 100 {
                    x = 45.0 + (i - 15) as f32 * 2.0;
                    y = 10.0;
                    break;
                }
            }
            let mut n = GameNode::new(i, NodeType::Farm, Owner::Neutral, x, y);
            n.label = format!("Farm{}", i - 14);
            nodes.insert(i, n);
        }

        // Factories
        for i in 19..23 {
            let mut x: f32;
            let mut y: f32;
            let mut attempts = 0;
            loop {
                x = rng.gen_range(5..95) as f32;
                y = rng.gen_range(5..95) as f32;
                if is_valid_position(x, y, &nodes, MIN_NODE_DISTANCE) {
                    break;
                }
                attempts += 1;
                if attempts > 100 {
                    x = 55.0 + (i - 19) as f32 * 2.0;
                    y = 10.0;
                    break;
                }
            }
            let mut n = GameNode::new(i, NodeType::Factory, Owner::Neutral, x, y);
            n.label = format!("Factory{}", i - 18);
            nodes.insert(i, n);
        }

        Self {
            nodes,
            edges: Vec::new(),
            player_gold: STARTING_GOLD,
            player_food: STARTING_FOOD,
            player_ore: 0,
            player_goods: 0,
            selected_node: None,
            selected_edge: None,
            connecting_from: None,
            tick_count: 0,
            game_over: false,
            victory: None,
            show_help: false,
            camera_x: 700.0,
            camera_y: 700.0,
            camera_zoom: 1.0,
            prev_gold: STARTING_GOLD,
        }
    }

    fn train_worker(&mut self, node_id: usize, tier: i8, target_skill: i32) -> bool {
        // tier: 1 = basic (10), 2 = specialist (40), 3 = expert (80)
        let cost = match tier {
            1 => TRAIN_BASIC_COST,
            2 => TRAIN_SPECIALIST_COST,
            3 => TRAIN_EXPERT_COST,
            _ => return false,
        };

        let node = match self.nodes.get(&node_id) {
            Some(n) => n,
            None => return false,
        };

        if node.node_type != NodeType::Settlement || node.owner != Owner::Player {
            return false;
        }

        if self.player_gold < cost {
            return false;
        }

        self.player_gold -= cost;

        let worker = Worker {
            job: JobType::Peasant,
            skill: target_skill, // starts at target skill, trains up
            location: node_id,
            training_tier: tier,
            training_target_skill: target_skill,
            training_end_tick: self.tick_count + training_time(target_skill),
        };

        if let Some(n) = self.nodes.get_mut(&node_id) {
            n.workers.push(worker);
            n.population += 1;
        }

        true
    }

    fn get_connected_nodes(&self, node_id: usize) -> Vec<usize> {
        let mut connected = Vec::new();
        for edge in &self.edges {
            if edge.from == node_id {
                connected.push(edge.to);
            }
            if edge.to == node_id {
                connected.push(edge.from);
            }
        }
        connected
    }

    fn get_edges_for_node(&self, node_id: usize) -> Vec<(usize, usize, usize)> {
        // Returns (edge_index, from_id, to_id)
        let mut edges = Vec::new();
        for (i, edge) in self.edges.iter().enumerate() {
            if edge.from == node_id || edge.to == node_id {
                edges.push((i, edge.from, edge.to));
            }
        }
        edges
    }

    fn get_node_label(&self, node_id: usize) -> String {
        self.nodes
            .get(&node_id)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn apply_transfer(&mut self, from_id: usize, to_id: usize) {
        // Check (from, to) direction
        if self.try_transfer_direction(from_id, to_id) {
            return;
        }
        // Check (to, from) direction
        self.try_transfer_direction(to_id, from_id);
    }

    fn calculate_influence(&self, neutral_id: usize) -> i32 {
        let neutral = match self.nodes.get(&neutral_id) {
            Some(n) => n,
            None => return 0,
        };

        if neutral.owner != Owner::Neutral {
            return 0;
        }

        // King Rating: +1 if gold increasing, -1 if decreasing, 0 if stable
        let king_rating = if self.player_gold > self.prev_gold {
            1
        } else if self.player_gold < self.prev_gold {
            -1
        } else {
            0
        };

        // Food Surplus: current food
        let food_surplus = self.player_food;

        // Happiness: simplified (use a base value for MVP)
        let happiness = 50; // Base happiness for MVP

        // Distance to nearest player settlement
        let mut min_distance = f32::MAX;
        for node in self.nodes.values() {
            if node.owner == Owner::Player && node.node_type == NodeType::Settlement {
                let dist = ((node.x - neutral.x).powi(2) + (node.y - neutral.y).powi(2)).sqrt();
                min_distance = min_distance.min(dist);
            }
        }

        // Distance decay: -1 per tile (capped at reasonable value)
        let distance_decay = (min_distance as i32).min(100);

        // Calculate influence
        let influence = (king_rating * 10) + (food_surplus * 5) + (happiness * 2) - distance_decay;
        influence.max(0).min(100)
    }

    fn apply_influence_effects(&mut self) {
        let mut petitions = Vec::new();
        let mut happiness_boosts = Vec::new();

        for (id, node) in &self.nodes {
            if node.owner != Owner::Neutral {
                continue;
            }

            let influence = self.calculate_influence(*id);

            // Effect: 21-40: +10 happiness to nearby neutrals
            if influence >= 21 && influence <= 40 {
                happiness_boosts.push(*id);
            }

            // Effect: 41-60: Neutrals may petition to join (10% chance per tick)
            if influence >= 41 && influence <= 60 {
                let mut rng = rand::thread_rng();
                if rng.gen_bool(0.1) {
                    petitions.push(*id);
                }
            }
        }

        // Apply happiness boosts (would need happiness field - simplified for MVP)
        // For now, just log

        // Apply petitions: neutral settlement joins player
        for id in petitions {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.owner = Owner::Player;
                node.label = "Yours".to_string();
            }
        }
    }

    fn save_game(&self, slot: usize) -> Result<(), Box<dyn std::error::Error>> {
        let save_dir = std::path::Path::new("saves");
        if !save_dir.exists() {
            std::fs::create_dir_all(save_dir)?;
        }

        let filename = if slot == 0 {
            "saves/autosave.json".to_string()
        } else {
            format!("saves/save_slot_{}.json", slot)
        };

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(filename, json)?;
        Ok(())
    }

    fn load_game(slot: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let filename = if slot == 0 {
            "saves/autosave.json".to_string()
        } else {
            format!("saves/save_slot_{}.json", slot)
        };

        let data = std::fs::read_to_string(filename)?;
        let state: GameState = serde_json::from_str(&data)?;
        Ok(state)
    }

    fn try_transfer_direction(&mut self, producer_id: usize, consumer_id: usize) -> bool {
        let (producer, consumer) = match (self.nodes.get(&producer_id), self.nodes.get(&consumer_id)) {
            (Some(p), Some(c)) => (p, c),
            _ => return false,
        };

        // Determine resource type and check if transfer is valid
        let (resource_type, amount) = match (producer.node_type, consumer.node_type) {
            (NodeType::Farm, NodeType::Settlement) => ("food", 1),
            (NodeType::Mine | NodeType::CopperMine | NodeType::IronMine, NodeType::Factory | NodeType::CopperFactory | NodeType::IronFactory) => ("ore", 1),
            (NodeType::Woodcamp, NodeType::PlankIndustry) => ("firewood", 1),
            (NodeType::PlankIndustry, NodeType::Settlement) => ("planks", 1),
            (NodeType::Factory | NodeType::CopperFactory | NodeType::IronFactory, NodeType::Settlement) => ("goods", 1),
            _ => return false,
        };

        // Check if producer has enough resource
        let has_enough = match resource_type {
            "food" => producer.food >= amount,
            "ore" => producer.ore >= amount,
            "firewood" => producer.firewood >= amount,
            "planks" => producer.planks >= amount,
            "goods" => producer.goods >= amount,
            _ => false,
        };

        if !has_enough {
            return false;
        }

        // Apply transfer
        if let Some(producer) = self.nodes.get_mut(&producer_id) {
            match resource_type {
                "food" => producer.food -= amount,
                "ore" => producer.ore -= amount,
                "firewood" => producer.firewood -= amount,
                "planks" => producer.planks -= amount,
                "goods" => producer.goods -= amount,
                _ => {}
            }
        }

        if let Some(consumer) = self.nodes.get_mut(&consumer_id) {
            match resource_type {
                "food" => consumer.food += amount,
                "ore" => consumer.ore += amount,
                "firewood" => consumer.firewood += amount,
                "planks" => consumer.planks += amount,
                "goods" => consumer.goods += amount,
                _ => {}
            }
        }

        true
    }

    fn tick(&mut self) {
        if self.game_over {
            return;
        }
        self.tick_count += 1;

        // Save current gold for king rating (will become prev_gold after tick)
        let gold_at_start = self.player_gold;

        let mut linked_ids: Vec<usize> = Vec::new();
        for edge in &self.edges {
            if let Some(n) = self.nodes.get(&edge.to) {
                if n.owner == Owner::Player {
                    linked_ids.push(edge.from);
                }
            }
            if let Some(n) = self.nodes.get(&edge.from) {
                if n.owner == Owner::Player {
                    linked_ids.push(edge.to);
                }
            }
        }
        let linked_set: std::collections::HashSet<usize> = linked_ids.into_iter().collect();

        for node in self.nodes.values_mut() {
            let linked = linked_set.contains(&node.id);
            let can_produce =
                node.owner == Owner::Player || (node.owner == Owner::Neutral && linked);

            if can_produce {
                match node.node_type {
                    NodeType::Settlement => {
                        if node.owner == Owner::Player {
                            // Gold: 0 for camp, 1/s for village, increases by 1 each tier
                            let gold_per_sec = if node.population >= 20000 {
                                4
                            } else if node.population >= 6000 {
                                3
                            } else if node.population >= 1000 {
                                2
                            } else if node.population >= 100 {
                                1
                            } else {
                                0
                            };
                            // Only produce gold every GOLD_TICK_INTERVAL
                            if self.tick_count % GOLD_TICK_INTERVAL == 0 {
                                node.gold += gold_per_sec;
                            }
                            // Food production per tick
                            node.food += node.population;
                        }
                    }
                    NodeType::Mine | NodeType::CopperMine => {
                        if linked {
                            node.ore += 1;
                            node.ore += node.count_miners() * MINER_PRODUCTION;
                        }
                    }
                    NodeType::IronMine => {
                        if linked {
                            node.ore += 1;
                            node.ore += node.count_miners() * MINER_PRODUCTION;
                        }
                    }
                    NodeType::Factory | NodeType::CopperFactory | NodeType::IronFactory => {
                        if linked && node.ore >= 1 {
                            node.goods += 1;
                            node.ore -= 1;
                        }
                        if linked {
                            node.goods += node.count_manufacturers() * MFGR_PRODUCTION;
                        }
                    }
                    NodeType::Farm => {
                        if linked {
                            node.food += 1;
                            node.food += node.count_farmers() * FARMER_PRODUCTION;
                        }
                    }
                    NodeType::Woodcamp => {
                        if linked {
                            node.firewood += 1;
                            node.firewood += node.count_lumberjacks() * 2;
                        }
                    }
                    NodeType::PlankIndustry => {
                        if linked && node.firewood >= 1 {
                            node.planks += 1;
                            node.firewood -= 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut total_gold = 0;
        let mut total_food = 0;
        let mut total_ore = 0;
        let mut total_goods = 0;

        for node in self.nodes.values() {
            if node.owner == Owner::Player {
                total_gold += node.gold;
                total_food += node.food;
                total_ore += node.ore;
                total_goods += node.goods;
            } else if node.owner == Owner::Neutral {
                if linked_set.contains(&node.id) {
                    total_gold += node.gold;
                    total_food += node.food;
                    total_ore += node.ore;
                    total_goods += node.goods;
                }
            }
        }

        total_gold += total_goods * 10;
        total_goods = 0;

        // Process worker training - increment skill each tick
        for node in self.nodes.values_mut() {
            for worker in &mut node.workers {
                if worker.training_tier > 0 && worker.skill < worker.training_target_skill {
                    worker.skill += 1;
                }
                // Clear training tier when complete
                if worker.training_tier > 0 && worker.skill >= worker.training_target_skill {
                    worker.training_tier = 0;
                }
            }
        }

        // Process Resource edge transfers
        let mut transfers = Vec::new();

        // First pass: determine which edges need transfer
        for (idx, edge) in self.edges.iter_mut().enumerate() {
            if edge.edge_type != EdgeType::Resource {
                continue;
            }
            if edge.caravans == 0 {
                continue;
            }

            edge.ticks_accumulated += 1;

            // Get nodes to calculate distance
            let (from_node, to_node) = match (self.nodes.get(&edge.from), self.nodes.get(&edge.to)) {
                (Some(f), Some(t)) => (f, t),
                _ => continue,
            };

            let distance = ((from_node.x - to_node.x).powi(2) + (from_node.y - to_node.y).powi(2)).sqrt() as u64;
            let required = ((distance * 2) as f32 / edge.caravans as f32).max(1.0) as u64;

            if edge.ticks_accumulated >= required {
                transfers.push((idx, edge.from, edge.to));
            }
        }

        // Second pass: apply transfers
        for (idx, from_id, to_id) in transfers {
            // Reset ticks for this edge
            if let Some(edge) = self.edges.get_mut(idx) {
                edge.ticks_accumulated = 0;
            }
            self.apply_transfer(from_id, to_id);
        }

        // Process Supply edges (Settlement -> Fort population transfer)
        for edge in &self.edges {
            if edge.edge_type != EdgeType::Supply {
                continue;
            }

            // Determine which is Settlement and which is Fort
            let (settlement_id, fort_id) = match (self.nodes.get(&edge.from), self.nodes.get(&edge.to)) {
                (Some(f), Some(t)) => {
                    if f.node_type == NodeType::Settlement && t.node_type == NodeType::Fort {
                        (edge.from, edge.to)
                    } else if f.node_type == NodeType::Fort && t.node_type == NodeType::Settlement {
                        (edge.to, edge.from)
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            // Transfer 1 population per tick from Settlement to Fort
            let settlement_pop = self.nodes.get(&settlement_id).map(|n| n.population).unwrap_or(0);
            if settlement_pop > 0 {
                if let Some(settlement) = self.nodes.get_mut(&settlement_id) {
                    settlement.population -= 1;
                }
                if let Some(fort) = self.nodes.get_mut(&fort_id) {
                    fort.population += 1;
                }
            }
        }

        // Process Combat edges (Fort <-> Fort auto-combat)
        // Collect combat edges first to avoid borrow issues
        let combat_edges: Vec<(usize, usize)> = self.edges.iter()
            .filter(|e| e.edge_type == EdgeType::Combat)
            .map(|e| (e.from, e.to))
            .collect();

        for (fort1_id, fort2_id) in combat_edges {
            // Get data immutably first
            let (pop1, workers1, owner1) = match self.nodes.get(&fort1_id) {
                Some(f) => (f.population, f.workers.len(), f.owner),
                None => continue,
            };
            let (pop2, workers2, owner2) = match self.nodes.get(&fort2_id) {
                Some(f) => (f.population, f.workers.len(), f.owner),
                None => continue,
            };

            if pop1 <= 0 || pop2 <= 0 {
                continue;
            }

            // Simplified combat power: 1.0 + 0.05 per worker
            let power1 = 1.0 + (workers1 as f32) * 0.05;
            let power2 = 1.0 + (workers2 as f32) * 0.05;

            // Damage: 1 per 2 combatants, modified by power
            let damage_to_1 = (pop2 as f32 / 2.0 * power2).ceil() as i32;
            let damage_to_2 = (pop1 as f32 / 2.0 * power1).ceil() as i32;

            // Apply damage
            if let Some(fort1) = self.nodes.get_mut(&fort1_id) {
                fort1.population -= damage_to_2.min(fort1.population);
                if fort1.population <= 0 {
                    fort1.population = 0;
                    // Fort is destroyed - change owner to attacker (fort2's owner)
                    fort1.owner = owner2;
                    fort1.population = 1; // Reinforce with 1 pop
                }
            }
            if let Some(fort2) = self.nodes.get_mut(&fort2_id) {
                fort2.population -= damage_to_1.min(fort2.population);
                if fort2.population <= 0 {
                    fort2.population = 0;
                    // Fort is destroyed - change owner to attacker (fort1's owner)
                    fort2.owner = owner1;
                    fort2.population = 1; // Reinforce with 1 pop
                }
            }
        }

        self.player_gold = total_gold;
        self.player_food = total_food;
        self.player_ore = total_ore;
        self.player_goods = total_goods;

        // Update prev_gold for king rating calculation (use gold at start of tick)
        self.prev_gold = gold_at_start;

        // Apply influence effects
        self.apply_influence_effects();

        // Auto-save every 60 ticks
        if self.tick_count % 60 == 0 && self.tick_count > 0 {
            let _ = self.save_game(0); // Auto-save to slot 0
        }

        if self.player_gold >= WIN_THRESHOLD {
            self.game_over = true;
            self.victory = Some(true);
        }

        let player_settlements = self
            .nodes
            .values()
            .filter(|n| n.owner == Owner::Player && n.node_type == NodeType::Settlement)
            .count();
        if player_settlements == 0 {
            self.game_over = true;
            self.victory = Some(false);
        }
    }
}

struct NodeEmpireApp {
    game: GameState,
}

impl NodeEmpireApp {
    fn new() -> Self {
        Self {
            game: GameState::new(),
        }
    }
}

impl eframe::App for NodeEmpireApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.game.tick();

        // Handle zoom with scroll wheel
        let scroll = ctx.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let zoom_change = -scroll * ZOOM_SENSITIVITY;
            self.game.camera_zoom = (self.game.camera_zoom + zoom_change).clamp(ZOOM_MIN, ZOOM_MAX);
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Coins: {}", self.game.player_gold));
                ui.label(format!("Food: {}", self.game.player_food));
                ui.label(format!("Ore: {}", self.game.player_ore));
                ui.label(format!("Goods: {}", self.game.player_goods));
                ui.separator();
                ui.label(format!("Tick: {}", self.game.tick_count));
                ui.separator();
                if ui.button("[?]").clicked() {
                    self.game.show_help = !self.game.show_help;
                }
            });
        });

        egui::SidePanel::left("left").show(ctx, |ui| {
            // Disconnect button
            let has_connections = {
                if let Some(sel) = self.game.selected_node {
                    !self.game.get_connected_nodes(sel).is_empty()
                } else {
                    false
                }
            };

            if has_connections {
                ui.label("Connections");
                if ui.button("Disconnect All").clicked() {
                    if let Some(sel) = self.game.selected_node {
                        self.game.edges.retain(|e| e.from != sel && e.to != sel);
                    }
                }
                ui.separator();

// Show individual edges to delete and manage
if let Some(sel) = self.game.selected_node {
    let edges = self.game.get_edges_for_node(sel);
    if !edges.is_empty() {
        ui.label("Connections:");
        for (edge_idx, from_id, to_id) in edges {
            let other = if from_id == sel { to_id } else { from_id };
            let label = self.game.get_node_label(other);

            // Get edge data (extract needed values to avoid borrow issues)
            let (edge_type, caravans, ticks_accumulated, from_id, to_id) = 
                if let Some(edge) = self.game.edges.get(edge_idx) {
                    (edge.edge_type, edge.caravans, edge.ticks_accumulated, edge.from, edge.to)
                } else {
                    continue;
                };
            
            let edge_type_str = match edge_type {
                EdgeType::Resource => "Resource",
                EdgeType::Supply => "Supply",
                EdgeType::Combat => "Combat",
            };
            
            ui.horizontal(|ui| {
                ui.label(format!("→ {} [{}]", label, edge_type_str));
                
                // Caravan controls for Resource edges
                if edge_type == EdgeType::Resource {
                    ui.label(format!("Caravans: {}", caravans));
                    if ui.button("-").clicked() && caravans > 0 {
                        if let Some(e) = self.game.edges.get_mut(edge_idx) {
                            e.caravans -= 1;
                        }
                    }
                    if ui.button("+").clicked() {
                        if let Some(e) = self.game.edges.get_mut(edge_idx) {
                            e.caravans += 1;
                        }
                    }
                }
                
                // Delete button
                if ui.button("X").clicked() {
                    self.game.edges.remove(edge_idx);
                }
            });
            
            // Show transfer progress for Resource edges
            if edge_type == EdgeType::Resource && caravans > 0 {
                if let (Some(from), Some(to)) = (
                    self.game.nodes.get(&from_id),
                    self.game.nodes.get(&to_id),
                ) {
                    let distance = ((from.x - to.x).powi(2) + (from.y - to.y).powi(2)).sqrt() as u64;
                    let required = ((distance * 2) as f32 / caravans as f32).max(1.0) as u64;
                    ui.label(format!("  Progress: {}/{} ticks", ticks_accumulated, required));
                }
            }
        }
    }
}
                ui.separator();
            }

            ui.label("Build");
            if ui.button("Village (50g)").clicked() {
                if self.game.player_gold >= 50 {
                    if let Some(sel) = self.game.selected_node {
                        if let Some(node) = self.game.nodes.get(&sel) {
                            if node.owner == Owner::Player {
                                let new_id = self.game.nodes.len() + 1;
                                let mut n = GameNode::new(
                                    new_id,
                                    NodeType::Settlement,
                                    Owner::Player,
                                    node.x + 2.0,
                                    node.y,
                                );
                                n.population = 5;
                                self.game.nodes.insert(new_id, n);
                                self.game.player_gold -= 50;
                            }
                        }
                    }
                }
            }

            ui.separator();
            ui.label("Train Worker");
            ui.label("Cost: gold only");

            // First check what node is selected
            let selected_id = self.game.selected_node;
            let selected_type =
                selected_id.and_then(|id| self.game.nodes.get(&id).map(|n| n.node_type));
            let is_production = matches!(
                selected_type,
                Some(NodeType::Farm)
                    | Some(NodeType::Mine)
                    | Some(NodeType::CopperMine)
                    | Some(NodeType::IronMine)
                    | Some(NodeType::Woodcamp)
                    | Some(NodeType::Factory)
                    | Some(NodeType::CopperFactory)
                    | Some(NodeType::IronFactory)
                    | Some(NodeType::PlankIndustry)
            );

            if is_production && selected_id.is_some() {
                let sel = selected_id.unwrap();
                ui.separator();
                ui.label("Select tier:");
                let gold = self.game.player_gold;

                if gold >= TRAIN_BASIC_COST && ui.button("Basic (10g, 50 ticks)").clicked() {
                    self.game.train_worker(sel, 1, 10);
                }
                if gold >= TRAIN_SPECIALIST_COST
                    && ui.button("Specialist (40g, 200 ticks)").clicked()
                {
                    self.game.train_worker(sel, 2, 40);
                }
                if gold >= TRAIN_EXPERT_COST && ui.button("Expert (80g, 400 ticks)").clicked() {
                    self.game.train_worker(sel, 3, 80);
                }

                // Show workers - get fresh reference
                if let Some(n) = self.game.nodes.get(&sel) {
                    if !n.workers.is_empty() {
                        ui.separator();
                        ui.label(format!("Workers: {}", n.workers.len()));
                        for (i, w) in n.workers.iter().enumerate() {
                            let status = if w.training_tier > 0 {
                                " (training)"
                            } else {
                                ""
                            };
                            ui.label(format!("{}. Skill: {}{}", i + 1, w.skill, status));
                        }
                    }
                }
            }
        });

        egui::SidePanel::right("right").show(ctx, |ui| {
            ui.label("Your Settlements");
            for (_, n) in
                self.game.nodes.iter().filter(|(_, n)| {
                    n.owner == Owner::Player && n.node_type == NodeType::Settlement
                })
            {
                ui.label(format!(
                    "{} pop:{} workers:{}",
                    n.label,
                    n.population,
                    n.workers.len()
                ));
            }

            ui.separator();
            ui.label("Your Buildings");

            for (_, n) in self.game.nodes.iter().filter(|(_, n)| {
                n.owner == Owner::Player
                    && matches!(
                        n.node_type,
                        NodeType::Mine | NodeType::CopperMine | NodeType::IronMine
                    )
            }) {
                ui.label(format!("{} miners:{}", n.label, n.count_miners()));
            }
            for (_, n) in self.game.nodes.iter().filter(|(_, n)| {
                n.owner == Owner::Player
                    && matches!(
                        n.node_type,
                        NodeType::Factory | NodeType::CopperFactory | NodeType::IronFactory
                    )
            }) {
                ui.label(format!("{} mfrs:{}", n.label, n.count_manufacturers()));
            }
            for (_, n) in self
                .game
                .nodes
                .iter()
                .filter(|(_, n)| n.owner == Owner::Player && n.node_type == NodeType::Farm)
            {
                ui.label(format!("{} farmers:{}", n.label, n.workers.len()));
            }
            for (_, n) in self
                .game
                .nodes
                .iter()
                .filter(|(_, n)| n.owner == Owner::Player && n.node_type == NodeType::Woodcamp)
            {
                ui.label(format!("{} lumberjacks:{}", n.label, n.count_lumberjacks()));
            }

            ui.separator();
            ui.label("Save/Load");
            ui.horizontal(|ui| {
                if ui.button("Save 1").clicked() {
                    let _ = self.game.save_game(1);
                }
                if ui.button("Load 1").clicked() {
                    if let Ok(state) = GameState::load_game(1) {
                        self.game = state;
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Save 2").clicked() {
                    let _ = self.game.save_game(2);
                }
                if ui.button("Load 2").clicked() {
                    if let Ok(state) = GameState::load_game(2) {
                        self.game = state;
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Save 3").clicked() {
                    let _ = self.game.save_game(3);
                }
                if ui.button("Load 3").clicked() {
                    if let Ok(state) = GameState::load_game(3) {
                        self.game = state;
                    }
                }
            });

            if let Some(v) = self.game.victory {
                ui.label(if v { "Victory!" } else { "Defeat!" });
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let painter = ui.painter();

            fn is_linked_to_player(
                node_id: usize,
                edges: &[EdgeData],
                nodes: &HashMap<usize, GameNode>,
            ) -> bool {
                for edge in edges {
                    if edge.from == node_id || edge.to == node_id {
                        let other = if edge.from == node_id {
                            edge.to
                        } else {
                            edge.from
                        };
                        if let Some(n) = nodes.get(&other) {
                            if n.owner == Owner::Player {
                                return true;
                            }
                        }
                    }
                }
                false
            }

            // Zoom factor
            let zoom = self.game.camera_zoom;

            for (edge_idx, edge) in self.game.edges.iter().enumerate() {
                if let (Some(from), Some(to)) = (
                    self.game.nodes.get(&edge.from),
                    self.game.nodes.get(&edge.to),
                ) {
                    let from_pos = egui::pos2(
                        from.x * 20.0 * zoom + self.game.camera_x,
                        from.y * 20.0 * zoom + self.game.camera_y,
                    );
                    let to_pos = egui::pos2(
                        to.x * 20.0 * zoom + self.game.camera_x,
                        to.y * 20.0 * zoom + self.game.camera_y,
                    );

                    // Color based on edge type
                    let color = match edge.edge_type {
                        EdgeType::Resource => egui::Color32::from_rgb(100, 255, 100), // Green
                        EdgeType::Supply => egui::Color32::from_rgb(100, 100, 255),    // Blue
                        EdgeType::Combat => egui::Color32::from_rgb(255, 100, 100),    // Red
                    };

                    let stroke = egui::Stroke::new(2.0, color);

                    painter.line_segment([from_pos, to_pos], stroke);

                    // Highlight selected edge
                    if self.game.selected_edge == Some(edge_idx) {
                        painter.line_segment(
                            [from_pos, to_pos],
                            egui::Stroke::new(4.0, egui::Color32::YELLOW),
                        );
                    }
                }
            }

            // Drag line when connecting
            if let Some(from_id) = self.game.connecting_from {
                if let Some(from) = self.game.nodes.get(&from_id) {
                    let pos = egui::pos2(
                        from.x * 20.0 * zoom + self.game.camera_x,
                        from.y * 20.0 * zoom + self.game.camera_y,
                    );
                    painter.circle_stroke(
                        pos,
                        20.0 * zoom,
                        egui::Stroke::new(3.0, egui::Color32::YELLOW),
                    );
                }
            }

            for node in self.game.nodes.values() {
                let pos = egui::pos2(
                    node.x * 20.0 * zoom + self.game.camera_x,
                    node.y * 20.0 * zoom + self.game.camera_y,
                );
                let linked = is_linked_to_player(node.id, &self.game.edges, &self.game.nodes);

                let color = if node.owner == Owner::Player || linked {
                    egui::Color32::from_rgb(100, 200, 100)
                } else if node.owner == Owner::Enemy {
                    egui::Color32::from_rgb(200, 100, 100)
                } else {
                    egui::Color32::from_rgb(180, 180, 180)
                };

                match node.node_type {
                    NodeType::Mine | NodeType::CopperMine => {
                        painter.circle_filled(pos, 10.0, egui::Color32::from_rgb(205, 133, 63));
                        painter.circle_stroke(
                            pos,
                            12.0,
                            egui::Stroke::new(2.0, egui::Color32::GRAY),
                        );
                    }
                    NodeType::IronMine => {
                        painter.circle_filled(pos, 10.0, egui::Color32::from_rgb(169, 169, 169));
                        painter.circle_stroke(
                            pos,
                            12.0,
                            egui::Stroke::new(2.0, egui::Color32::DARK_GRAY),
                        );
                    }
                    NodeType::Woodcamp => {
                        let rect = egui::Rect::from_center_size(pos, egui::vec2(18.0, 14.0));
                        painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(34, 139, 34));
                    }
                    NodeType::Farm => {
                        let rect = egui::Rect::from_center_size(pos, egui::vec2(18.0, 18.0));
                        painter.rect_filled(rect, 2.0, color);
                    }
                    NodeType::Factory | NodeType::CopperFactory | NodeType::IronFactory => {
                        painter.circle_filled(pos, 14.0, color);
                        painter.circle_stroke(
                            pos,
                            16.0,
                            egui::Stroke::new(2.0, egui::Color32::DARK_GRAY),
                        );
                    }
                    NodeType::PlankIndustry => {
                        let rect =
                            egui::Rect::from_center_size(pos, egui::vec2(16.0 * zoom, 16.0 * zoom));
                        painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(139, 69, 19));
                    }
                    NodeType::Market => {
                        let s = 12.0 * zoom;
                        let p1 = pos + egui::vec2(0.0, -s);
                        let p2 = pos + egui::vec2(s, 0.0);
                        let p3 = pos + egui::vec2(0.0, s);
                        let p4 = pos + egui::vec2(-s, 0.0);
                        let stroke = egui::Stroke::new(2.0, color);
                        painter.line_segment([p1, p2], stroke);
                        painter.line_segment([p2, p3], stroke);
                        painter.line_segment([p3, p4], stroke);
                        painter.line_segment([p4, p1], stroke);
                    }
                    _ => {
                        painter.circle_filled(pos, 12.0 * zoom, color);
                    }
                }

                if self.game.selected_node == Some(node.id)
                    || self.game.connecting_from == Some(node.id)
                {
                    painter.circle_stroke(
                        pos,
                        18.0 * zoom,
                        egui::Stroke::new(2.0, egui::Color32::YELLOW),
                    );
                }

                painter.text(
                    pos + egui::vec2(0.0, -22.0 * zoom),
                    egui::Align2::CENTER_CENTER,
                    &node.label,
                    egui::FontId::proportional(12.0 * zoom),
                    color,
                );
            }

            let rect = ui.min_rect();
            let resp = ui.interact(rect, "map".into(), egui::Sense::click_and_drag());

            // Only pan if actually dragged (with threshold)
            if resp.dragged() {
                let drag_dist = resp.drag_delta().x.abs() + resp.drag_delta().y.abs();
                if drag_dist > 8.0 {
                    self.game.camera_x += resp.drag_delta().x;
                    self.game.camera_y += resp.drag_delta().y;
                }
            }

            // Handle clicks (with zoom)
            if resp.clicked() {
                if let Some(p) = resp.interact_pointer_pos() {
                    let mx = (p.x - self.game.camera_x) / 20.0;
                    let my = (p.y - self.game.camera_y) / 20.0;

                    let mut clicked_node: Option<usize> = None;
                    for (id, node) in &self.game.nodes {
                        let dx = (node.x - mx as f32).abs();
                        let dy = (node.y - my as f32).abs();
                        if dx < 1.0 && dy < 1.0 {
                            clicked_node = Some(*id);
                            break;
                        }
                    }

                    if let Some(clicked_id) = clicked_node {
                        if let Some(from_id) = self.game.connecting_from {
                            if from_id != clicked_id {
if let (Some(from), Some(to)) = (
    self.game.nodes.get(&from_id),
    self.game.nodes.get(&clicked_id),
) {
    let cost = connection_cost(from, to);
    if self.game.player_gold >= cost {
        self.game.player_gold -= cost;

        // Determine edge type based on connected node types
        let edge_type = match (from.node_type, to.node_type) {
            (NodeType::Settlement, NodeType::Fort) | (NodeType::Fort, NodeType::Settlement) => EdgeType::Supply,
            (NodeType::Fort, NodeType::Fort) => EdgeType::Combat,
            _ => EdgeType::Resource,
        };

        // Set caravans: 1 for Resource edges, 0 for others
        let caravans = if edge_type == EdgeType::Resource { 1 } else { 0 };

        self.game.edges.push(EdgeData {
            from: from_id,
            to: clicked_id,
            edge_type,
            caravans,
            ticks_accumulated: 0,
        });
    }
}
                            }
                            self.game.connecting_from = None;
                        } else {
                            self.game.connecting_from = Some(clicked_id);
                        }
                        self.game.selected_node = Some(clicked_id);
                    } else {
                        self.game.connecting_from = None;
                    }
                }
            }
        });

        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            if let Some(sel) = self.game.selected_node {
                if let Some(n) = self.game.nodes.get(&sel) {
                    if let Some(from_id) = self.game.connecting_from {
                        if from_id != sel {
                            if let Some(from) = self.game.nodes.get(&from_id) {
                                let cost = connection_cost(from, n);
                                ui.label(format!(
                                    "Selected: {} ({:?}) - Connect cost: {}g",
                                    n.label, n.node_type, cost
                                ));
                            }
                        } else {
                            ui.label(format!(
                                "Selected: {} ({:?}) - Click another node to connect",
                                n.label, n.node_type
                            ));
                        }
                    } else {
                        // Show fort-specific info
                        if n.node_type == NodeType::Fort {
                            ui.label(format!(
                                "Selected: {} ({:?}) Pop:{} Workers:{}",
                                n.label, n.node_type, n.population, n.workers.len()
                            ));
                        } else {
                            ui.label(format!(
                                "Selected: {} ({:?}) Pop:{} Gold:{} Food:{}",
                                n.label, n.node_type, n.population, n.gold, n.food
                            ));
                        }

                        // Show influence for neutral settlements
                        if n.owner == Owner::Neutral {
                            let influence = self.game.calculate_influence(sel);
                            ui.label(format!("Influence: {}/100", influence));

                            // Incite Rebellion button (61-80 influence, costs 100g)
                            if influence >= 61 && influence <= 80 {
                                if ui.button("Incite Rebellion (100g)").clicked() {
                                    if self.game.player_gold >= 100 {
                                        self.game.player_gold -= 100;
                                        // Make the settlement rebel (change to player)
                                        if let Some(node) = self.game.nodes.get_mut(&sel) {
                                            node.owner = Owner::Player;
                                            node.label = "Yours".to_string();
                                        }
                                    }
                                }
                            }

                            // Buy Settlement button (81-100 influence, costs 200g)
                            if influence >= 81 {
                                if ui.button("Buy Settlement (200g)").clicked() {
                                    if self.game.player_gold >= 200 {
                                        self.game.player_gold -= 200;
                                        // Buy the settlement
                                        if let Some(node) = self.game.nodes.get_mut(&sel) {
                                            node.owner = Owner::Player;
                                            node.label = "Yours".to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                ui.label("Click a node to select");
            }
        });

        if self.game.show_help {
            egui::Window::new("Help").open(&mut true).show(ctx, |ui| {
                ui.label("How to Play");
                ui.label("");
                ui.label("Select Your village");
                ui.label("Click TRAIN to create workers");
                ui.label("  Miner - works at Mine (+2 ore)");
                ui.label("  Manufacturer - works at Factory (+2 goods)");
                ui.label("  Farmer - works at Farm (+3 food)");
                ui.label("  Lumberjack - works at Woodcamp (+2)");
                ui.label("Click then another node to connect");
                ui.label("Connecting costs gold based on distance");
                ui.label("Reach 5000 gold to WIN!");
                ui.label("");
                ui.label("Node Types:");
                ui.label("  Cu = Copper Mine");
                ui.label("  Fe = Iron Mine");
                ui.label("  Wood = Woodcamp");
                ui.label("");
                ui.label("Influence System:");
                ui.label("  Click neutral settlements to see influence");
                ui.label("  High influence (61+) can incite rebellion (100g)");
                ui.label("  Very high (81+) can buy settlements (200g)");
                ui.label("  Influence grows with gold, food, happiness");
                ui.label("");
                if ui.button("Close").clicked() {
                    self.game.show_help = false;
                }
            });
        }
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    eframe::run_native(
        "Node Empire",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(NodeEmpireApp::new()))),
    )
}
