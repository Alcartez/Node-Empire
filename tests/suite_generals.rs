//! Test suite: Generals, Leadership, Upkeep, Bonuses, and Mutiny mechanics.

mod common;
use common::ScenarioHarness;
use node_empire::game::types::*;

#[test]
fn test_general_salary_deduction_and_upkeep() {
    let mut harness = ScenarioHarness::new();
    let f_id = harness.add_fort(
        "Border Fort",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        5,
        EquipmentTier::Militia,
    );
    let g_id = harness.add_general(
        "Lord Arthur",
        Owner::Player,
        Race::Human,
        Some(f_id),
        80,
        75,
        70,
    );

    // Initial loyalty is 70 (< 80)
    // Advance 100 ticks with well-funded treasury
    harness.world.player_ledger.gold = 500;
    harness.step(100);

    // When paid on time and loyalty < 80, loyalty drifts up by +1 (70 -> 71)
    let gen = harness
        .world
        .generals
        .iter()
        .find(|g| g.id == g_id)
        .unwrap();
    assert_eq!(gen.loyalty, 71);

    harness.assert_invariants();
}

#[test]
fn test_general_unpaid_loyalty_decay() {
    let mut harness = ScenarioHarness::new();
    let f_id = harness.add_fort(
        "Border Fort",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        5,
        EquipmentTier::Militia,
    );
    let g_id = harness.add_general(
        "Lord Arthur",
        Owner::Player,
        Race::Human,
        Some(f_id),
        80,
        75,
        50,
    );

    // Drain treasury to 0 on every tick so player is continuously bankrupt
    for _ in 0..100 {
        harness.world.player_ledger.gold = 0;
        harness.world.tick();
    }

    // Since general went unpaid, loyalty decayed by 6 (50 -> 44)
    let gen = harness
        .world
        .generals
        .iter()
        .find(|g| g.id == g_id)
        .unwrap();
    assert_eq!(gen.loyalty, 44);

    harness.assert_invariants();
}

#[test]
fn test_general_mutiny_rebellion() {
    let mut harness = ScenarioHarness::new();
    let s_id = harness.add_settlement(
        "Capital",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        50,
        1000,
    );
    let f_id = harness.add_fort(
        "Vulnerable Fort",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        8,
        EquipmentTier::Militia,
    );
    harness.connect(s_id, f_id, EdgeType::Supply, Owner::Player);

    // Loyalty at 5 (< 10 critical mutiny threshold)
    let g_id = harness.add_general(
        "Traitor Roger",
        Owner::Player,
        Race::Human,
        Some(f_id),
        70,
        70,
        5,
    );
    harness.world.player_ledger.gold = 0;

    // Advance through upkeep pass
    harness.step(100);

    // The fort and general rebel and flip to Bandit faction
    let fort = harness.world.node(f_id).unwrap();
    assert_eq!(fort.owner, Owner::Bandit);
    assert_eq!(fort.faction, None);

    let general = harness
        .world
        .generals
        .iter()
        .find(|g| g.id == g_id)
        .unwrap();
    assert_eq!(general.owner, Owner::Bandit);
    assert_eq!(general.loyalty, 100); // Re-set to full loyalty as an independent leader

    harness.assert_invariants();
}

#[test]
fn test_general_leadership_boosts_supplied_settlement_order() {
    // Harness A: Fort with a brilliant General (Leadership 90)
    let mut h_leader = ScenarioHarness::new();
    let s_a = h_leader.add_settlement(
        "Town A",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        50,
        5000,
    );
    let f_a = h_leader.add_fort(
        "Fort A",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        5,
        EquipmentTier::Militia,
    );
    h_leader.connect(s_a, f_a, EdgeType::Supply, Owner::Player);
    h_leader.add_general(
        "Great General",
        Owner::Player,
        Race::Human,
        Some(f_a),
        90,
        80,
        100,
    );

    // Harness B: Fort with an inexperienced General (Leadership 20)
    let mut h_poor = ScenarioHarness::new();
    let s_b = h_poor.add_settlement(
        "Town B",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        50,
        5000,
    );
    let f_b = h_poor.add_fort(
        "Fort B",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        5,
        EquipmentTier::Militia,
    );
    h_poor.connect(s_b, f_b, EdgeType::Supply, Owner::Player);
    h_poor.add_general(
        "Weak General",
        Owner::Player,
        Race::Human,
        Some(f_b),
        20,
        20,
        100,
    );

    // Step both scenarios
    h_leader.step(50);
    h_poor.step(50);

    let order_a = h_leader.world.node(s_a).unwrap().public_order;
    let order_b = h_poor.world.node(s_b).unwrap().public_order;

    assert!(
        order_a > order_b,
        "Higher leadership general must result in greater civic public order ({} vs {})",
        order_a,
        order_b
    );
}

