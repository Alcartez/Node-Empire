//! Test suite: node war — siege investment, supply, and capture without
//! depopulating the target.

mod common;

use common::ScenarioHarness;
use node_empire::game::types::*;
use node_empire::game::world::Node;

fn sutler(harness: &mut ScenarioHarness, id: u32) -> u32 {
    let market = Node::new(
        id,
        NodeType::Market,
        Owner::Player,
        None,
        40.0,
        20.0,
        "Sutler's Market".into(),
    );
    harness.world.nodes.insert(id, market);
    id
}

/// A player siege camp facing an enemy town. When `supplied`, the camp has a
/// friendly Supply road to a market, which keeps its siege effort at full
/// strength without moving population like a village road would.
fn siege_scenario(supplied: bool) -> (ScenarioHarness, u32, u32) {
    let mut harness = ScenarioHarness::quiet();
    let fort = harness.add_fort(
        "Siege Camp",
        Owner::Player,
        Some(Faction::Breplid),
        Some(Race::Human),
        60,
        EquipmentTier::Militia,
    );
    let town = harness.add_settlement("Target Town", Owner::Enemy, None, Race::Human, 120, 5000);
    harness.connect(fort, town, EdgeType::Siege, Owner::Player);

    if supplied {
        let market = sutler(&mut harness, 200);
        harness.connect(fort, market, EdgeType::Supply, Owner::Player);
    }

    (harness, fort, town)
}

#[test]
fn a_siege_captures_a_town_without_erasing_its_people() {
    let (mut harness, _fort, town) = siege_scenario(true);
    let before = harness.world.node(town).unwrap().population;

    let captured = harness.step_until(300, |w| {
        w.node(town)
            .map(|n| n.owner == Owner::Player)
            .unwrap_or(false)
    });

    assert!(captured, "the siege should eventually take the town");
    let taken = harness.world.node(town).unwrap();
    assert!(
        taken.population > 50,
        "capture should preserve most of the townsfolk (before {}, after {})",
        before,
        taken.population
    );
    assert_eq!(taken.siege_progress, 0.0, "investment resets after capture");
    assert!(!taken.besieged, "a captured town is no longer besieged");
    assert_eq!(taken.loyalty, 40);

    harness.assert_invariants();
}

#[test]
fn a_supplied_siege_makes_faster_progress() {
    let (mut supplied, _, supplied_town) = siege_scenario(true);
    let (mut unsupplied, _, unsupplied_town) = siege_scenario(false);

    supplied.step(20);
    unsupplied.step(20);

    let fast = supplied.world.node(supplied_town).unwrap().siege_progress;
    let slow = unsupplied
        .world
        .node(unsupplied_town)
        .unwrap()
        .siege_progress;
    assert!(fast > 0.0 && slow > 0.0, "both sieges should invest");
    assert!(
        fast > slow,
        "a supplied camp should outpace an unsupplied one ({} vs {})",
        fast,
        slow
    );
}

#[test]
fn a_broken_siege_loses_its_investment() {
    let (mut harness, _fort, town) = siege_scenario(true);
    harness.step(12);
    assert!(harness.world.node(town).unwrap().siege_progress > 0.0);

    harness.world.edges.retain(|e| e.kind != EdgeType::Siege);

    harness.step(60);
    let relaxed = harness.world.node(town).unwrap();
    assert_eq!(relaxed.siege_progress, 0.0);
    assert!(!relaxed.besieged);
    assert_eq!(relaxed.owner, Owner::Enemy);
}
