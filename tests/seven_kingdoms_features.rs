//! Integration tests for Seven Kingdoms mechanics:
//! * Starting Forts & Founding Generals
//! * Demographic Race Multipliers & Settlement Growth
//! * General Salaries, Loyalty, Bonuses & Mutiny
//! * Fort Equipment Tiers (Militia -> Bronze -> Iron-Clad -> Steel & Siege)

use node_empire::game::types::*;
use node_empire::game::world::World;

#[test]
fn test_general_loyalty_and_bonus() {
    let mut w = World::new(42, Faction::Breplid, Difficulty::Medium);
    let player_gen = w
        .generals
        .iter()
        .find(|g| g.owner == Owner::Player)
        .expect("Player should have a starting general")
        .clone();

    // Verify initial general properties
    assert!(player_gen.leadership >= 50);
    assert!(player_gen.combat >= 50);
    assert!(player_gen.loyalty >= 80);

    // Give player enough gold
    w.player_ledger.gold = 200;

    // Bestow bonus
    let initial_loyalty = player_gen.loyalty;
    let res = w.pay_general_bonus(player_gen.id, Owner::Player);
    assert!(res.is_ok());
    assert_eq!(w.player_ledger.gold, 150);

    let updated_gen = w
        .generals
        .iter()
        .find(|g| g.id == player_gen.id)
        .unwrap();
    assert_eq!(updated_gen.loyalty, (initial_loyalty + 15).min(100));

    // Try without enough gold
    w.player_ledger.gold = 20;
    let fail_res = w.pay_general_bonus(player_gen.id, Owner::Player);
    assert!(fail_res.is_err());
}

#[test]
fn test_general_mutiny_when_loyalty_drops() {
    let mut w = World::new(99, Faction::SilkSpinners, Difficulty::Medium);
    let player_gen = w
        .generals
        .iter()
        .find(|g| g.owner == Owner::Player)
        .expect("Player general exists")
        .clone();

    let fort_id = player_gen.fort_id.expect("Stationed at starting fort");
    assert_eq!(w.node(fort_id).unwrap().owner, Owner::Player);

    // Empty treasury and set general loyalty to 8 (< 10)
    w.player_ledger.gold = 0;
    if let Some(g) = w.generals.iter_mut().find(|g| g.id == player_gen.id) {
        g.loyalty = 8;
    }

    // Advance clock past the upkeep interval (100 ticks)
    for _ in 0..100 {
        w.tick();
    }

    // General mutinied! Fort and general are now Bandits
    let fort = w.node(fort_id).unwrap();
    assert_eq!(fort.owner, Owner::Bandit);
    assert_eq!(fort.faction, None);

    let mutineer = w.generals.iter().find(|g| g.id == player_gen.id).unwrap();
    assert_eq!(mutineer.owner, Owner::Bandit);
}

#[test]
fn test_fort_equipment_progression() {
    let mut w = World::new(101, Faction::Mez, Difficulty::Easy);
    let player_fort = w
        .nodes
        .values()
        .find(|n| n.kind == NodeType::Fort && n.owner == Owner::Player)
        .expect("Player starting fort exists")
        .id;

    assert_eq!(w.node(player_fort).unwrap().equipment, EquipmentTier::Militia);

    // Give player plenty of gold
    w.player_ledger.gold = 1000;

    // Militia -> Bronze
    let res1 = w.upgrade_fort_equipment(player_fort, Owner::Player);
    assert!(res1.is_ok());
    assert_eq!(w.node(player_fort).unwrap().equipment, EquipmentTier::Bronze);
    assert!(w.player_ledger.gold < 1000);

    // Bronze -> Iron-Clad
    let res2 = w.upgrade_fort_equipment(player_fort, Owner::Player);
    assert!(res2.is_ok());
    assert_eq!(w.node(player_fort).unwrap().equipment, EquipmentTier::IronClad);

    // Iron-Clad -> Steel & Siege
    let res3 = w.upgrade_fort_equipment(player_fort, Owner::Player);
    assert!(res3.is_ok());
    assert_eq!(w.node(player_fort).unwrap().equipment, EquipmentTier::SteelSiege);

    // Max tier reached
    let res4 = w.upgrade_fort_equipment(player_fort, Owner::Player);
    assert!(res4.is_err());
}

#[test]
fn test_race_trait_mechanics() {
    // Verify each race possesses its intended demographic and martial profiles
    assert!(Race::HiveSwarm.growth_multiplier() > Race::Human.growth_multiplier());
    assert!(Race::Beetlefolk.combat_multiplier() > Race::Human.combat_multiplier());
    assert!(Race::Beetlefolk.defense_multiplier() > Race::Human.defense_multiplier());
    assert!(Race::Reptilian.food_consumption_multiplier() < Race::Human.food_consumption_multiplier());
}
