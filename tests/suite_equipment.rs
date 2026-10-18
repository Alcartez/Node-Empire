//! Test suite: Fort Equipment Tiers, armory upgrades, and power progression.

mod common;
use common::ScenarioHarness;
use node_empire::game::types::*;

#[test]
fn test_equipment_tier_upgrade_sequence() {
    let mut harness = ScenarioHarness::new();
    let f_id = harness.add_fort(
        "Iron Bastion",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        10,
        EquipmentTier::Militia,
    );

    harness.world.player_ledger.gold = 1000;

    // Militia -> Bronze
    assert_eq!(harness.world.node(f_id).unwrap().equipment, EquipmentTier::Militia);
    assert!(harness.world.upgrade_fort_equipment(f_id, Owner::Player).is_ok());
    assert_eq!(harness.world.node(f_id).unwrap().equipment, EquipmentTier::Bronze);

    // Bronze -> Iron-Clad
    assert!(harness.world.upgrade_fort_equipment(f_id, Owner::Player).is_ok());
    assert_eq!(harness.world.node(f_id).unwrap().equipment, EquipmentTier::IronClad);

    // Iron-Clad -> Steel & Siege
    assert!(harness.world.upgrade_fort_equipment(f_id, Owner::Player).is_ok());
    assert_eq!(harness.world.node(f_id).unwrap().equipment, EquipmentTier::SteelSiege);

    // Beyond max tier errors gracefully
    assert!(harness.world.upgrade_fort_equipment(f_id, Owner::Player).is_err());

    harness.assert_invariants();
}

#[test]
fn test_equipment_combat_and_defense_buffs() {
    let militia = EquipmentTier::Militia;
    let bronze = EquipmentTier::Bronze;
    let iron = EquipmentTier::IronClad;
    let steel = EquipmentTier::SteelSiege;

    assert!(bronze.combat_mult() > militia.combat_mult());
    assert!(iron.combat_mult() > bronze.combat_mult());
    assert!(steel.combat_mult() > iron.combat_mult());

    assert!(bronze.defense_mult() > militia.defense_mult());
    assert!(iron.defense_mult() > bronze.defense_mult());
    assert!(steel.defense_mult() > iron.defense_mult());
}

#[test]
fn test_cannot_upgrade_foreign_fort() {
    let mut harness = ScenarioHarness::new();
    let foe_f_id = harness.add_fort(
        "Enemy Citadel",
        Owner::Enemy,
        Some(Faction::Mez),
        Some(Race::Reptilian),
        10,
        EquipmentTier::Militia,
    );

    harness.world.player_ledger.gold = 500;

    let res = harness.world.upgrade_fort_equipment(foe_f_id, Owner::Player);
    assert!(res.is_err());
    assert_eq!(harness.world.node(foe_f_id).unwrap().equipment, EquipmentTier::Militia);
}