#[test]
fn test_general_fort_defense_bonus_scales_with_leadership_and_loyalty() {
    let veteran = General::new(1, "Veteran".into(), Owner::Player, Race::Human, 80, 40, 80);
    let raw = General::new(2, "Raw".into(), Owner::Player, Race::Human, 20, 40, 80);
    let disloyal = General::new(3, "Disloyal".into(), Owner::Player, Race::Human, 80, 40, 10);

    assert!(
        veteran.fort_defense_multiplier() > raw.fort_defense_multiplier(),
        "leadership must raise the fort defense bonus"
    );
    assert!(
        veteran.fort_defense_multiplier() > disloyal.fort_defense_multiplier(),
        "loyalty must raise the fort defense bonus"
    );
    assert!(
        (veteran.fort_defense_multiplier() - 1.24).abs() < 1e-6,
        "80 leadership + 80 loyalty should give +24% defense"
    );
}

#[test]
fn test_general_loyalty_aura_steadies_supplied_town() {
    let mut with_general = ScenarioHarness::quiet();
    let town = with_general.add_settlement(
        "Steady Town",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        50,
        5000,
    );
    with_general.world.node_mut(town).unwrap().loyalty = 50;
    let fort = with_general.add_fort(
        "Steady Fort",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        5,
        EquipmentTier::Militia,
    );
    with_general.connect(town, fort, EdgeType::Supply, Owner::Player);
    with_general.add_general(
        "Steady Lord",
        Owner::Player,
        Race::Human,
        Some(fort),
        80,
        40,
        100,
    );

    let mut without = ScenarioHarness::quiet();
    let control = without.add_settlement(
        "Steady Town",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        50,
        5000,
    );
    without.world.node_mut(control).unwrap().loyalty = 50;
    let fort_b = without.add_fort(
        "Steady Fort",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        5,
        EquipmentTier::Militia,
    );
    without.connect(control, fort_b, EdgeType::Supply, Owner::Player);

    with_general.step(100);
    without.step(100);

    // 80 leadership restores 3 loyalty per 100 ticks.
    assert_eq!(with_general.world.node(town).unwrap().loyalty, 53);
    assert_eq!(
        without.world.node(control).unwrap().loyalty,
        50,
        "without a commander the town receives no loyalty aura"
    );
}

#[test]
fn test_general_defense_preserves_the_garrison() {
    fn battle(with_general: bool) -> (ScenarioHarness, u32) {
        let mut harness = ScenarioHarness::quiet();
        // Mirror-image garrisons, both Breplid humans, so the only asymmetry
        // in the fight is the commander on the player's wall.
        let player_fort = harness.add_fort(
            "Player Hold",
            Owner::Player,
            Some(Faction::Breplid),
            Some(Race::Human),
            100,
            EquipmentTier::Militia,
        );
        let enemy_fort = harness.add_fort(
            "Enemy Hold",
            Owner::Enemy,
            Some(Faction::Breplid),
            Some(Race::Human),
            100,
            EquipmentTier::Militia,
        );
        if with_general {
            harness.add_general(
                "Old Guard",
                Owner::Player,
                Race::Human,
                Some(player_fort),
                100,
                0,
                100,
            );
        }
        harness.connect(player_fort, enemy_fort, EdgeType::Combat, Owner::Player);
        (harness, player_fort)
    }

    let (mut held, held_fort) = battle(true);
    let (mut bare, bare_fort) = battle(false);
    held.step(3);
    bare.step(3);

    let held_pop = held.world.node(held_fort).unwrap().population;
    let bare_pop = bare.world.node(bare_fort).unwrap().population;
    assert!(
        held_pop > bare_pop,
        "a commander's fort defense must preserve troops ({} vs {})",
        held_pop,
        bare_pop
    );
}
