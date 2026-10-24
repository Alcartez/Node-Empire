//! Test suite: Fort Equipment Tiers, armory upgrades, and power progression.

mod common;
use common::ScenarioHarness;
use node_empire::game::types::*;

fn supplied_fort(harness: &mut ScenarioHarness) -> (u32, u32) {
    let town = harness.add_settlement(
        "Forge Town",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        50,
        500,
    );
    harness.world.node_mut(town).unwrap().stock.goods = 500;
    let fort = harness.add_fort(
        "Iron Bastion",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        10,
        EquipmentTier::Militia,
    );
    harness.connect(town, fort, EdgeType::Supply, Owner::Player);
    (town, fort)
}

#[test]
fn test_equipment_tier_upgrade_sequence_pays_gold_and_goods() {
    let mut harness = ScenarioHarness::new();
    let (town, f_id) = supplied_fort(&mut harness);
    harness.world.player_ledger.gold = 1000;

    let expected = [
        (EquipmentTier::Bronze, 50, 15),
        (EquipmentTier::IronClad, 125, 40),
        (EquipmentTier::SteelSiege, 250, 80),
    ];
    let mut gold = 1000;
    let mut goods = 500;

    for (tier, gold_cost, goods_cost) in expected {
        let before = harness.world.node(f_id).unwrap().equipment;
        assert!(harness
            .world
            .upgrade_fort_equipment(f_id, Owner::Player)
            .is_ok());
        gold -= gold_cost;
        goods -= goods_cost;

        assert_eq!(harness.world.node(f_id).unwrap().equipment, tier);
        assert_eq!(
            harness.world.player_ledger.gold, gold,
            "armory upgrade should cost {gold_cost} gold"
        );
        assert_eq!(
            harness.world.node(town).unwrap().stock.goods,
            goods,
            "armory upgrade should draw {goods_cost} goods from the supplying town"
        );
        assert_ne!(before, tier);
    }

    // Beyond max tier errors gracefully.
    assert!(harness
        .world
        .upgrade_fort_equipment(f_id, Owner::Player)
        .is_err());

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

    let res = harness
        .world
        .upgrade_fort_equipment(foe_f_id, Owner::Player);
    assert!(res.is_err());
    assert_eq!(
        harness.world.node(foe_f_id).unwrap().equipment,
        EquipmentTier::Militia
    );
}

#[test]
fn test_upgrade_needs_a_supply_road() {
    let mut harness = ScenarioHarness::new();
    let town = harness.add_settlement(
        "Unconnected Town",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        50,
        500,
    );
    harness.world.node_mut(town).unwrap().stock.goods = 500;
    let fort = harness.add_fort(
        "Lonely Fort",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        10,
        EquipmentTier::Militia,
    );
    harness.world.player_ledger.gold = 1000;

    let res = harness.world.upgrade_fort_equipment(fort, Owner::Player);
    assert!(res.is_err(), "no Supply road means no armory upgrade");
    assert_eq!(harness.world.player_ledger.gold, 1000);
    assert_eq!(harness.world.node(town).unwrap().stock.goods, 500);
    assert_eq!(
        harness.world.node(fort).unwrap().equipment,
        EquipmentTier::Militia
    );
}

#[test]
fn test_upgrade_refuses_without_goods_and_charges_nothing() {
    let mut harness = ScenarioHarness::new();
    let (town, fort) = supplied_fort(&mut harness);
    harness.world.node_mut(town).unwrap().stock.goods = 10;
    harness.world.player_ledger.gold = 1000;

    let res = harness.world.upgrade_fort_equipment(fort, Owner::Player);
    assert!(res.is_err(), "10 goods cannot pay a 15-good upgrade");
    assert_eq!(
        harness.world.player_ledger.gold, 1000,
        "a refused upgrade must not charge gold"
    );
    assert_eq!(
        harness.world.node(town).unwrap().stock.goods,
        10,
        "a refused upgrade must not consume goods"
    );
    assert_eq!(
        harness.world.node(fort).unwrap().equipment,
        EquipmentTier::Militia
    );
}
