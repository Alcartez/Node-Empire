//! Test suite: Racial demographics, food consumption, and growth rate dynamics.

mod common;
use common::ScenarioHarness;
use node_empire::game::types::*;

#[test]
fn test_reptilian_food_conservation_vs_hive_swarm_consumption() {
    let mut harness = ScenarioHarness::new();

    // Reptilian settlement: 100 population, 1000 food
    let rep_id = harness.add_settlement("Coldscale", Owner::Player, Some(Faction::Mez), Race::Reptilian, 100, 1000);

    // Hive Swarm settlement: 100 population, 1000 food
    let hive_id = harness.add_settlement("Broodnest", Owner::Player, Some(Faction::HiveSwarms), Race::HiveSwarm, 100, 1000);

    // Step 100 ticks (20 consumption cycles)
    harness.step(100);

    let rep_food = harness.world.node(rep_id).unwrap().stock.food;
    let hive_food = harness.world.node(hive_id).unwrap().stock.food;

    assert!(
        rep_food > hive_food,
        "Reptilians with cold-blooded efficiency (0.75x) must have consumed significantly less food than voracious Hive Swarm (1.30x) ({} vs {})",
        rep_food,
        hive_food
    );

    harness.assert_invariants();
}

#[test]
fn test_racial_combat_multipliers() {
    // Verify pure multiplier ratios match the design spec
    assert!(
        Race::Beetlefolk.combat_multiplier() > Race::Human.combat_multiplier(),
        "Beetlefolk should have higher combat multiplier than baseline Humans"
    );
    assert!(
        Race::Beetlefolk.defense_multiplier() > Race::Human.defense_multiplier(),
        "Beetlefolk with heavy carapace armor should have higher defense multiplier than Humans"
    );
    assert!(
        Race::HiveSwarm.growth_multiplier() > Race::Human.growth_multiplier(),
        "Hive Swarm should experience higher population growth roll bonus than Humans"
    );
}
