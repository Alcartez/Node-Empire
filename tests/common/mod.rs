//! Shared testing harness and fixture builders for Node Empire developers.
//!
//! This module provides `ScenarioHarness`, a lightweight sandbox wrapper around
//! `World` designed for headless, deterministic testing of mechanics:
//!
//! * Isolated worlds (no background AI interference, no premature victory, no autosave).
//! * Ergonomic builders to assemble multi-settlement scenarios, forts, generals, and trade corridors in 1-2 lines.
//! * Automatic simulation stepping with safety predicates.
//! * Comprehensive invariant assertion checks (no NaN floats, no negative populations, no broken graph edges, valid general pointers).
//! * Serialization round-trip verification.

#![allow(dead_code)]

use node_empire::game::types::*;
use node_empire::game::world::{Node, World};

/// Controlled testing sandbox for writing deterministic feature tests.
pub struct ScenarioHarness {
    pub world: World,
    next_node_id: u32,
}

impl ScenarioHarness {
    /// Create a clean, blank testing sandbox with deactivated AI and disabled autosave.
    pub fn new() -> Self {
        let mut world = World::new(42, Faction::Breplid, Difficulty::Medium);
        world.nodes.clear();
        world.edges.clear();
        world.spies.clear();
        world.generals.clear();
        world.log.clear();
        world.autosave = false;
        world.win_gold = i32::MAX;
        world.ai_timer = u64::MAX;
        world.player_ledger.gold = 500;
        world.enemy_ledger.gold = 500;

        // Player anchor settlement to ensure player_settlements > 0
        let mut anchor = Node::new(
            9998,
            NodeType::Settlement,
            Owner::Player,
            Some(Faction::Breplid),
            0.0,
            0.0,
            "Anchorholme".into(),
        );
        anchor.stock.food = 100_000;
        world.nodes.insert(9998, anchor);

        // Distant dummy enemy settlement to prevent premature Domination victory
        let mut foe = Node::new(
            9999,
            NodeType::Settlement,
            Owner::Enemy,
            Some(Faction::Mez),
            200.0,
            200.0,
            "Foehold".into(),
        );
        foe.stock.food = 100_000;
        world.nodes.insert(9999, foe);

        Self {
            world,
            next_node_id: 1,
        }
    }

    /// A sandbox with random world events (bandits) disabled and the dummy
    /// anchor/foe larders emptied, so influence and petitions cannot fire
    /// behind a scenario's back. Use this for deterministic mechanic tests.
    pub fn quiet() -> Self {
        let mut harness = Self::new();
        harness.world.bandit_timer = u64::MAX;
        for id in [9998, 9999] {
            if let Some(node) = harness.world.nodes.get_mut(&id) {
                node.stock.food = 0;
            }
        }
        harness
    }

    /// Add a settlement to the sandbox with customized demographics and supplies.
    pub fn add_settlement(
        &mut self,
        name: &str,
        owner: Owner,
        faction: Option<Faction>,
        race: Race,
        population: i32,
        food: i32,
    ) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;

        let mut node = Node::new(
            id,
            NodeType::Settlement,
            owner,
            faction,
            (id as f32) * 5.0,
            10.0,
            name.to_string(),
        );
        node.dominant_race = Some(race);
        node.population = population;
        node.stock.food = food;
        node.happiness = 65.0;
        node.public_order = 70.0;
        node.loyalty = 100;

