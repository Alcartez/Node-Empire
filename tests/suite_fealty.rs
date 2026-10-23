//! Test suite: neutral fealty — trade-driven allegiance, the zero-sum tug of
//! war, the Trading Partner dividend, and peaceful integration.
//!
//! Fealty is awarded inside `World::caravans` when a road delivers stock to a
//! neutral settlement (`sim.rs::award_fealty`). These tests drive the real tick
//! pipeline with a hand-built two-village road.

mod common;

use common::ScenarioHarness;
use node_empire::game::types::*;
use node_empire::game::world::{Node, World};

/// A raw delivery to a neutral site is worth 1.2 fealty; manufactured goods
/// (factory → settlement) are worth 2.5. This is the base unit the tests use.
const FOOD_DELIVERY: f32 = 1.2;

/// Give a settlement enough food to have a surplus after the 15-tick reserve
/// every caravan trip leaves behind (`keep = pop / 10 * 15`).
fn top_up(world: &mut World, settlement: u32) {
    if let Some(node) = world.nodes.get_mut(&settlement) {
        node.stock.food = 200;
    }
}

fn farm_for(world: &mut World, id: u32, owner: Owner, x: f32, y: f32) {
    let node = Node::new(id, NodeType::Farm, owner, None, x, y, format!("Farm {id}"));
    world.nodes.insert(id, node);
}

#[test]
fn a_delivery_builds_fealty_but_does_not_integrate_immediately() {
    let mut harness = ScenarioHarness::quiet();
    let home = harness.add_settlement(
        "Edgeshire",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        30,
        200,
    );
    let neutral = harness.add_settlement("Greenvale", Owner::Neutral, None, Race::Human, 20, 200);
    harness.connect(home, neutral, EdgeType::Resource, Owner::Player);

    // Stop on the first tick that produces a delivery, whatever the caravan
    // period is for this road length.
    assert!(
        harness.step_until(60, |w| w.node(neutral).is_some_and(|n| n
            .fealty
            .get(&Owner::Player)
            .copied()
            .unwrap_or(0.0)
            > 0.0)),
        "a connected road should deliver within 60 ticks"
    );

    let site = harness.world.node(neutral).unwrap();
    let fealty = site.fealty.get(&Owner::Player).copied().unwrap_or(0.0);
    assert!(
        (fealty - FOOD_DELIVERY).abs() < 0.01,
        "one food delivery should award {FOOD_DELIVERY} fealty, got {fealty}"
    );
    assert_eq!(site.fealty_tier(Owner::Player), AllegianceTier::WaryNeutral);
    assert_eq!(
        site.owner,
        Owner::Neutral,
        "a single delivery must not annex the village"
    );
}

#[test]
fn fealty_crosses_trading_partner_at_25_and_pays_a_dividend() {
    let mut harness = ScenarioHarness::quiet();
    let home = harness.add_settlement(
        "Edgeshire",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        30,
        200,
    );
    let neutral = harness.add_settlement("Greenvale", Owner::Neutral, None, Race::Human, 20, 200);
    harness.connect(home, neutral, EdgeType::Resource, Owner::Player);

    // Sit just below the tier boundary so one trip crosses it.
    if let Some(site) = harness.world.nodes.get_mut(&neutral) {
        site.fealty.insert(Owner::Player, 24.0);
    }

    // Tick one at a time and measure the ledger on the crossing tick. Tax income
    // is at most one gold per tick, so a delta of 3+ can only come from the
    // dividend.
    let mut dividend_tick = None;
    for _ in 0..60 {
        let before = harness.world.player_ledger.gold;
        harness.step(1);
        let gained = harness.world.player_ledger.gold - before;
        let fealty = harness
            .world
            .node(neutral)
            .unwrap()
            .fealty
            .get(&Owner::Player)
            .copied()
            .unwrap_or(0.0);
        if fealty >= 25.0 {
            dividend_tick = Some(gained);
            break;
        }
    }

    let site = harness.world.node(neutral).unwrap();
    assert_eq!(
        site.fealty_tier(Owner::Player),
        AllegianceTier::TradingPartner
    );
    let fealty = site.fealty.get(&Owner::Player).copied().unwrap_or(0.0);
    assert!(
        fealty >= 25.0,
        "fealty should now be at least 25, got {fealty}"
    );

    // Every qualifying delivery pays a +3 commercial dividend.
    let gained = dividend_tick.expect("fealty never crossed the 25 threshold");
    assert!(
        gained >= 3,
        "crossing Trading Partner should pay the +3 dividend, tick gained {gained}"
    );
}

