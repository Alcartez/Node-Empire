//! Test suite: auto-employment — connected production buildings fill one post
//! per interval from the settlement's idle population.

mod common;

use common::ScenarioHarness;
use node_empire::game::types::*;
use node_empire::game::world::Node;

fn producer(harness: &mut ScenarioHarness, id: u32, kind: NodeType, owner: Owner) -> u32 {
    let node = Node::new(
        id,
        kind,
        owner,
        None,
        (id as f32) * 2.0,
        40.0,
        format!("Building {id}"),
    );
    harness.world.nodes.insert(id, node);
    id
}

#[test]
fn a_connected_building_fills_one_post_per_interval() {
    let mut harness = ScenarioHarness::quiet();
    let village = harness.add_settlement(
        "Millbrook",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        20,
        500,
    );
    let mine = producer(&mut harness, 100, NodeType::Mine, Owner::Player);
    harness.connect(village, mine, EdgeType::Resource, Owner::Player);

    // AUTO_EMPLOY_INTERVAL = 20 ticks.
    harness.step(19);
    assert_eq!(
        harness.world.node(mine).unwrap().worker_count(),
        0,
        "no hiring before the interval elapses"
    );
    assert_eq!(harness.world.node(village).unwrap().population, 20);

    harness.step(1);
    assert_eq!(
        harness.world.node(mine).unwrap().worker_count(),
        1,
        "one novice should walk to the mine"
    );
    assert_eq!(
        harness.world.node(village).unwrap().population,
        19,
        "the worker is drawn from the village population"
    );

    harness.step(20);
    assert_eq!(harness.world.node(mine).unwrap().worker_count(), 2);
    assert_eq!(harness.world.node(village).unwrap().population, 18);

    harness.assert_invariants();
}

#[test]
fn an_unconnected_building_stays_empty() {
    let mut harness = ScenarioHarness::quiet();
    let village = harness.add_settlement(
        "Millbrook",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        20,
        500,
    );
    let _mine = producer(&mut harness, 101, NodeType::Mine, Owner::Player);

    harness.step(60);

    assert_eq!(harness.world.node(101).unwrap().worker_count(), 0);
    assert_eq!(
        harness.world.node(village).unwrap().population,
        20,
        "an unconnected building must not drain the village"
    );

    harness.assert_invariants();
}

#[test]
fn the_settlement_keeps_its_reserve_population() {
    let mut harness = ScenarioHarness::quiet();
    let village = harness.add_settlement(
        "Millbrook",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        MIN_SETTLEMENT_POP_RESERVE,
        500,
    );
    let mine = producer(&mut harness, 102, NodeType::Mine, Owner::Player);
    harness.connect(village, mine, EdgeType::Resource, Owner::Player);

    harness.step(60);

    assert_eq!(harness.world.node(mine).unwrap().worker_count(), 0);
    assert_eq!(
        harness.world.node(village).unwrap().population,
        MIN_SETTLEMENT_POP_RESERVE
    );

    harness.assert_invariants();
}

#[test]
fn foreign_buildings_are_not_staffed() {
    let mut harness = ScenarioHarness::quiet();
    let village = harness.add_settlement(
        "Millbrook",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        20,
        500,
    );
    let enemy_mine = producer(&mut harness, 103, NodeType::Mine, Owner::Enemy);
    harness.connect(village, enemy_mine, EdgeType::Resource, Owner::Player);

    harness.step(60);

    assert_eq!(
        harness.world.node(enemy_mine).unwrap().worker_count(),
        0,
        "a player village must not staff enemy property"
    );

    harness.assert_invariants();
}