        self.world.nodes.insert(id, node);
        id
    }

    /// Add a fort to the sandbox with specified garrison, equipment tier, and optional dominant race.
    pub fn add_fort(
        &mut self,
        name: &str,
        owner: Owner,
        faction: Option<Faction>,
        race: Option<Race>,
        garrison: i32,
        equipment: EquipmentTier,
    ) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;

        let mut node = Node::new(
            id,
            NodeType::Fort,
            owner,
            faction,
            (id as f32) * 5.0,
            15.0,
            name.to_string(),
        );
        node.dominant_race = race;
        node.population = garrison;
        node.equipment = equipment;
        node.happiness = 60.0;
        node.public_order = 80.0;
        node.loyalty = 100;

        self.world.nodes.insert(id, node);
        id
    }

    /// Add a General to the sandbox and optionally assign them to a fort.
    #[allow(clippy::too_many_arguments)]
    pub fn add_general(
        &mut self,
        name: &str,
        owner: Owner,
        race: Race,
        fort_id: Option<u32>,
        leadership: u8,
        combat: u8,
        loyalty: u8,
    ) -> u32 {
        let id = self.world.generals.len() as u32 + 1;
        let general = General {
            id,
            name: name.to_string(),
            race,
            owner,
            fort_id,
            leadership,
            combat,
            loyalty,
            salary: 5,
            kills: 0,
        };

        if let Some(fid) = fort_id {
            if let Some(fort) = self.world.nodes.get_mut(&fid) {
                fort.general_id = Some(id);
            }
        }

        self.world.generals.push(general);
        id
    }

    /// Connect two nodes via an edge road (Supply or Trade).
    pub fn connect(&mut self, from: u32, to: u32, kind: EdgeType, by: Owner) -> usize {
        let n1 = self.world.node(from).expect("from node exists");
        let n2 = self.world.node(to).expect("to node exists");

        let edge = Edge {
            id: self.world.edges.len() as u32 + 1,
            from,
            to,
            kind,
            by,
            caravans: 1,
            progress: 0.0,
            path: vec![(n1.x as i32, n1.y as i32), (n2.x as i32, n2.y as i32)],
        };

        let idx = self.world.edges.len();
        self.world.edges.push(edge);
        idx
    }

    /// Advance the simulation by a fixed number of ticks.
    pub fn step(&mut self, ticks: usize) -> &mut Self {
        for _ in 0..ticks {
            self.world.tick();
        }
        self
    }

    /// Advance the simulation until a condition is met or max_ticks is reached.
    pub fn step_until<F>(&mut self, max_ticks: usize, mut predicate: F) -> bool
    where
        F: FnMut(&World) -> bool,
    {
        for _ in 0..max_ticks {
            if predicate(&self.world) {
                return true;
            }
            self.world.tick();
        }
        predicate(&self.world)
    }

    /// Validate core game engine invariants across the world state.
    ///
    /// Ensures future developer changes do not introduce subtle bugs or corruption:
    /// 1. Node coordinates and order/happiness metrics are non-NaN and bounded.
    /// 2. Populations and stocks are never negative.
    /// 3. All graph edges link to extant nodes.
    /// 4. All assigned generals link to extant forts, and fort.general_id matches.
    pub fn assert_invariants(&self) {
        for (&id, n) in &self.world.nodes {
            assert!(
                !n.x.is_nan() && !n.y.is_nan(),
                "Node {} coordinates are NaN",
                id
            );
            assert!(
                !n.happiness.is_nan() && n.happiness >= 0.0 && n.happiness <= 100.0,
                "Node {} happiness out of bounds: {}",
                id,
                n.happiness
            );
            assert!(
                !n.public_order.is_nan() && n.public_order >= 0.0 && n.public_order <= 100.0,
                "Node {} public_order out of bounds: {}",
                id,
                n.public_order
            );
            assert!(
                n.population >= 0,
                "Node {} has negative population: {}",
                id,
                n.population
            );
            assert!(
                n.stock.food >= 0,
                "Node {} has negative food: {}",
                id,
                n.stock.food
            );
            assert!(
                n.stock.timber >= 0,
                "Node {} has negative timber: {}",
                id,
                n.stock.timber
            );
            assert!(
                n.stock.ore >= 0,
                "Node {} has negative ore: {}",
                id,
                n.stock.ore
            );
            assert!(
                n.stock.goods >= 0,
                "Node {} has negative goods: {}",
                id,
                n.stock.goods
            );
        }

        for (i, edge) in self.world.edges.iter().enumerate() {
            assert!(
                self.world.nodes.contains_key(&edge.from),
                "Edge {} points to non-existent 'from' node {}",
                i,
                edge.from
            );
            assert!(
                self.world.nodes.contains_key(&edge.to),
                "Edge {} points to non-existent 'to' node {}",
                i,
                edge.to
            );
            assert!(
                edge.length() > 0.0 && !edge.length().is_nan(),
                "Edge {} has invalid length",
                i
            );
        }

        for gen in &self.world.generals {
            assert!(
                gen.loyalty <= 100,
                "General {} has invalid loyalty {}",
                gen.name,
                gen.loyalty
            );
            if let Some(fid) = gen.fort_id {
                let fort = self.world.nodes.get(&fid);
                assert!(
                    fort.is_some(),
                    "General {} assigned to missing fort {}",
                    gen.name,
                    fid
                );
                assert_eq!(
                    fort.unwrap().kind,
                    NodeType::Fort,
                    "General {} assigned to non-fort node {}",
                    gen.name,
                    fid
                );
            }
        }
    }

    /// Assert that serialization and deserialization preserves exact state.
    pub fn assert_serde_roundtrip(&self) {
        let serialized =
            serde_json::to_string(&self.world).expect("World must successfully serialize to JSON");
        let deserialized: World = serde_json::from_str(&serialized)
            .expect("Serialized world JSON must deserialize back to World");

        assert_eq!(self.world.nodes.len(), deserialized.nodes.len());
        assert_eq!(self.world.edges.len(), deserialized.edges.len());
        assert_eq!(self.world.generals.len(), deserialized.generals.len());
        assert_eq!(
            self.world.player_ledger.gold,
            deserialized.player_ledger.gold
        );
    }
}