#[test]
fn a_second_faction_erodes_fealty_and_integration_flips_the_ring() {
    let mut harness = ScenarioHarness::quiet();
    let home = harness.add_settlement(
        "Edgeshire",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        30,
        200,
    );
    let rival = harness.add_settlement(
        "Ironhold",
        Owner::Enemy,
        Some(Faction::Mez),
        Race::Human,
        30,
        200,
    );
    let neutral = harness.add_settlement("Greenvale", Owner::Neutral, None, Race::Human, 20, 200);
    // The neutral's own starter farm: owned by the village, roaded by the
    // village, so its internal deliveries must not award anyone fealty.
    farm_for(&mut harness.world, 1001, Owner::Neutral, 20.0, 30.0);
    harness.connect(neutral, 1001, EdgeType::Resource, Owner::Neutral);
    harness.connect(home, neutral, EdgeType::Resource, Owner::Player);

    // Drive food deliveries until the village petitions to join. Top the
    // player larder up each trip; integration needs 85 fealty (71 food
    // deliveries, two per short road period).
    let mut trips = 0;
    while harness.world.node(neutral).unwrap().owner == Owner::Neutral {
        top_up(&mut harness.world, home);
        harness.step(10);
        trips += 1;
        assert!(trips < 200, "village did not integrate within 200 trips");
    }

    let site = harness.world.node(neutral).unwrap();
    assert_eq!(site.owner, Owner::Player);
    assert_eq!(site.faction, Some(Faction::Breplid));
    assert!(
        site.fealty.get(&Owner::Player).copied().unwrap_or(0.0) >= 85.0,
        "integration should require 85+ fealty"
    );

    // The neutral ring flips with the village.
    assert_eq!(
        harness.world.node(1001).unwrap().owner,
        Owner::Player,
        "the village's own farm should join the realm with it"
    );

    // Now that it is ours, a rival delivery cannot erode the (obsolete) score.
    let before = harness
        .world
        .node(neutral)
        .unwrap()
        .fealty
        .get(&Owner::Player)
        .copied()
        .unwrap_or(0.0);
    harness.connect(rival, neutral, EdgeType::Resource, Owner::Enemy);
    harness.step(20);
    let after = harness
        .world
        .node(neutral)
        .unwrap()
        .fealty
        .get(&Owner::Player)
        .copied()
        .unwrap_or(0.0);
    assert_eq!(
        before, after,
        "a road to a realm's own village must not build rival fealty"
    );
}

#[test]
fn rival_deliveries_erode_existing_fealty() {
    let mut harness = ScenarioHarness::quiet();
    let _home = harness.add_settlement(
        "Edgeshire",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        30,
        200,
    );
    let rival = harness.add_settlement(
        "Ironhold",
        Owner::Enemy,
        Some(Faction::Mez),
        Race::Human,
        30,
        200,
    );
    let neutral = harness.add_settlement("Greenvale", Owner::Neutral, None, Race::Human, 20, 200);
    if let Some(site) = harness.world.nodes.get_mut(&neutral) {
        site.fealty.insert(Owner::Player, 10.0);
    }

    harness.connect(rival, neutral, EdgeType::Resource, Owner::Enemy);
    harness.step(20);

    let site = harness.world.node(neutral).unwrap();
    let player_fealty = site.fealty.get(&Owner::Player).copied().unwrap_or(0.0);
    assert!(
        player_fealty < 10.0,
        "rival trade should erode our fealty, got {player_fealty}"
    );
    assert!(
        site.fealty.get(&Owner::Enemy).copied().unwrap_or(0.0) > 0.0,
        "the rival should be building its own fealty"
    );

    harness.assert_invariants();
}
