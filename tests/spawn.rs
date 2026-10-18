//! New-game spawn guarantees.
//!
//! Every seed must hand the player a capital they can actually play: on solid
//! ground, inside the map, with a road to food, and a starter ring they own
//! and have already crewed. Before this, some seeds began with a capital
//! separated from its farm by water, which starved to zero population and
//! could never recover.

use node_empire::game::types::*;
use node_empire::game::world::World;

/// The starter ring: the capital's own producers, each pre-crewed and roaded.
#[test]
fn the_starter_ring_is_owned_and_crewed() {
    let mut w = World::new(4, Faction::SilkSpinners, Difficulty::Medium);
    w.autosave = false;

    let cap_id = w
        .settlements_of(Owner::Player)
        .first()
        .map(|n| n.id)
        .expect("a player capital");

    let ring: Vec<(u32, NodeType, i32)> = w
        .nodes
        .values()
        .filter(|n| n.owner == Owner::Player && n.is_production())
        .map(|n| (n.id, n.kind, n.worker_count()))
        .collect();

    assert!(
        ring.iter().any(|(_, kind, _)| *kind == NodeType::Farm),
        "a capital must always start with its own farm"
    );

    for (id, kind, crew) in &ring {
        assert_eq!(*crew, 1, "the starter {kind:?} should be pre-crewed");
        let node = w.node(*id).expect("ring node");
        assert_eq!(
            node.workers[0].training_left, 0,
            "starter workers must be ready, not in training"
        );
        assert!(
            w.edges_of(*id)
                .any(|e| e.other(*id) == cap_id && e.kind == EdgeType::Resource),
            "the starter {kind:?} must be roaded to the capital"
        );
    }

    let (filled, posts) = w.employment_of(Owner::Player);
    assert!(filled > 0, "the starter ring must already be working");
    assert!(posts >= filled, "posts {posts} < filled {filled}");
}

/// 200 seeds, checked end to end.
#[test]
fn every_seed_spawns_a_viable_capital() {
    const SEEDS: u64 = 200;

    for seed in 0..SEEDS {
        let mut w = World::new(seed, Faction::SilkSpinners, Difficulty::Medium);
        w.autosave = false;

        // Nothing is placed off the map or on water/mountains.
        for n in w.nodes.values() {
            let (x, y) = (n.x as i32, n.y as i32);
            assert!(
                w.map.in_bounds(x, y),
                "seed {seed}: {} at ({x},{y}) is off-map",
                n.name
            );
            assert!(
                w.map.terrain_at(x, y).passable(),
                "seed {seed}: {} sits on impassable terrain",
                n.name
            );
        }

        // The player's capital: on buildable ground, fully populated.
        let (cap_id, cx, cy, pop0) = match w.settlements_of(Owner::Player).first() {
            Some(n) => (n.id, n.x as i32, n.y as i32, n.population),
            None => panic!("seed {seed}: no player capital"),
        };
        assert!(
            w.map.terrain_at(cx, cy).buildable(),
            "seed {seed}: capital is not on buildable ground"
        );
        assert!(
            pop0 >= STARTING_POP,
            "seed {seed}: capital starts with only {pop0} people"
        );

        // A farm is roaded in, so the capital cannot begin starving.
        assert!(
            w.edges_of(cap_id).any(|e| {
                e.kind == EdgeType::Resource
                    && w.node(e.other(cap_id))
                        .map(|n| n.kind == NodeType::Farm)
                        .unwrap_or(false)
            }),
            "seed {seed}: the capital has no food road"
        );

        // A starter fort is roaded in and commanded by a general.
        let fort_id = match w.fort_supplying(cap_id) {
            Some(fid) => fid,
            None => panic!("seed {seed}: capital has no starter fort supply link"),
        };
        let fort = w.node(fort_id).expect("fort node");
        assert_eq!(fort.owner, Owner::Player);
        assert!(fort.population >= 5, "seed {seed}: fort has < 5 garrison");
        assert!(fort.general_id.is_some(), "seed {seed}: fort has no commanding general");

        // Five simulated minutes later it is still a going concern.
        for _ in 0..300 {
            w.tick();
        }

        for n in w.nodes.values() {
            assert!(
                n.population >= 0,
                "seed {seed}: {} has negative population",
                n.name
            );
            assert!(
                n.stock.food >= 0 && n.stock.ore >= 0 && n.stock.timber >= 0 && n.stock.goods >= 0,
                "seed {seed}: {} has negative stock",
                n.name
            );
        }
        for n in w.settlements_of(Owner::Player) {
            assert!(
                n.population > 0,
                "seed {seed}: {} was allowed to die out",
                n.name
            );
        }
        assert!(
            w.node(cap_id).map(|n| n.population).unwrap_or(0) > 0,
            "seed {seed}: the capital starved (started at {pop0})"
        );
    }
}

/// The starter fort: every capital starts with an owned fort, a supply road, and a founding general.
#[test]
fn the_starter_fort_and_general_are_spawned() {
    let mut w = World::new(4, Faction::SilkSpinners, Difficulty::Medium);
    w.autosave = false;

    let cap_id = w
        .settlements_of(Owner::Player)
        .first()
        .map(|n| n.id)
        .expect("a player capital");

    // Must have a Supply road to an owned Fort
    let fort_id = w
        .fort_supplying(cap_id)
        .expect("player capital must have a starter fort");
    let fort = w.node(fort_id).expect("fort exists");
    assert_eq!(fort.owner, Owner::Player);
    assert_eq!(fort.kind, NodeType::Fort);
    assert!(
        fort.population >= 5,
        "starter fort must be garrisoned with at least 5 troops"
    );

    // Must be commanded by a General
    let gen_id = fort
        .general_id
        .expect("starter fort must have a commanding general");
    let general = w.general(gen_id).expect("general exists in world");
    assert_eq!(general.owner, Owner::Player);
    assert_eq!(general.fort_id, Some(fort_id));
    assert!(general.leadership >= 50);
    assert!(general.combat >= 50);
    assert!(general.loyalty >= 70);

    // Enemy capital also starts with a fort and general
    let enemy_cap_id = w
        .settlements_of(Owner::Enemy)
        .first()
        .map(|n| n.id)
        .expect("an enemy capital");
    let enemy_fort_id = w
        .fort_supplying(enemy_cap_id)
        .expect("enemy capital must have a starter fort");
    let enemy_fort = w.node(enemy_fort_id).expect("enemy fort exists");
    assert_eq!(enemy_fort.owner, Owner::Enemy);
    assert!(enemy_fort.general_id.is_some());
}

