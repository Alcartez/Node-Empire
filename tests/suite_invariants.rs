//! Test suite: Invariant guards, procedural stability, and serialization round-trips.

mod common;
use common::ScenarioHarness;
use node_empire::game::types::*;
use node_empire::game::world::World;

#[test]
fn test_procedural_generation_invariants_across_seeds() {
    let factions = [
        Faction::Breplid,
        Faction::Hapset,
        Faction::Argo,
        Faction::SilkSpinners,
        Faction::Mez,
        Faction::HiveSwarms,
    ];

    for seed in 1..=30 {
        let faction = factions[(seed as usize) % factions.len()];
        let world = World::new(seed, faction, Difficulty::Medium);

        // Assert starting nodes contain starter capital and fort
        let player_settlement = world
            .nodes
            .values()
            .find(|n| n.is_settlement() && n.owner == Owner::Player);
        assert!(
            player_settlement.is_some(),
            "Seed {} missing player settlement",
            seed
        );

        let player_fort = world
            .nodes
            .values()
            .find(|n| n.kind == NodeType::Fort && n.owner == Owner::Player);
        assert!(player_fort.is_some(), "Seed {} missing player fort", seed);

        let player_gen = world.generals.iter().find(|g| g.owner == Owner::Player);
        assert!(player_gen.is_some(), "Seed {} missing player general", seed);

        // Check node invariant sanity
        for n in world.nodes.values() {
            assert!(!n.x.is_nan() && !n.y.is_nan());
            assert!(!n.happiness.is_nan());
            assert!(!n.public_order.is_nan());
            assert!(n.population >= 0);
        }
    }
}

#[test]
fn test_save_load_roundtrip_with_new_features() {
    let mut harness = ScenarioHarness::new();
    let s_id = harness.add_settlement(
        "Capital",
        Owner::Player,
        Some(Faction::SilkSpinners),
        Race::SilkSpinner,
        120,
        5000,
    );
    let f_id = harness.add_fort(
        "Grand Fort",
        Owner::Player,
        Some(Faction::SilkSpinners),
        Some(Race::SilkSpinner),
        25,
        EquipmentTier::IronClad,
    );
    harness.connect(s_id, f_id, EdgeType::Supply, Owner::Player);
    harness.add_general(
        "Silk Marshal",
        Owner::Player,
        Race::SilkSpinner,
        Some(f_id),
        85,
        90,
        80,
    );

    // Step 50 ticks to accumulate state changes
    harness.step(50);

    // Verify invariants and serde round-trip
    harness.assert_invariants();
    harness.assert_serde_roundtrip();
}
