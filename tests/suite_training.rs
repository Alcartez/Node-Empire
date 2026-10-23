//! Test suite: manual worker training — gold cost, skill tiers, population
//! draw, on-the-job growth, and refusal handling.

mod common;

use common::ScenarioHarness;
use node_empire::game::types::*;
use node_empire::game::world::Node;

/// Add a bare production building to the sandbox.
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

/// A quiet sandbox with one player village and one player mine. The anchor
/// settlement is shrunk to a single soul so the mine's village is always the
/// settlement that supplies the worker.
fn training_scenario() -> (ScenarioHarness, u32, u32) {
    let mut harness = ScenarioHarness::quiet();
    if let Some(anchor) = harness.world.nodes.get_mut(&9998) {
        anchor.population = 1;
    }
    let village = harness.add_settlement(
        "Millbrook",
        Owner::Player,
        Some(Faction::Breplid),
        Race::Human,
        20,
        500,
    );
    let mine = producer(&mut harness, 100, NodeType::Mine, Owner::Player);
    (harness, village, mine)
}

#[test]
fn manual_training_charges_gold_and_draws_population() {
    let (mut harness, village, mine) = training_scenario();
    let gold_before = harness.world.player_ledger.gold;

    harness
        .world
        .train_worker(mine, 0, Owner::Player)
        .expect("basic training should be accepted");

    let node = harness.world.node(mine).unwrap();
    assert_eq!(node.worker_count(), 1, "the mine should gain one worker");
    let worker = &node.workers[0];
    assert_eq!(worker.job, JobType::Miner, "job follows the building type");
    assert_eq!(worker.skill, TRAIN_SKILLS[0], "novices start at tier skill");
    assert_eq!(
        worker.training_left,
        (TRAIN_SKILLS[0] as u32) * 2,
        "training lasts two ticks per starting skill point"
    );

    assert_eq!(
        harness.world.player_ledger.gold,
        gold_before - TRAIN_COSTS[0],
        "basic training costs 10 gold"
    );
    assert_eq!(
        harness.world.node(village).unwrap().population,
        19,
        "the recruit is drawn from village population"
    );

    harness.assert_invariants();
}

#[test]
fn training_completes_and_grants_one_skill() {
    let (mut harness, _village, mine) = training_scenario();
    harness.world.train_worker(mine, 1, Owner::Player).unwrap();

    let trainer = (TRAIN_SKILLS[1] as u32) * 2;
    harness.step(trainer as usize - 1);
    assert_eq!(
        harness.world.node(mine).unwrap().workers[0].skill,
        TRAIN_SKILLS[1],
        "skill is held until training finishes"
    );

    harness.step(1);
    let worker = &harness.world.node(mine).unwrap().workers[0];
    assert_eq!(worker.training_left, 0, "training should be complete");
    assert_eq!(
        worker.skill,
        TRAIN_SKILLS[1] + 1,
        "completion grants exactly one skill point"
    );

    harness.assert_invariants();
}

#[test]
fn workers_gain_a_skill_every_thirty_ticks() {
    let (mut harness, _village, mine) = training_scenario();
    harness.world.train_worker(mine, 0, Owner::Player).unwrap();

    let trainer = (TRAIN_SKILLS[0] as u32) * 2;
    harness.step(trainer as usize);
    let learned = harness.world.node(mine).unwrap().workers[0].skill;
    assert_eq!(learned, TRAIN_SKILLS[0] + 1);

    // Growth fires on ticks divisible by 30; wait for the next boundary.
    let wait = 30 - (harness.world.tick % 30);
    harness.step(wait as usize - 1);
    assert_eq!(
        harness.world.node(mine).unwrap().workers[0].skill,
        learned,
        "no growth just before the 30-tick boundary"
    );

    harness.step(1);
    assert_eq!(
        harness.world.node(mine).unwrap().workers[0].skill,
        learned + 1,
        "one skill point at the boundary"
    );

    harness.step(1);
    assert_eq!(
        harness.world.node(mine).unwrap().workers[0].skill,
        learned + 1,
        "growth is one point per boundary, not per tick"
    );

    harness.assert_invariants();
}

#[test]
fn refused_training_never_charges_gold() {
    let (mut harness, _village, mine) = training_scenario();
    let gold_before = harness.world.player_ledger.gold;

    let foreign = producer(&mut harness, 101, NodeType::Mine, Owner::Enemy);
    assert!(
        harness
            .world
            .train_worker(foreign, 0, Owner::Player)
            .is_err(),
        "a player cannot staff an enemy building"
    );
    assert_eq!(
        harness.world.player_ledger.gold, gold_before,
        "a refused foreign command must not cost gold"
    );
    assert_eq!(harness.world.node(foreign).unwrap().worker_count(), 0);

    let slots = harness.world.node(mine).unwrap().job_slots();
    for i in 0..slots {
        harness
            .world
            .nodes
            .get_mut(&mine)
            .unwrap()
            .workers
            .push(Worker::new(JobType::Miner, 10, 0, format!("Veteran {i}")));
    }
    assert!(
        harness.world.train_worker(mine, 0, Owner::Player).is_err(),
        "a fully staffed building refuses more workers"
    );
    assert_eq!(
        harness.world.player_ledger.gold, gold_before,
        "a refused full-building command must not cost gold"
    );

    harness.assert_invariants();
}
